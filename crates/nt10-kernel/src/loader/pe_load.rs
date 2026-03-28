//! Bring-up: VFS → PE headers → optional ASLR slide + base reloc application.

use super::aslr;
use super::import_::count_import_descriptors;
use super::pe_image::{parse_pe64_headers, Pe64Headers, PeValidateError};
use super::reloc::{apply_pe64_relocs, RelocError};
use super::vfs_image::read_mount_into_buffer;
use crate::fs::vfs::VfsTable;
use crate::subsystems::win32::exec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeLoadError {
    Vfs,
    Pe(PeValidateError),
    ImageLargerThanBuffer,
    Reloc(RelocError),
    RelocsRequiredButMissing,
}

/// Result of loading into an in-memory buffer (same layout as on-disk for bring-up).
#[derive(Clone, Copy, Debug)]
pub struct PeLoadBringup {
    pub headers: Pe64Headers,
    pub bytes_in_buffer: usize,
    pub load_base: u64,
    pub import_dll_count: usize,
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
    Ok(PeLoadBringup {
        headers,
        bytes_in_buffer: n,
        load_base,
        import_dll_count: imports,
    })
}
