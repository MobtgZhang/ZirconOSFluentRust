//! PE32 (WOW64) — minimal optional-header validation (mirrors [`super::pe_image::parse_pe64_headers`] fields).

use super::pe::{
    IMAGE_DOS_SIGNATURE, IMAGE_FILE_MACHINE_I386, IMAGE_NT_OPTIONAL_HDR32_MAGIC, IMAGE_NT_SIGNATURE,
    IMAGE_DLLCHARACTERISTICS_NX_COMPAT, IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT,
    IMAGE_DIRECTORY_ENTRY_EXCEPTION, IMAGE_DIRECTORY_ENTRY_RESOURCE, IMAGE_DIRECTORY_ENTRY_TLS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pe32ValidateError {
    BufferTooSmall,
    BadDosSignature,
    BadPeSignature,
    NotI386,
    OptionalTooLarge,
    BadOptionalMagic,
    NotPe32,
    RelocsRequiredButMissing,
}

#[derive(Clone, Copy, Debug)]
pub struct Pe32Headers {
    pub entry_point_rva: u32,
    pub image_base: u64,
    pub size_of_image: u32,
    pub number_of_sections: u16,
    pub subsystem: u16,
    pub import_table_rva: u32,
    pub import_table_size: u32,
    pub base_reloc_rva: u32,
    pub base_reloc_size: u32,
    pub resource_rva: u32,
    pub resource_size: u32,
    pub exception_rva: u32,
    pub exception_size: u32,
    pub tls_rva: u32,
    pub tls_size: u32,
    pub delay_import_rva: u32,
    pub delay_import_size: u32,
    pub nx_compat_marked: bool,
}

#[inline]
fn u16_le(s: &[u8], o: usize) -> Option<u16> {
    let b = s.get(o..o + 2)?;
    Some(u16::from_le_bytes([b[0], b[1]]))
}

#[inline]
fn u32_le(s: &[u8], o: usize) -> Option<u32> {
    let b = s.get(o..o + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

#[inline]
fn data_directory_pe32(image: &[u8], opt_off: usize, index: usize) -> (u32, u32) {
    let base = opt_off + 0x60 + index * 8;
    let rva = u32_le(image, base).unwrap_or(0);
    let size = u32_le(image, base + 4).unwrap_or(0);
    (rva, size)
}

/// Parses COFF + PE32 optional header fields needed for WOW64 bring-up.
#[must_use]
pub fn parse_pe32_headers(image: &[u8]) -> Result<Pe32Headers, Pe32ValidateError> {
    if image.len() < 0x200 {
        return Err(Pe32ValidateError::BufferTooSmall);
    }
    if u16_le(image, 0).ok_or(Pe32ValidateError::BufferTooSmall)? != IMAGE_DOS_SIGNATURE {
        return Err(Pe32ValidateError::BadDosSignature);
    }
    let pe_off = u32_le(image, 0x3C).ok_or(Pe32ValidateError::BufferTooSmall)? as usize;
    if pe_off + 0x120 > image.len() {
        return Err(Pe32ValidateError::BufferTooSmall);
    }
    if u32_le(image, pe_off).ok_or(Pe32ValidateError::BufferTooSmall)? != IMAGE_NT_SIGNATURE {
        return Err(Pe32ValidateError::BadPeSignature);
    }
    if u16_le(image, pe_off + 4).ok_or(Pe32ValidateError::BufferTooSmall)? != IMAGE_FILE_MACHINE_I386 {
        return Err(Pe32ValidateError::NotI386);
    }
    let num_sections = u16_le(image, pe_off + 6).ok_or(Pe32ValidateError::BufferTooSmall)?;
    let size_opt = u16_le(image, pe_off + 20).ok_or(Pe32ValidateError::BufferTooSmall)? as usize;
    let opt_off = pe_off + 24;
    if opt_off.checked_add(size_opt).map(|e| e > image.len()).unwrap_or(true) {
        return Err(Pe32ValidateError::OptionalTooLarge);
    }
    let opt_magic = u16_le(image, opt_off).ok_or(Pe32ValidateError::BufferTooSmall)?;
    if opt_magic != IMAGE_NT_OPTIONAL_HDR32_MAGIC {
        return Err(Pe32ValidateError::BadOptionalMagic);
    }
    if size_opt < 0xE0 {
        return Err(Pe32ValidateError::NotPe32);
    }
    let entry = u32_le(image, opt_off + 0x10).ok_or(Pe32ValidateError::BufferTooSmall)?;
    let image_base = u32_le(image, opt_off + 0x1C).ok_or(Pe32ValidateError::BufferTooSmall)? as u64;
    let size_image = u32_le(image, opt_off + 0x38).ok_or(Pe32ValidateError::BufferTooSmall)?;
    let subsystem = u16_le(image, opt_off + 0x44).unwrap_or(0);
    // PE32: `IMAGE_DATA_DIRECTORY[0]` at optional +0x60 (import is index 1 → +0x68).
    let import_rva = u32_le(image, opt_off + 0x68).unwrap_or(0);
    let import_size = u32_le(image, opt_off + 0x6C).unwrap_or(0);
    let reloc_rva = u32_le(image, opt_off + 0x88).unwrap_or(0);
    let reloc_size = u32_le(image, opt_off + 0x8C).unwrap_or(0);
    let dll_char = u16_le(image, opt_off + 0x46).unwrap_or(0);
    let nx_compat_marked = (dll_char & IMAGE_DLLCHARACTERISTICS_NX_COMPAT) != 0;
    let (resource_rva, resource_size) = data_directory_pe32(image, opt_off, IMAGE_DIRECTORY_ENTRY_RESOURCE);
    let (exception_rva, exception_size) =
        data_directory_pe32(image, opt_off, IMAGE_DIRECTORY_ENTRY_EXCEPTION);
    let (tls_rva, tls_size) = data_directory_pe32(image, opt_off, IMAGE_DIRECTORY_ENTRY_TLS);
    let (delay_import_rva, delay_import_size) =
        data_directory_pe32(image, opt_off, IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT);
    Ok(Pe32Headers {
        entry_point_rva: entry,
        image_base,
        size_of_image: size_image,
        number_of_sections: num_sections,
        subsystem,
        import_table_rva: import_rva,
        import_table_size: import_size,
        base_reloc_rva: reloc_rva,
        base_reloc_size: reloc_size,
        resource_rva,
        resource_size,
        exception_rva,
        exception_size,
        tls_rva,
        tls_size,
        delay_import_rva,
        delay_import_size,
        nx_compat_marked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_buffer_rejected() {
        let b = [0u8; 4];
        assert!(matches!(
            parse_pe32_headers(&b),
            Err(Pe32ValidateError::BufferTooSmall)
        ));
    }
}
