//! Base relocations — AMD64 DIR64 (`type == 10`) only for bring-up.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocError {
    OutOfRange,
    BadBlock,
    UnsupportedType,
}

const IMAGE_REL_BASED_DIR64: u16 = 10;
const IMAGE_REL_BASED_ABSOLUTE: u16 = 0;

/// Applies `.reloc`-style blocks in `image` using VA delta `delta` (new_base - old_base).
///
/// `reloc_rva` is the file offset (or mapped RVA) of the first block; **`0` means “no relocation directory”**
/// and is ignored (matches PE data directory convention).
pub fn apply_pe64_relocs(
    image: &mut [u8],
    reloc_rva: u32,
    reloc_size: u32,
    delta: i64,
) -> Result<(), RelocError> {
    if reloc_rva == 0 || reloc_size == 0 {
        return Ok(());
    }
    if delta == 0 {
        return Ok(());
    }
    let start = reloc_rva as usize;
    let sz = reloc_size as usize;
    let end = start.checked_add(sz).ok_or(RelocError::OutOfRange)?;
    if end > image.len() {
        return Err(RelocError::OutOfRange);
    }
    let mut off = start;
    while off + 8 <= end {
        let page_rva = u32::from_le_bytes([
            image[off],
            image[off + 1],
            image[off + 2],
            image[off + 3],
        ]);
        let block_size = u32::from_le_bytes([
            image[off + 4],
            image[off + 5],
            image[off + 6],
            image[off + 7],
        ]) as usize;
        if block_size < 8 || off.checked_add(block_size).map(|e| e > end).unwrap_or(true) {
            return Err(RelocError::BadBlock);
        }
        let mut p = off + 8;
        while p + 2 <= off + block_size {
            let w = u16::from_le_bytes([image[p], image[p + 1]]);
            p += 2;
            let typ = w >> 12;
            let o = u32::from(w & 0x0FFF);
            if typ == IMAGE_REL_BASED_ABSOLUTE {
                continue;
            }
            if typ != IMAGE_REL_BASED_DIR64 {
                return Err(RelocError::UnsupportedType);
            }
            let addr = page_rva.checked_add(o).ok_or(RelocError::OutOfRange)? as usize;
            if addr + 8 > image.len() {
                return Err(RelocError::OutOfRange);
            }
            let cur = u64::from_le_bytes([
                image[addr],
                image[addr + 1],
                image[addr + 2],
                image[addr + 3],
                image[addr + 4],
                image[addr + 5],
                image[addr + 6],
                image[addr + 7],
            ]);
            let newv = (cur as i64).wrapping_add(delta) as u64;
            image[addr..addr + 8].copy_from_slice(&newv.to_le_bytes());
        }
        off += block_size;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir64_delta_applies() {
        let mut img = [0u8; 128];
        let reloc_base = 0x20usize;
        let page = 0u32;
        img[reloc_base..reloc_base + 4].copy_from_slice(&page.to_le_bytes());
        img[reloc_base + 4..reloc_base + 8].copy_from_slice(&16u32.to_le_bytes());
        let fix_off = 0x40usize;
        let w: u16 = (IMAGE_REL_BASED_DIR64 << 12) | ((fix_off as u16) & 0x0FFF);
        img[reloc_base + 8..reloc_base + 10].copy_from_slice(&w.to_le_bytes());
        img[fix_off..fix_off + 8].copy_from_slice(&0x5000u64.to_le_bytes());
        apply_pe64_relocs(&mut img, reloc_base as u32, 16, 0x1000).unwrap();
        let v = u64::from_le_bytes([
            img[fix_off],
            img[fix_off + 1],
            img[fix_off + 2],
            img[fix_off + 3],
            img[fix_off + 4],
            img[fix_off + 5],
            img[fix_off + 6],
            img[fix_off + 7],
        ]);
        assert_eq!(v, 0x6000);
    }
}
