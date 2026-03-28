//! MFT — minimal read-only header slice for bring-up vertical tests.

/// First bytes of a file record (fixed part; attributes follow).
#[repr(C, packed)]
pub struct MftRecordHeader {
    pub magic: [u8; 4],
    pub usa_offset: u16,
    pub usa_count: u16,
    pub log_seq_number: u64,
    pub sequence: u16,
    pub link_count: u16,
    pub attrs_offset: u16,
    pub flags: u16,
    pub bytes_in_use: u32,
    pub bytes_allocated: u32,
}

impl MftRecordHeader {
    #[must_use]
    pub fn from_prefix(slice: &[u8]) -> Option<&Self> {
        if slice.len() < core::mem::size_of::<Self>() {
            return None;
        }
        unsafe { Some(&*(slice.as_ptr().cast::<Self>())) }
    }

    #[must_use]
    pub fn is_file_record_magic(&self) -> bool {
        &self.magic == b"FILE"
    }
}

/// Slices the first MFT record at a known byte offset (read-only bring-up).
#[must_use]
pub fn mft_record_slice<'a>(vol: &'a [u8], mft_byte_offset: usize, record_size: usize) -> Option<&'a [u8]> {
    vol.get(mft_byte_offset..mft_byte_offset + record_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_synthetic_file_record() {
        let mut buf = [0u8; 64];
        buf[0..4].copy_from_slice(b"FILE");
        let slice = mft_record_slice(&buf, 0, 64).unwrap();
        let h = MftRecordHeader::from_prefix(slice).unwrap();
        assert!(h.is_file_record_magic());
    }
}
