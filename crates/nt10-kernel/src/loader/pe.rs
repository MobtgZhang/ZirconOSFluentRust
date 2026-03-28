//! PE32+ loader — sizes and signatures from the public PE/COFF spec (independent implementation).

/// DOS header `e_magic`.
pub const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D;
/// PE header `Signature`.
pub const IMAGE_NT_SIGNATURE: u32 = 0x0000_4550;

/// `IMAGE_FILE_MACHINE_AMD64`
pub const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
/// `IMAGE_FILE_MACHINE_I386` (WOW64 bring-up).
pub const IMAGE_FILE_MACHINE_I386: u16 = 0x014C;

/// `IMAGE_NT_OPTIONAL_HDR64_MAGIC`
pub const IMAGE_NT_OPTIONAL_HDR64_MAGIC: u16 = 0x20B;
/// `IMAGE_NT_OPTIONAL_HDR32_MAGIC`
pub const IMAGE_NT_OPTIONAL_HDR32_MAGIC: u16 = 0x10B;

/// `IMAGE_DLLCHARACTERISTICS_NX_COMPAT` — optional header DllCharacteristics bit (PE/COFF).
pub const IMAGE_DLLCHARACTERISTICS_NX_COMPAT: u16 = 0x0100;

/// `IMAGE_SUBSYSTEM_WINDOWS_GUI` — PE optional header `Subsystem` (graphical).
pub const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;
/// `IMAGE_SUBSYSTEM_WINDOWS_CUI` — console subsystem.
pub const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;

/// `IMAGE_DIRECTORY_ENTRY_RESOURCE` index into the PE data directory.
pub const IMAGE_DIRECTORY_ENTRY_RESOURCE: usize = 2;
/// `IMAGE_DIRECTORY_ENTRY_EXCEPTION`
pub const IMAGE_DIRECTORY_ENTRY_EXCEPTION: usize = 3;
/// `IMAGE_DIRECTORY_ENTRY_BASERELOC` (parsed separately for bring-up).
pub const IMAGE_DIRECTORY_ENTRY_BASERELOC: usize = 5;
/// `IMAGE_DIRECTORY_ENTRY_TLS`
pub const IMAGE_DIRECTORY_ENTRY_TLS: usize = 9;
/// `IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT`
pub const IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT: usize = 13;

/// Read U16 from PE base at `offset` if in range.
///
/// # Safety
/// `base` must be valid for `len` bytes.
#[must_use]
pub unsafe fn read_u16(base: *const u8, len: usize, offset: usize) -> Option<u16> {
    if offset.checked_add(2)? > len {
        return None;
    }
    Some(u16::from_le_bytes([
        *base.add(offset),
        *base.add(offset + 1),
    ]))
}

/// PE header offset from DOS `e_lfanew`.
///
/// # Safety
/// `base` valid for `len` bytes; MZ + PE signatures must be verified by caller.
#[must_use]
pub unsafe fn pe_header_offset(base: *const u8, len: usize) -> Option<usize> {
    if read_u16(base, len, 0)? != IMAGE_DOS_SIGNATURE {
        return None;
    }
    let e_lfanew = read_u32(base, len, 0x3C)?;
    Some(e_lfanew as usize)
}

/// # Safety
/// Same as `read_u16`.
#[must_use]
pub unsafe fn read_u32(base: *const u8, len: usize, offset: usize) -> Option<u32> {
    if offset.checked_add(4)? > len {
        return None;
    }
    Some(u32::from_le_bytes([
        *base.add(offset),
        *base.add(offset + 1),
        *base.add(offset + 2),
        *base.add(offset + 3),
    ]))
}

/// Byte offset of the PE optional header (immediately after the 20-byte COFF file header).
///
/// # Safety
/// `pe_off` must point at the PE signature (`PE\0\0`); `base`/`len` must cover the COFF header.
#[must_use]
pub unsafe fn optional_header_offset(base: *const u8, len: usize, pe_off: usize) -> Option<usize> {
    if pe_off.checked_add(24)? > len {
        return None;
    }
    if read_u32(base, len, pe_off)? != IMAGE_NT_SIGNATURE {
        return None;
    }
    Some(pe_off + 24)
}
