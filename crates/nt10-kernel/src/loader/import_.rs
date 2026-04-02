//! Import directory — count IMAGE_IMPORT_DESCRIPTOR entries, minimal IAT bind (bring-up).

use super::pe::IMAGE_SCN_MEM_EXECUTE;
use super::pe_image::{coff_section_table, parse_pe64_headers, Pe64Headers};

#[inline]
fn u32_le(image: &[u8], o: usize) -> Option<u32> {
    let b = image.get(o..o + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[inline]
fn u64_le(image: &[u8], o: usize) -> Option<u64> {
    let b = image.get(o..o + 8)?;
    Some(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

/// First `ret` (`0xC3`) in any **executable** COFF section (file-backed bytes).
#[must_use]
pub fn find_ret_gadget_rva(image: &[u8]) -> Option<u32> {
    let h = parse_pe64_headers(image).ok()?;
    find_ret_gadget_rva_with_headers(image, &h)
}

fn find_ret_gadget_rva_with_headers(image: &[u8], _h: &Pe64Headers) -> Option<u32> {
    let (table_off, nsec) = coff_section_table(image).ok()?;
    for i in 0..nsec as usize {
        let base = table_off + i * 40;
        let virt_addr = u32_le(image, base + 12)?;
        let virt_size = u32_le(image, base + 8)?;
        let raw_size = u32_le(image, base + 16)?;
        let raw_ptr = u32_le(image, base + 20)?;
        let chars = u32_le(image, base + 36)?;
        if chars & IMAGE_SCN_MEM_EXECUTE == 0 {
            continue;
        }
        if raw_ptr == 0 {
            continue;
        }
        let scan = (raw_size.min(virt_size)) as usize;
        let ro = raw_ptr as usize;
        let slice = image.get(ro..ro.checked_add(scan)?)?;
        if let Some(rel) = slice.iter().position(|&b| b == 0xC3) {
            return virt_addr.checked_add(rel as u32);
        }
    }
    None
}

/// Fill PE32+ **IAT** slots with `image_base + ret_rva` for every thunk in each import descriptor
/// (bring-up: unknown DLLs become callable `ret` stubs).
pub fn bind_pe64_import_iat_ret_stubs(image: &mut [u8]) -> Result<(), ()> {
    let n = image.len();
    let h = parse_pe64_headers(&image[..n]).map_err(|_| ())?;
    let stub_rva = find_ret_gadget_rva_with_headers(&image[..n], &h).ok_or(())?;
    let va = h
        .image_base
        .checked_add(stub_rva as u64)
        .ok_or(())?;
    let table_end = h.import_table_rva as u64 + h.import_table_size as u64;
    let mut drva = h.import_table_rva;
    while (drva as u64) + 20 <= table_end {
        let off = import_rva_to_raw(&image[..n], drva).ok_or(())?;
        if off + 20 > n {
            return Err(());
        }
        let oft = u32_le(image, off).unwrap_or(0);
        let name_rva = u32_le(image, off + 12).unwrap_or(0);
        let ft = u32_le(image, off + 16).unwrap_or(0);
        if oft == 0 && name_rva == 0 {
            break;
        }
        let ilt_rva = if oft != 0 { oft } else { ft };
        if ilt_rva == 0 || ft == 0 {
            drva = drva.checked_add(20).ok_or(())?;
            continue;
        }
        let mut idx: u32 = 0;
        loop {
            let ilt_rva_i = ilt_rva.checked_add(idx.checked_mul(8).ok_or(())?).ok_or(())?;
            let ilt_off = import_rva_to_raw(&image[..n], ilt_rva_i).ok_or(())?;
            let ent = u64_le(image, ilt_off).ok_or(())?;
            if ent == 0 {
                break;
            }
            let iat_rva_i = ft.checked_add(idx.checked_mul(8).ok_or(())?).ok_or(())?;
            let iat_off = import_rva_to_raw(&image[..n], iat_rva_i).ok_or(())?;
            let slot = image.get_mut(iat_off..iat_off + 8).ok_or(())?;
            slot.copy_from_slice(&va.to_le_bytes());
            idx = idx.checked_add(1).ok_or(())?;
        }
        drva = drva.checked_add(20).ok_or(())?;
    }
    Ok(())
}

/// Map a PE RVA to an on-disk offset using the COFF section table.
#[must_use]
pub fn rva_to_raw_offset(image: &[u8], rva: u32) -> Option<usize> {
    let (table_off, nsec) = coff_section_table(image).ok()?;
    for i in 0..nsec as usize {
        let base = table_off + i * 40;
        let virt_size = u32_le(image, base + 8)?;
        let virt_addr = u32_le(image, base + 12)?;
        let raw_size = u32_le(image, base + 16)?;
        let raw_ptr = u32_le(image, base + 20)?;
        if rva < virt_addr {
            continue;
        }
        let end = virt_addr.saturating_add(virt_size.max(raw_size));
        if rva >= end {
            continue;
        }
        let delta = (rva - virt_addr) as usize;
        if raw_ptr == 0 {
            continue;
        }
        let ro = raw_ptr as usize;
        let rsz = raw_size as usize;
        if delta < rsz && ro.checked_add(delta)?.checked_add(1)? <= image.len() {
            return Some(ro + delta);
        }
    }
    None
}

/// File offset for an import descriptor or thunk **RVA**, with bring-up fallback when `rva` is
/// already a file offset (tiny unit tests without a full section table).
fn import_rva_to_raw(image: &[u8], rva: u32) -> Option<usize> {
    rva_to_raw_offset(image, rva).or_else(|| {
        let s = rva as usize;
        (s < image.len()).then_some(s)
    })
}

fn dll_name_bytes(image: &[u8], name_rva: u32) -> Option<&[u8]> {
    let off = rva_to_raw_offset(image, name_rva).unwrap_or_else(|| {
        if (name_rva as usize) < image.len() {
            name_rva as usize
        } else {
            usize::MAX
        }
    });
    if off == usize::MAX {
        return None;
    }
    let tail = image.get(off..)?;
    let len = tail.iter().position(|&b| b == 0)?;
    Some(&tail[..len])
}

fn ascii_eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| x.to_ascii_lowercase() == y.to_ascii_lowercase())
}

#[must_use]
pub fn is_allowlisted_import_dll(name: &[u8]) -> bool {
    const ALLOW: &[&[u8]] = &[b"NTDLL.DLL", b"KERNEL32.DLL", b"KERNELBASE.DLL"];
    ALLOW.iter().any(|p| ascii_eq_ignore_case(name, p))
}

/// `true` if every import descriptor names only [`is_allowlisted_import_dll`] DLLs (thunks may be unresolved).
#[must_use]
pub fn import_descriptors_allowlisted_only(image: &[u8], import_rva: u32, import_size: u32) -> bool {
    if import_rva == 0 || import_size < 40 {
        return true;
    }
    let table_end = import_rva as u64 + import_size as u64;
    let mut drva = import_rva;
    while (drva as u64) + 20 <= table_end {
        let Some(off) = import_rva_to_raw(image, drva) else {
            return false;
        };
        if off + 20 > image.len() {
            return false;
        }
        let orig_thunk = u32_le(image, off).unwrap_or(0);
        let name_rva = u32_le(image, off + 12).unwrap_or(0);
        if orig_thunk == 0 && name_rva == 0 {
            break;
        }
        let Some(nm) = dll_name_bytes(image, name_rva) else {
            return false;
        };
        if !is_allowlisted_import_dll(nm) {
            return false;
        }
        drva += 20;
    }
    true
}

/// Returns the number of non-terminal import descriptors (RVA/size from data directory).
#[must_use]
pub fn count_import_descriptors(image: &[u8], import_rva: u32, import_size: u32) -> usize {
    if import_rva == 0 || import_size < 40 {
        return 0;
    }
    let table_end = import_rva as u64 + import_size as u64;
    let mut count = 0usize;
    let mut drva = import_rva;
    while (drva as u64) + 20 <= table_end {
        let Some(off) = import_rva_to_raw(image, drva) else {
            return 0;
        };
        if off + 20 > image.len() {
            return 0;
        }
        let orig_thunk = u32::from_le_bytes([
            image[off],
            image[off + 1],
            image[off + 2],
            image[off + 3],
        ]);
        let name_rva = u32::from_le_bytes([
            image[off + 12],
            image[off + 13],
            image[off + 14],
            image[off + 15],
        ]);
        if orig_thunk == 0 && name_rva == 0 {
            break;
        }
        count += 1;
        drva += 20;
    }
    count
}

/// Bring-up: returns `true` when the PE needs **no** full IAT bind: empty import directory **or**
/// every listed DLL is in the KERNEL32 / KERNELBASE / NTDLL allow-list **or**
/// [`bind_pe64_import_iat_ret_stubs`] succeeds (non-allowlisted DLLs → single in-image `ret` target).
#[must_use]
pub fn resolve_imports_for_image_stub(image: &mut [u8]) -> bool {
    let n = image.len();
    if n < 0x200 {
        return false;
    }
    let Ok(h) = parse_pe64_headers(&image[..n]) else {
        return false;
    };
    let img = &image[..n];
    if count_import_descriptors(img, h.import_table_rva, h.import_table_size) == 0 {
        return true;
    }
    if import_descriptors_allowlisted_only(img, h.import_table_rva, h.import_table_size) {
        return true;
    }
    bind_pe64_import_iat_ret_stubs(image).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::pe::{
        IMAGE_DOS_SIGNATURE, IMAGE_FILE_MACHINE_AMD64, IMAGE_NT_OPTIONAL_HDR64_MAGIC, IMAGE_NT_SIGNATURE,
        IMAGE_SUBSYSTEM_WINDOWS_CUI,
    };

    /// Minimal PE32+ with `.text` (`ret`) + `.idata` (one non-allowlisted DLL, one thunk).
    fn synthetic_pe_unknown_import() -> [u8; 0x900] {
        const PE: usize = 0x100;
        const OPT: usize = PE + 24;
        const SEC: usize = OPT + 0xF0;
        const RAW_TEXT: usize = 0x400;
        const RAW_IDATA: usize = 0x600;
        let mut img = [0u8; 0x900];
        img[0..2].copy_from_slice(&IMAGE_DOS_SIGNATURE.to_le_bytes());
        img[0x3C..0x40].copy_from_slice(&(PE as u32).to_le_bytes());
        img[PE..PE + 4].copy_from_slice(&IMAGE_NT_SIGNATURE.to_le_bytes());
        img[PE + 4..PE + 6].copy_from_slice(&IMAGE_FILE_MACHINE_AMD64.to_le_bytes());
        img[PE + 6..PE + 8].copy_from_slice(&2u16.to_le_bytes());
        img[PE + 20..PE + 22].copy_from_slice(&0xF0u16.to_le_bytes());
        img[OPT..OPT + 2].copy_from_slice(&IMAGE_NT_OPTIONAL_HDR64_MAGIC.to_le_bytes());
        img[OPT + 0x10..OPT + 0x14].copy_from_slice(&0x1000u32.to_le_bytes());
        img[OPT + 0x18..OPT + 0x20].copy_from_slice(&0x1400_0000u64.to_le_bytes());
        img[OPT + 0x38..OPT + 0x3C].copy_from_slice(&0x3000u32.to_le_bytes());
        img[OPT + 0x44..OPT + 0x46].copy_from_slice(&IMAGE_SUBSYSTEM_WINDOWS_CUI.to_le_bytes());
        let idd = OPT + 0x70 + 8;
        img[idd..idd + 4].copy_from_slice(&0x2000u32.to_le_bytes());
        img[idd + 4..idd + 8].copy_from_slice(&0x60u32.to_le_bytes());
        let s0 = SEC;
        img[s0..s0 + 8].copy_from_slice(b".text\0\0\0");
        img[s0 + 8..s0 + 12].copy_from_slice(&0x200u32.to_le_bytes());
        img[s0 + 12..s0 + 16].copy_from_slice(&0x1000u32.to_le_bytes());
        img[s0 + 16..s0 + 20].copy_from_slice(&0x200u32.to_le_bytes());
        img[s0 + 20..s0 + 24].copy_from_slice(&(RAW_TEXT as u32).to_le_bytes());
        img[s0 + 36..s0 + 40].copy_from_slice(&0x6000_0020u32.to_le_bytes());
        let s1 = SEC + 40;
        img[s1..s1 + 8].copy_from_slice(b".idata\0\0");
        img[s1 + 8..s1 + 12].copy_from_slice(&0x100u32.to_le_bytes());
        img[s1 + 12..s1 + 16].copy_from_slice(&0x2000u32.to_le_bytes());
        img[s1 + 16..s1 + 20].copy_from_slice(&0x100u32.to_le_bytes());
        img[s1 + 20..s1 + 24].copy_from_slice(&(RAW_IDATA as u32).to_le_bytes());
        img[s1 + 36..s1 + 40].copy_from_slice(&0xC000_0040u32.to_le_bytes());
        img[RAW_TEXT] = 0x90;
        img[RAW_TEXT + 1] = 0xC3;
        let id = RAW_IDATA;
        // IMAGE_IMPORT_DESCRIPTOR 20 bytes, then ILT (8 + 8 zero), then IAT — must not overlap.
        img[id..id + 4].copy_from_slice(&0x2018u32.to_le_bytes());
        img[id + 12..id + 16].copy_from_slice(&0x2040u32.to_le_bytes());
        img[id + 16..id + 20].copy_from_slice(&0x2030u32.to_le_bytes());
        img[id + 20..id + 40].fill(0);
        img[id + 0x40..id + 0x48].copy_from_slice(b"FOO.DLL\0");
        let ilt_off = RAW_IDATA + 0x18;
        img[ilt_off..ilt_off + 8].copy_from_slice(&0x2050u64.to_le_bytes());
        img[ilt_off + 8..ilt_off + 16].fill(0);
        let iat_off = RAW_IDATA + 0x30;
        img[iat_off..iat_off + 8].fill(0);
        let name_off = RAW_IDATA + 0x50;
        img[name_off..name_off + 2].copy_from_slice(&0u16.to_le_bytes());
        img[name_off + 2..name_off + 10].copy_from_slice(b"DummyFn\0");
        img
    }

    #[test]
    fn count_two_dlls() {
        let mut img = [0u8; 128];
        let base = 40usize;
        img[base..base + 4].copy_from_slice(&1u32.to_le_bytes());
        img[base + 12..base + 16].copy_from_slice(&80u32.to_le_bytes());
        let base2 = base + 20;
        img[base2..base2 + 4].copy_from_slice(&1u32.to_le_bytes());
        img[base2 + 12..base2 + 16].copy_from_slice(&90u32.to_le_bytes());
        img[base + 40..base + 60].fill(0);
        assert_eq!(count_import_descriptors(&img, base as u32, 60), 2);
    }

    #[test]
    fn stub_false_on_invalid_pe() {
        let mut b = [0u8; 512];
        assert!(!resolve_imports_for_image_stub(&mut b));
    }

    #[test]
    fn bind_ret_stub_fills_iat_for_unknown_dll() {
        let mut img = synthetic_pe_unknown_import();
        assert!(!import_descriptors_allowlisted_only(&img, 0x2000, 0x60));
        bind_pe64_import_iat_ret_stubs(&mut img).expect("bind");
        let iat_file = rva_to_raw_offset(&img, 0x2030).expect("iat");
        let got = u64_le(&img, iat_file).expect("slot");
        assert_eq!(got, 0x1400_0000u64 + 0x1001);
    }

    #[test]
    fn resolve_stub_true_after_ret_bind() {
        let mut img = synthetic_pe_unknown_import();
        assert!(resolve_imports_for_image_stub(&mut img));
    }

    #[test]
    fn allowlisted_kernel32_ntdll_descriptors_pass_stub() {
        let mut img = [0u8; 256];
        let base = 40usize;
        img[base..base + 4].copy_from_slice(&1u32.to_le_bytes());
        img[base + 12..base + 16].copy_from_slice(&128u32.to_le_bytes());
        img[128..128 + 13].copy_from_slice(b"KERNEL32.DLL\0");
        let base2 = base + 20;
        img[base2..base2 + 4].copy_from_slice(&1u32.to_le_bytes());
        img[base2 + 12..base2 + 16].copy_from_slice(&200u32.to_le_bytes());
        img[200..200 + 10].copy_from_slice(b"NTDLL.DLL\0");
        img[base + 40..base + 60].fill(0);
        assert!(import_descriptors_allowlisted_only(&img, base as u32, 60));
    }
}
