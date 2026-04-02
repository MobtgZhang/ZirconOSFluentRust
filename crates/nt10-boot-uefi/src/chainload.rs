//! Load another UEFI PE image from the ESP into memory and `StartImage` (e.g. Microsoft Boot Manager).

use core::ffi::c_void;
use core::ptr;

use r_efi::efi;
use r_efi::efi::protocols::file;
use r_efi::efi::protocols::loaded_image;
use r_efi::efi::protocols::simple_file_system;

unsafe fn handle_protocol<T>(
    st: *mut efi::SystemTable,
    handle: efi::Handle,
    guid: *mut efi::Guid,
) -> Result<*mut T, efi::Status> {
    let bs = (*st).boot_services;
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    let mut iface: *mut c_void = ptr::null_mut();
    let r = ((*bs).handle_protocol)(handle, guid, &mut iface);
    if r != efi::Status::SUCCESS {
        return Err(r);
    }
    Ok(iface.cast())
}

fn ascii_to_ucs2_component(name: &[u8], out: &mut [efi::Char16]) -> Result<usize, efi::Status> {
    if name.is_empty() || name.len() + 1 > out.len() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    for &b in name {
        if b >= 0x80 || b == 0 {
            return Err(efi::Status::INVALID_PARAMETER);
        }
        if b == b'/' || b == b'\\' {
            return Err(efi::Status::INVALID_PARAMETER);
        }
    }
    for (i, &b) in name.iter().enumerate() {
        out[i] = u16::from(b);
    }
    out[name.len()] = 0;
    Ok(name.len() + 1)
}

/// Open a file on the same volume as `image` by path relative to ESP root (uses `\` or `/` as separators).
pub unsafe fn open_esp_path(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    path: &[u8],
) -> Result<*mut file::Protocol, efi::Status> {
    let li: *mut loaded_image::Protocol = handle_protocol(
        st,
        image,
        &loaded_image::PROTOCOL_GUID as *const _ as *mut _,
    )?;
    let dev = (*li).device_handle;
    if dev.is_null() {
        return Err(efi::Status::NOT_FOUND);
    }
    let sfs: *mut simple_file_system::Protocol = handle_protocol(
        st,
        dev,
        &simple_file_system::PROTOCOL_GUID as *const _ as *mut _,
    )?;

    let mut cur: *mut file::Protocol = ptr::null_mut();
    let r = ((*sfs).open_volume)(sfs, &mut cur);
    if r != efi::Status::SUCCESS || cur.is_null() {
        return Err(r);
    }

    let mut start = 0usize;
    while start < path.len() && (path[start] == b'\\' || path[start] == b'/') {
        start += 1;
    }

    let mut i = start;
    while i <= path.len() {
        if i == path.len() || path[i] == b'\\' || path[i] == b'/' {
            if i > start {
                let comp = &path[start..i];
                let mut ucs2 = [0u16; 128];
                let _n = ascii_to_ucs2_component(comp, &mut ucs2)?;
                let mut next: *mut file::Protocol = ptr::null_mut();
                let rr = ((*cur).open)(
                    cur,
                    &mut next,
                    ucs2.as_mut_ptr(),
                    file::MODE_READ,
                    0,
                );
                let _ = ((*cur).close)(cur);
                if rr != efi::Status::SUCCESS || next.is_null() {
                    return Err(rr);
                }
                cur = next;
            }
            start = i + 1;
        }
        i += 1;
    }

    Ok(cur)
}

unsafe fn file_size(fp: *mut file::Protocol) -> Result<u64, efi::Status> {
    let r = ((*fp).set_position)(fp, 0xffff_ffff_ffff_ffff);
    if r != efi::Status::SUCCESS {
        return Err(r);
    }
    let mut pos: u64 = 0;
    let r = ((*fp).get_position)(fp, &mut pos);
    if r != efi::Status::SUCCESS {
        return Err(r);
    }
    let _ = ((*fp).set_position)(fp, 0);
    Ok(pos)
}

/// Read entire file into boot-services pool memory.
unsafe fn read_file_pool(
    st: *mut efi::SystemTable,
    fp: *mut file::Protocol,
) -> Result<(*mut u8, usize), efi::Status> {
    let sz = file_size(fp)?;
    if sz == 0 || sz > 64 * 1024 * 1024 {
        return Err(efi::Status::LOAD_ERROR);
    }
    let bs = (*st).boot_services;
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    let mut raw: *mut c_void = ptr::null_mut();
    let r = ((*bs).allocate_pool)(efi::LOADER_DATA, sz as usize, &mut raw);
    if r != efi::Status::SUCCESS || raw.is_null() {
        return Err(r);
    }
    let base = raw.cast::<u8>();
    let mut off = 0usize;
    while off < sz as usize {
        let mut chunk = (sz as usize - off).min(4 * 1024 * 1024);
        let rr = ((*fp).read)(fp, &mut chunk, base.add(off).cast());
        if rr != efi::Status::SUCCESS {
            let _ = ((*bs).free_pool)(raw);
            return Err(rr);
        }
        if chunk == 0 {
            let _ = ((*bs).free_pool)(raw);
            return Err(efi::Status::LOAD_ERROR);
        }
        off += chunk;
    }
    Ok((base, sz as usize))
}

/// Load PE/COFF from buffer and transfer control via `StartImage` (does not return on success).
pub unsafe fn start_pe_from_buffer(
    st: *mut efi::SystemTable,
    parent: efi::Handle,
    image_bytes: *mut u8,
    image_size: usize,
) -> efi::Status {
    let bs = (*st).boot_services;
    if bs.is_null() {
        return efi::Status::INVALID_PARAMETER;
    }
    let mut new_image: efi::Handle = ptr::null_mut();
    let r = ((*bs).load_image)(
        efi::Boolean::FALSE,
        parent,
        ptr::null_mut(),
        image_bytes.cast(),
        image_size,
        &mut new_image,
    );
    // Firmware copies out of SourceBuffer; pool can be released before StartImage.
    let _ = ((*bs).free_pool)(image_bytes.cast());
    if r != efi::Status::SUCCESS {
        return r;
    }
    let mut exit_sz = 0usize;
    let mut exit_data: *mut efi::Char16 = ptr::null_mut();
    let sr = ((*bs).start_image)(new_image, &mut exit_sz, &mut exit_data);
    let _ = ((*bs).unload_image)(new_image);
    sr
}

/// Open path, load into memory, `StartImage`. Returns firmware status if the child exits or fails to start.
pub unsafe fn chainload_efi_path(
    st: *mut efi::SystemTable,
    parent: efi::Handle,
    path: &[u8],
) -> efi::Status {
    let fp = match open_esp_path(st, parent, path) {
        Ok(p) => p,
        Err(e) => return e,
    };
    let (buf, sz) = match read_file_pool(st, fp) {
        Ok(x) => x,
        Err(e) => {
            let _ = ((*fp).close)(fp);
            return e;
        }
    };
    let _ = ((*fp).close)(fp);
    start_pe_from_buffer(st, parent, buf, sz)
}
