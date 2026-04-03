//! NTFS attributes — read-only walk of resident attribute list in a file record slice.
//! Undocumented on-disk corners return partial results or stop the walk; do not infer layout from
//! Windows-only internals — extend only from public NTFS descriptions + self-tests on sample images.

/// End-of-attribute-list marker in little-endian records.
pub const NTFS_ATTR_TYPE_END: u32 = 0xFFFF_FFFF;

/// Collects attribute type codes until `NTFS_ATTR_TYPE_END` or corrupt length.
#[must_use]
pub fn ntfs_collect_attr_types(record: &[u8], attrs_offset: u16, out: &mut [u32]) -> usize {
    let mut off = attrs_offset as usize;
    let mut n = 0usize;
    while n < out.len() && off + 8 <= record.len() {
        let ty = u32::from_le_bytes([
            record[off],
            record[off + 1],
            record[off + 2],
            record[off + 3],
        ]);
        if ty == NTFS_ATTR_TYPE_END {
            break;
        }
        let len = u32::from_le_bytes([
            record[off + 4],
            record[off + 5],
            record[off + 6],
            record[off + 7],
        ]) as usize;
        if len < 8 || off.checked_add(len).map(|e| e > record.len()).unwrap_or(true) {
            break;
        }
        out[n] = ty;
        n += 1;
        off += len;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::ntfs::mft::MftRecordHeader;

    #[test]
    fn walk_synthetic_attrs() {
        let mut buf = [0u8; 128];
        buf[0..4].copy_from_slice(b"FILE");
        let attrs_off: u16 = 64;
        buf[20..22].copy_from_slice(&attrs_off.to_le_bytes());
        let a0 = attrs_off as usize;
        buf[a0..a0 + 4].copy_from_slice(&0x30u32.to_le_bytes());
        buf[a0 + 4..a0 + 8].copy_from_slice(&24u32.to_le_bytes());
        let a1 = a0 + 24;
        buf[a1..a1 + 4].copy_from_slice(&NTFS_ATTR_TYPE_END.to_le_bytes());
        let h = MftRecordHeader::from_prefix(&buf).unwrap();
        let mut types = [0u32; 4];
        let c = ntfs_collect_attr_types(&buf, h.attrs_offset, &mut types);
        assert_eq!(c, 1);
        assert_eq!(types[0], 0x30);
    }
}
