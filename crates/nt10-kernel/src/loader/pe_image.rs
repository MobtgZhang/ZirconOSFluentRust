//! PE32+ (AMD64) header validation on a byte slice — layout follows the public PE/COFF specification.
//!
//! # Win32 bring-up checklist (ZirconOS)
//!
//! | Field / directory | Role | Loader status |
//! |-------------------|------|-----------------|
//! | `Subsystem` | GUI vs CUI entry expectations | Parsed; GUI (`2`) vs CUI (`3`) distinguished for logging / future CRT |
//! | Base reloc | ASLR slide | Applied in [`super::pe_load`] |
//! | Import directory | DLL thunk slots | Counted; full bind deferred |
//! | TLS directory | Thread-local + callbacks | Parsed; **not** applied — see [`super::tls_bringup`] |
//! | Delay import | Late-bound DLLs | Parsed; **not** applied |
//! | Exception directory | x64 unwind info | Parsed; registration deferred |
//! | Resource (`.rsrc`) | Menus, dialogs, icons | Parsed; decode in user32/shell path |

use super::pe::{
    IMAGE_DOS_SIGNATURE, IMAGE_FILE_MACHINE_AMD64, IMAGE_NT_OPTIONAL_HDR64_MAGIC, IMAGE_NT_SIGNATURE,
    IMAGE_SUBSYSTEM_WINDOWS_CUI, IMAGE_SUBSYSTEM_WINDOWS_GUI,
    IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT, IMAGE_DIRECTORY_ENTRY_EXCEPTION, IMAGE_DIRECTORY_ENTRY_RESOURCE,
    IMAGE_DIRECTORY_ENTRY_TLS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeValidateError {
    BufferTooSmall,
    BadDosSignature,
    BadPeSignature,
    NotAmd64,
    OptionalTooLarge,
    BadOptionalMagic,
    NotPe32Plus,
}

#[derive(Clone, Copy, Debug)]
pub struct Pe64Headers {
    pub entry_point_rva: u32,
    pub image_base: u64,
    pub size_of_image: u32,
    pub number_of_sections: u16,
    /// PE optional header `Subsystem` (`IMAGE_SUBSYSTEM_WINDOWS_*`).
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
    /// `IMAGE_DLLCHARACTERISTICS_NX_COMPAT` from the PE32+ optional header (public COFF field).
    pub nx_compat_marked: bool,
}

impl Pe64Headers {
    #[must_use]
    pub const fn is_windows_gui_subsystem(self) -> bool {
        self.subsystem == IMAGE_SUBSYSTEM_WINDOWS_GUI
    }

    #[must_use]
    pub const fn is_windows_cui_subsystem(self) -> bool {
        self.subsystem == IMAGE_SUBSYSTEM_WINDOWS_CUI
    }
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
fn u64_le(s: &[u8], o: usize) -> Option<u64> {
    let b = s.get(o..o + 8)?;
    Some(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

#[inline]
fn data_directory(image: &[u8], opt_off: usize, index: usize) -> (u32, u32) {
    let base = opt_off + 0x70 + index * 8;
    let rva = u32_le(image, base).unwrap_or(0);
    let size = u32_le(image, base + 4).unwrap_or(0);
    (rva, size)
}

/// Parses COFF + PE32+ optional header fields needed for bring-up load.
#[must_use]
pub fn parse_pe64_headers(image: &[u8]) -> Result<Pe64Headers, PeValidateError> {
    if image.len() < 0x200 {
        return Err(PeValidateError::BufferTooSmall);
    }
    if u16_le(image, 0).ok_or(PeValidateError::BufferTooSmall)? != IMAGE_DOS_SIGNATURE {
        return Err(PeValidateError::BadDosSignature);
    }
    let pe_off = u32_le(image, 0x3C).ok_or(PeValidateError::BufferTooSmall)? as usize;
    if pe_off + 0x120 > image.len() {
        return Err(PeValidateError::BufferTooSmall);
    }
    if u32_le(image, pe_off).ok_or(PeValidateError::BufferTooSmall)? != IMAGE_NT_SIGNATURE {
        return Err(PeValidateError::BadPeSignature);
    }
    if u16_le(image, pe_off + 4).ok_or(PeValidateError::BufferTooSmall)? != IMAGE_FILE_MACHINE_AMD64 {
        return Err(PeValidateError::NotAmd64);
    }
    let num_sections = u16_le(image, pe_off + 6).ok_or(PeValidateError::BufferTooSmall)?;
    let size_opt = u16_le(image, pe_off + 20).ok_or(PeValidateError::BufferTooSmall)? as usize;
    let opt_off = pe_off + 24;
    if opt_off.checked_add(size_opt).map(|e| e > image.len()).unwrap_or(true) {
        return Err(PeValidateError::OptionalTooLarge);
    }
    let opt_magic = u16_le(image, opt_off).ok_or(PeValidateError::BufferTooSmall)?;
    if opt_magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC {
        return Err(PeValidateError::BadOptionalMagic);
    }
    if size_opt < 0xF0 {
        return Err(PeValidateError::NotPe32Plus);
    }
    let entry = u32_le(image, opt_off + 0x10).ok_or(PeValidateError::BufferTooSmall)?;
    let image_base = u64_le(image, opt_off + 0x18).ok_or(PeValidateError::BufferTooSmall)?;
    let size_image = u32_le(image, opt_off + 0x38).ok_or(PeValidateError::BufferTooSmall)?;
    let subsystem = u16_le(image, opt_off + 0x44).unwrap_or(0);
    let import_rva = u32_le(image, opt_off + 0x78).unwrap_or(0);
    let import_size = u32_le(image, opt_off + 0x7C).unwrap_or(0);
    let reloc_rva = u32_le(image, opt_off + 0x98).unwrap_or(0);
    let reloc_size = u32_le(image, opt_off + 0x9C).unwrap_or(0);
    let dll_char = u16_le(image, opt_off + 0x46).unwrap_or(0);
    let nx_compat_marked = (dll_char & super::pe::IMAGE_DLLCHARACTERISTICS_NX_COMPAT) != 0;

    let (resource_rva, resource_size) = data_directory(image, opt_off, IMAGE_DIRECTORY_ENTRY_RESOURCE);
    let (exception_rva, exception_size) = data_directory(image, opt_off, IMAGE_DIRECTORY_ENTRY_EXCEPTION);
    let (tls_rva, tls_size) = data_directory(image, opt_off, IMAGE_DIRECTORY_ENTRY_TLS);
    let (delay_import_rva, delay_import_size) =
        data_directory(image, opt_off, IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT);

    Ok(Pe64Headers {
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

/// Byte offset of the first COFF [`IMAGE_SECTION_HEADER`] and how many follow (PE32+ AMD64).
#[must_use]
pub fn coff_section_table(image: &[u8]) -> Result<(usize, u16), PeValidateError> {
    if image.len() < 0x200 {
        return Err(PeValidateError::BufferTooSmall);
    }
    if u16_le(image, 0).ok_or(PeValidateError::BufferTooSmall)? != IMAGE_DOS_SIGNATURE {
        return Err(PeValidateError::BadDosSignature);
    }
    let pe_off = u32_le(image, 0x3C).ok_or(PeValidateError::BufferTooSmall)? as usize;
    if pe_off + 24 > image.len() {
        return Err(PeValidateError::BufferTooSmall);
    }
    if u32_le(image, pe_off).ok_or(PeValidateError::BufferTooSmall)? != IMAGE_NT_SIGNATURE {
        return Err(PeValidateError::BadPeSignature);
    }
    let num_sections = u16_le(image, pe_off + 6).ok_or(PeValidateError::BufferTooSmall)?;
    let size_opt = u16_le(image, pe_off + 20).ok_or(PeValidateError::BufferTooSmall)? as usize;
    let opt_off = pe_off + 24;
    let table_off = opt_off
        .checked_add(size_opt)
        .ok_or(PeValidateError::OptionalTooLarge)?;
    let need = table_off
        .checked_add(num_sections as usize * 40)
        .ok_or(PeValidateError::OptionalTooLarge)?;
    if need > image.len() {
        return Err(PeValidateError::BufferTooSmall);
    }
    Ok((table_off, num_sections))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_tiny_buffer() {
        let b = [0u8; 4];
        assert!(matches!(
            parse_pe64_headers(&b),
            Err(PeValidateError::BufferTooSmall)
        ));
    }
}
