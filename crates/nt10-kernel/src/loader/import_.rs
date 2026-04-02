//! Import directory — count IMAGE_IMPORT_DESCRIPTOR entries (read-only bring-up).

use super::pe_image::parse_pe64_headers;

/// Returns the number of non-terminal import descriptors (RVA/size from data directory).
#[must_use]
pub fn count_import_descriptors(image: &[u8], import_rva: u32, import_size: u32) -> usize {
    if import_rva == 0 || import_size < 40 {
        return 0;
    }
    let start = import_rva as usize;
    let end = start.saturating_add(import_size as usize);
    if end > image.len() {
        return 0;
    }
    let mut count = 0usize;
    let mut off = start;
    while off + 20 <= end {
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
        off += 20;
    }
    count
}

/// Bring-up: returns `true` when the PE has **no** import DLLs to bind (empty import directory).
///
/// Full IAT fill + DLL load remains future work; this marks images that can run without imports.
#[must_use]
pub fn resolve_imports_for_image_stub(image: &mut [u8]) -> bool {
    let n = image.len();
    if n < 0x200 {
        return false;
    }
    let Ok(h) = parse_pe64_headers(&image[..n]) else {
        return false;
    };
    count_import_descriptors(&image[..n], h.import_table_rva, h.import_table_size) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
