//! Bring-up: VFS → PE headers → optional ASLR slide + base reloc application.

use super::aslr;
use super::import_::count_import_descriptors;
#[cfg(target_arch = "x86_64")]
use super::pe_image::coff_section_table;
use super::pe_image::{parse_pe64_headers, Pe64Headers, PeValidateError};
use super::reloc::{apply_pe64_relocs, RelocError};
use super::vfs_image::read_mount_into_buffer;
use crate::fs::vfs::VfsTable;
use crate::subsystems::win32::exec;

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::read_cr3;
#[cfg(target_arch = "x86_64")]
use crate::loader::pe::IMAGE_SCN_MEM_WRITE;
#[cfg(target_arch = "x86_64")]
use crate::mm::nx_image::nx_pte_for_section_characteristics;
#[cfg(target_arch = "x86_64")]
use crate::mm::pt::{self, PageFlags};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeLoadError {
    Vfs,
    Pe(PeValidateError),
    ImageLargerThanBuffer,
    Reloc(RelocError),
    RelocsRequiredButMissing,
    /// x86_64: failed to install per-page mappings for a PE section.
    SectionMap,
}

/// Result of loading into an in-memory buffer (same layout as on-disk for bring-up).
#[derive(Clone, Copy, Debug)]
pub struct PeLoadBringup {
    pub headers: Pe64Headers,
    pub bytes_in_buffer: usize,
    pub load_base: u64,
    pub     import_dll_count: usize,
}

#[cfg(target_arch = "x86_64")]
#[inline]
fn u32_le_at(image: &[u8], o: usize) -> Option<u32> {
    let b = image.get(o..o + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[cfg(target_arch = "x86_64")]
fn align_up_u64(v: u64, a: u64) -> u64 {
    let m = a - 1;
    v.saturating_add(m) & !m
}

/// Map each file-backed 4 KiB page of the PE at `load_base + RVA` with NX/write derived from section flags.
///
/// Requires an **identity-mapped** `image` buffer (same physical as kernel virtual); no-op if the PFN pool
/// is not initialized.
///
/// # Safety
/// `image` / `headers` / `load_base` must match the loaded module; run only during bring-up under the active CR3.
#[cfg(target_arch = "x86_64")]
pub unsafe fn map_pe_image_sections_bringup(
    image: &[u8],
    headers: &Pe64Headers,
    load_base: u64,
) -> Result<(), PeLoadError> {
    if !crate::mm::phys::pfn_pool_initialized() {
        return Ok(());
    }
    let (table_off, nsec) = coff_section_table(image).map_err(PeLoadError::Pe)?;
    let cr3 = read_cr3();
    let img_top = load_base.saturating_add(headers.size_of_image as u64);
    for i in 0..nsec as usize {
        let o = table_off + i * 40;
        let vsize = u32_le_at(image, o + 8).ok_or(PeLoadError::Pe(PeValidateError::BufferTooSmall))?
            as u64;
        let va_sec =
            u32_le_at(image, o + 12).ok_or(PeLoadError::Pe(PeValidateError::BufferTooSmall))? as u64;
        let raw_size =
            u32_le_at(image, o + 16).ok_or(PeLoadError::Pe(PeValidateError::BufferTooSmall))? as u64;
        let ptr_raw =
            u32_le_at(image, o + 20).ok_or(PeLoadError::Pe(PeValidateError::BufferTooSmall))? as u64;
        let chars =
            u32_le_at(image, o + 36).ok_or(PeLoadError::Pe(PeValidateError::BufferTooSmall))?;
        if vsize == 0 && raw_size == 0 {
            continue;
        }
        let span = vsize.max(raw_size);
        let region_end = align_up_u64(va_sec.saturating_add(span), 4096);
        let first_page = va_sec & !0xfff;
        let nx = nx_pte_for_section_characteristics(chars);
        let write = (chars & IMAGE_SCN_MEM_WRITE) != 0;
        let flags = PageFlags {
            present: true,
            write,
            user: false,
            nx,
            write_combining: false,
        };
        let mut page_rva = first_page;
        while page_rva < region_end {
            let gvaddr = load_base.wrapping_add(page_rva);
            if gvaddr >= img_top {
                break;
            }
            let delta = page_rva.saturating_sub(va_sec);
            if delta < raw_size {
                let fo = (ptr_raw as u64).saturating_add(delta) as usize;
                if fo.saturating_add(4096) > image.len() {
                    page_rva = page_rva.saturating_add(4096);
                    continue;
                }
                let pa = image.as_ptr() as u64 + fo as u64;
                unsafe {
                    if pt::map_4k(cr3, gvaddr, pa, flags).is_err() {
                        return Err(PeLoadError::SectionMap);
                    }
                    crate::arch::x86_64::tlb::invlpg(gvaddr);
                }
            }
            page_rva = page_rva.saturating_add(4096);
        }
    }
    Ok(())
}

/// Loads from `vfs` slot into `buf`, picks a page-aligned pseudo-random base with [`aslr::image_slide`],
/// and applies base relocations when present.
pub fn load_pe_from_vfs_bringup(
    vfs: &VfsTable,
    slot: usize,
    buf: &mut [u8],
    aslr_seed: u64,
) -> Result<PeLoadBringup, PeLoadError> {
    let n = read_mount_into_buffer(vfs, slot, buf).map_err(|_| PeLoadError::Vfs)?;
    let headers = parse_pe64_headers(&buf[..n]).map_err(PeLoadError::Pe)?;
    if headers.size_of_image as usize > buf.len() {
        return Err(PeLoadError::ImageLargerThanBuffer);
    }
    let slide = aslr::image_slide(aslr_seed, 1, 16);
    let load_base = headers.image_base.wrapping_add(slide);
    let delta = (load_base as i64).wrapping_sub(headers.image_base as i64);
    if delta != 0 && (headers.base_reloc_rva == 0 || headers.base_reloc_size == 0) {
        return Err(PeLoadError::RelocsRequiredButMissing);
    }
    apply_pe64_relocs(
        &mut buf[..n],
        headers.base_reloc_rva,
        headers.base_reloc_size,
        delta,
    )
    .map_err(PeLoadError::Reloc)?;
    let imports = count_import_descriptors(
        &buf[..n],
        headers.import_table_rva,
        headers.import_table_size,
    );
    super::tls_bringup::record_tls_directory_present(&headers);
    exec::on_main_module_mapping_complete(headers.entry_point_rva, load_base, imports);
    crate::mm::nx_image::record_pe_nx_hint(headers.nx_compat_marked);
    #[cfg(target_arch = "x86_64")]
    unsafe {
        map_pe_image_sections_bringup(&buf[..n], &headers, load_base)?;
    }
    Ok(PeLoadBringup {
        headers,
        bytes_in_buffer: n,
        load_base,
        import_dll_count: imports,
    })
}
