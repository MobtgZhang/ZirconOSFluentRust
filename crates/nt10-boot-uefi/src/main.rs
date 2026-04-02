//! ZBM10 UEFI application: GOP, ACPI RSDP, FAT load of [`NT10KRNL.BIN`](../../scripts/pack-esp.sh),
//! memory map, `ExitBootServices`, jump to low physical kernel with `ZirconBootInfo *` in `%rdi`.

#![no_std]
#![no_main]

mod boot_config;
mod boot_font;
mod boot_menu;
mod boot_nv;
mod boot_rng;
mod boot_ui_gfx;
mod chainload;
mod pointer_input;
mod secure_boot;

use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};
use core::ptr::{self, addr_of_mut};

use nt10_boot_protocol::{
    FramebufferInfo, HandoffMemoryDescriptor, ZirconBootInfo, ZIRCON_BOOT_INFO_VERSION,
    ZIRNON10_MAGIC,
};
use r_efi::efi;
use r_efi::efi::protocols::file;
use r_efi::efi::protocols::loaded_image;
use r_efi::efi::protocols::simple_file_system;
use r_efi::efi::{ALLOCATE_ADDRESS, ALLOCATE_ANY_PAGES, LOADER_CODE};

const MAX_DESCRIPTORS: usize = 512;
/// Must match [`link/x86_64-uefi-load.ld`](../../../link/x86_64-uefi-load.ld) and `nt10-kernel-bin`.
const KERNEL_ENTRY_PHYS: u64 = 0x800_0000;

const DEFAULT_KERNEL_FILE: &str = "NT10KRNL.BIN";

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

fn con_out(st: *mut efi::SystemTable, s: &[u16]) {
    unsafe {
        let _ = ((*(*st).con_out).output_string)((*st).con_out, s.as_ptr() as *mut efi::Char16);
    }
}

// UCS-2 + NUL (ASCII only) for early boot failures before serial is available.
const MSG_SEC: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x0073, 0x0065, 0x0063, 0x000a, 0x0000,
];
const MSG_NO_KERNEL: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x006e, 0x006f, 0x0020, 0x006b, 0x0065,
    0x0072, 0x006e, 0x0065, 0x006c, 0x000a, 0x0000,
];
const MSG_ALLOC: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x0061, 0x006c, 0x006c, 0x006f, 0x0063,
    0x000a, 0x0000,
];
/// AllocateAddress failed but AllocateAnyPages for the same page count succeeded (physical span not free at link base).
const MSG_ALLOC_FIXED_FAIL: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x0066, 0x0069, 0x0078, 0x0061, 0x0064,
    0x0064, 0x0072, 0x000a, 0x0000,
];
const MSG_READ: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x0072, 0x0065, 0x0061, 0x0064, 0x000a,
    0x0000,
];
const MSG_LOAD: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x006c, 0x006f, 0x0061, 0x0064, 0x000a,
    0x0000,
];
const MSG_MAP: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x006d, 0x0061, 0x0070, 0x000a, 0x0000,
];
const MSG_EBS: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x0065, 0x0062, 0x0073, 0x000a, 0x0000,
];
const MSG_HANDOFF: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0020, 0x0068, 0x0061, 0x006e, 0x0064, 0x006f,
    0x0066, 0x0066, 0x000a, 0x0000,
];

fn print_pre_exit(st: *mut efi::SystemTable) {
    const S: &[u16] = &[
        0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x003a, 0x0065, 0x0078, 0x0069, 0x0074, 0x0069,
        0x006e, 0x0067, 0x0020, 0x0042, 0x006f, 0x006f, 0x0074, 0x0053, 0x0076, 0x0063, 0x0073,
        0x002e, 0x002e, 0x002e, 0x000a, 0x0000,
    ];
    con_out(st, S);
}

fn locate_gop(
    st: *mut efi::SystemTable,
) -> Result<*mut efi::protocols::graphics_output::Protocol, efi::Status> {
    use r_efi::efi::protocols::graphics_output;
    let mut handles: *mut efi::Handle = ptr::null_mut();
    let mut n_handles: usize = 0;
    unsafe {
        if (*st).hdr.revision < efi::SYSTEM_TABLE_REVISION_1_10 {
            return Err(efi::Status::UNSUPPORTED);
        }
        let r = ((*(*st).boot_services).locate_handle_buffer)(
            efi::BY_PROTOCOL,
            &graphics_output::PROTOCOL_GUID as *const _ as *mut _,
            ptr::null_mut(),
            &mut n_handles,
            &mut handles,
        );
        if r != efi::Status::SUCCESS {
            return Err(r);
        }

        let mut iface: *mut c_void = ptr::null_mut();
        let mut found = efi::Status::NOT_FOUND;
        for i in 0..n_handles {
            let h = *handles.add(i);
            let r = ((*(*st).boot_services).handle_protocol)(
                h,
                &graphics_output::PROTOCOL_GUID as *const _ as *mut _,
                &mut iface,
            );
            if r == efi::Status::SUCCESS {
                found = efi::Status::SUCCESS;
                break;
            }
        }
        let fr = ((*(*st).boot_services).free_pool)(handles as *mut c_void);
        if fr != efi::Status::SUCCESS {
            return Err(fr);
        }
        match found {
            efi::Status::SUCCESS => Ok(iface as *mut graphics_output::Protocol),
            _ => Err(efi::Status::NOT_FOUND),
        }
    }
}

fn fill_framebuffer(
    gop: *mut efi::protocols::graphics_output::Protocol,
    out: &mut FramebufferInfo,
) {
    out.base = 0;
    out.size = 0;
    if gop.is_null() {
        return;
    }
    unsafe {
        let mode_ptr = (*gop).mode;
        if mode_ptr.is_null() {
            return;
        }
        let mode = &*mode_ptr;
        out.base = mode.frame_buffer_base;
        out.size = mode.frame_buffer_size;
        let info = mode.info;
        if !info.is_null() {
            out.horizontal_resolution = (*info).horizontal_resolution;
            out.vertical_resolution = (*info).vertical_resolution;
            out.pixels_per_scan_line = (*info).pixels_per_scan_line;
            out.pixel_format = (*info).pixel_format;
        }
    }
}

fn find_acpi_rsdp(st: *mut efi::SystemTable) -> u64 {
    unsafe {
        let n = (*st).number_of_table_entries;
        let tables = (*st).configuration_table;
        if tables.is_null() || n == 0 {
            return 0;
        }
        for i in 0..n {
            let t = *tables.add(i);
            if t.vendor_guid == efi::ACPI_20_TABLE_GUID {
                return t.vendor_table as u64;
            }
        }
        0
    }
}

fn find_smbios_anchor(st: *mut efi::SystemTable) -> u64 {
    unsafe {
        let n = (*st).number_of_table_entries;
        let tables = (*st).configuration_table;
        if tables.is_null() || n == 0 {
            return 0;
        }
        for i in 0..n {
            let t = *tables.add(i);
            if t.vendor_guid == efi::SMBIOS3_TABLE_GUID {
                return t.vendor_table as u64;
            }
        }
        for i in 0..n {
            let t = *tables.add(i);
            if t.vendor_guid == efi::SMBIOS_TABLE_GUID {
                return t.vendor_table as u64;
            }
        }
        0
    }
}

fn get_memory_map(
    st: *mut efi::SystemTable,
    storage: *mut HandoffMemoryDescriptor,
    max_entries: usize,
) -> Result<(usize, usize, usize), efi::Status> {
    let bs = unsafe { (*st).boot_services };
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }

    let mut map_size: usize = 0;
    let mut map_key: usize = 0;
    let mut desc_size: usize = 0;
    let mut ver: u32 = 0;

    unsafe {
        let r = ((*bs).get_memory_map)(
            &mut map_size,
            ptr::null_mut(),
            &mut map_key,
            &mut desc_size,
            &mut ver,
        );
        if r != efi::Status::BUFFER_TOO_SMALL {
            return Err(r);
        }
    }

    let mut alloc_size = map_size + 4 * desc_size.max(size_of::<efi::MemoryDescriptor>());
    let mut raw: *mut c_void = ptr::null_mut();

    loop {
        if !raw.is_null() {
            unsafe {
                let _ = ((*bs).free_pool)(raw);
            }
            raw = ptr::null_mut();
        }
        let r = unsafe { ((*bs).allocate_pool)(efi::LOADER_DATA, alloc_size, &mut raw) };
        if r != efi::Status::SUCCESS {
            return Err(r);
        }

        let mut sz = alloc_size;
        let mut key: usize = 0;
        let mut dsz: usize = 0;
        let mut v: u32 = 0;
        let r = unsafe {
            ((*bs).get_memory_map)(
                &mut sz,
                raw as *mut efi::MemoryDescriptor,
                &mut key,
                &mut dsz,
                &mut v,
            )
        };

        if r == efi::Status::BUFFER_TOO_SMALL {
            alloc_size = sz + 2 * dsz.max(1);
            continue;
        }
        if r != efi::Status::SUCCESS {
            unsafe {
                let _ = ((*bs).free_pool)(raw);
            }
            return Err(r);
        }

        // Firmware may use a larger stride (newer UEFI adds fields at the end of each descriptor).
        // We only need the leading fields that match `HandoffMemoryDescriptor` (40 bytes on x86_64).
        let handoff_entry_size = size_of::<HandoffMemoryDescriptor>();
        if dsz < handoff_entry_size {
            unsafe {
                let _ = ((*bs).free_pool)(raw);
            }
            return Err(efi::Status::UNSUPPORTED);
        }

        let n_entries = sz / dsz;
        let copy_n = n_entries.min(max_entries);
        unsafe {
            for i in 0..copy_n {
                let src = (raw as *const u8).add(i * dsz);
                let dst = storage.add(i).cast::<u8>();
                ptr::copy_nonoverlapping(src, dst, handoff_entry_size);
            }
            // Do not free_pool here: Boot Services allocations/frees after the last successful
            // GetMemoryMap invalidate `map_key`, and ExitBootServices will fail with INVALID_PARAMETER.
        }
        return Ok((copy_n, key, handoff_entry_size));
    }
}

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

fn kernel_name_to_ucs2(name: &str, out: &mut [efi::Char16]) -> Result<usize, efi::Status> {
    let b = name.as_bytes();
    if b.is_empty() || b.len() + 1 > out.len() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    for byte in b {
        if *byte >= 0x80 {
            return Err(efi::Status::UNSUPPORTED);
        }
    }
    for (i, byte) in b.iter().enumerate() {
        out[i] = u16::from(*byte);
    }
    out[b.len()] = 0;
    Ok(b.len() + 1)
}

/// UCS-2 path under `EFI\ZirconOSFluent\<kernel>` on the FAT volume that holds `BOOTX64.EFI`.
unsafe fn open_kernel_file(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    kernel_file: &str,
) -> Result<*mut file::Protocol, efi::Status> {
    let mut ucs2_name = [0u16; 64];
    let _n = kernel_name_to_ucs2(kernel_file, &mut ucs2_name)?;

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

    let mut root: *mut file::Protocol = ptr::null_mut();
    let r = ((*sfs).open_volume)(sfs, &mut root);
    if r != efi::Status::SUCCESS || root.is_null() {
        return Err(r);
    }

    let mut efi_dir: *mut file::Protocol = ptr::null_mut();
    let mut name_efi: [efi::Char16; 4] = [0x0045, 0x0046, 0x0049, 0];
    let r = ((*root).open)(
        root,
        &mut efi_dir,
        name_efi.as_mut_ptr(),
        file::MODE_READ,
        0,
    );
    if r != efi::Status::SUCCESS {
        let _ = ((*root).close)(root);
        return Err(r);
    }

    let mut z_dir: *mut file::Protocol = ptr::null_mut();
    // "ZirconOSFluent"
    let mut name_zircon: [efi::Char16; 15] = [
        0x005a, 0x0069, 0x0072, 0x0063, 0x006f, 0x006e, 0x004f, 0x0053, 0x0046, 0x006c, 0x0075, 0x0065,
        0x006e, 0x0074, 0,
    ];
    let r = ((*efi_dir).open)(
        efi_dir,
        &mut z_dir,
        name_zircon.as_mut_ptr(),
        file::MODE_READ,
        0,
    );
    if r != efi::Status::SUCCESS {
        let _ = ((*efi_dir).close)(efi_dir);
        let _ = ((*root).close)(root);
        return Err(r);
    }

    let mut bin: *mut file::Protocol = ptr::null_mut();
    let r = ((*z_dir).open)(z_dir, &mut bin, ucs2_name.as_mut_ptr(), file::MODE_READ, 0);
    let _ = ((*z_dir).close)(z_dir);
    let _ = ((*efi_dir).close)(efi_dir);
    let _ = ((*root).close)(root);
    if r != efi::Status::SUCCESS {
        return Err(r);
    }

    Ok(bin)
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

unsafe fn load_kernel_to_fixed_phys(
    st: *mut efi::SystemTable,
    fp: *mut file::Protocol,
) -> Result<(), efi::Status> {
    let sz = match file_size(fp) {
        Ok(s) => s,
        Err(e) => {
            con_out(st, MSG_READ);
            return Err(e);
        }
    };
    if sz == 0 {
        con_out(st, MSG_LOAD);
        return Err(efi::Status::LOAD_ERROR);
    }
    let bs = (*st).boot_services;
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }

    let page_size: u64 = 4096;
    let pages = sz.div_ceil(page_size).max(1) as usize;
    let mut phys: u64 = KERNEL_ENTRY_PHYS;
    let r = ((*bs).allocate_pages)(ALLOCATE_ADDRESS, LOADER_CODE, pages, &mut phys);
    if r != efi::Status::SUCCESS {
        let mut any: u64 = 0;
        let r2 = ((*bs).allocate_pages)(ALLOCATE_ANY_PAGES, LOADER_CODE, pages, &mut any);
        if r2 == efi::Status::SUCCESS {
            unsafe {
                let _ = ((*bs).free_pages)(any, pages);
            }
            con_out(st, MSG_ALLOC_FIXED_FAIL);
        } else {
            con_out(st, MSG_ALLOC);
        }
        return Err(r);
    }

    let total = (pages as u64) * page_size;
    let base = phys as *mut u8;
    ptr::write_bytes(base, 0, total as usize);

    let mut off: usize = 0;
    while off < sz as usize {
        let mut chunk = (sz as usize - off).min(4 * 1024 * 1024);
        let r = ((*fp).read)(fp, &mut chunk, base.add(off).cast());
        if r != efi::Status::SUCCESS {
            con_out(st, MSG_READ);
            return Err(r);
        }
        if chunk == 0 {
            con_out(st, MSG_READ);
            return Err(efi::Status::LOAD_ERROR);
        }
        off += chunk;
    }

    Ok(())
}

#[inline(never)]
unsafe fn jump_to_kernel(handoff_phys: u64, entry_phys: u64) -> ! {
    core::arch::asm!(
        "cli",
        "mov rdi, {h}",
        "mov rax, {e}",
        "jmp rax",
        h = in(reg) handoff_phys,
        e = in(reg) entry_phys,
        options(nomem, noreturn),
    );
}

static mut HANDOFF: MaybeUninit<ZirconBootInfo> = MaybeUninit::uninit();
static mut MEM_MAP_STORE: [HandoffMemoryDescriptor; MAX_DESCRIPTORS] = [HandoffMemoryDescriptor {
    r#type: 0,
    _padding: 0,
    physical_start: 0,
    virtual_start: 0,
    number_of_pages: 0,
    attribute: 0,
}; MAX_DESCRIPTORS];

#[export_name = "efi_main"]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // UEFI entry: `st` is the firmware-provided system table pointer.
pub extern "C" fn efi_main(image: efi::Handle, st: *mut efi::SystemTable) -> efi::Status {
    if st.is_null() {
        return efi::Status::INVALID_PARAMETER;
    }

    if let Err(e) = secure_boot::optional_pre_boot_security_hook(st) {
        con_out(st, MSG_SEC);
        return e;
    }

    let mut zcfg = unsafe { boot_config::load_zbm10_cfg(st, image) };
    let mut kernel_path_buf = [0u8; boot_config::NAME_MAX];
    let kernel_path = {
        let s = zcfg.kernel_name_str().unwrap_or(DEFAULT_KERNEL_FILE).as_bytes();
        let n = s.len().min(kernel_path_buf.len());
        kernel_path_buf[..n].copy_from_slice(&s[..n]);
        core::str::from_utf8(&kernel_path_buf[..n]).unwrap_or(DEFAULT_KERNEL_FILE)
    };

    // GOP is optional: headless QEMU (`-display none`) and some firmware builds omit
    // `EFI_GRAPHICS_OUTPUT_PROTOCOL`; failing here made `efi_main` return early so the Shell
    // looked like "the loader never ran" and Bds could report `Unsupported`.
    let gop = locate_gop(st).unwrap_or(ptr::null_mut());

    if let Err(e) =
        unsafe { boot_menu::run_boot_menu(st, image, gop, kernel_path, &mut zcfg) }
    {
        return e;
    }

    let kernel_open = zcfg.kernel_name_str().unwrap_or(DEFAULT_KERNEL_FILE);
    let kernel_fp = match unsafe { open_kernel_file(st, image, kernel_open) } {
        Ok(p) => p,
        Err(e) => {
            con_out(st, MSG_NO_KERNEL);
            return e;
        }
    };

    let load_r = unsafe { load_kernel_to_fixed_phys(st, kernel_fp) };
    unsafe {
        let _ = ((*kernel_fp).close)(kernel_fp);
    }
    if let Err(e) = load_r {
        return e;
    }

    let acpi = find_acpi_rsdp(st);
    let smbios = find_smbios_anchor(st);

    let mut fb = FramebufferInfo {
        base: 0,
        size: 0,
        horizontal_resolution: 0,
        vertical_resolution: 0,
        pixels_per_scan_line: 0,
        pixel_format: 0,
    };
    fill_framebuffer(gop, &mut fb);

    let mut firmware_rng_seed = [0u8; 16];
    unsafe {
        boot_rng::try_fill_seed(st, &mut firmware_rng_seed);
    }

    // Tell the user we're about to leave firmware; must run before the final GetMemoryMap —
    // ConOut may use Boot Services that invalidate the map key.
    print_pre_exit(st);

    // Capture the map immediately before ExitBootServices (after all other allocators above).
    let store = addr_of_mut!(MEM_MAP_STORE).cast::<HandoffMemoryDescriptor>();
    let (count, map_key, dsz) = match get_memory_map(st, store, MAX_DESCRIPTORS) {
        Ok(x) => x,
        Err(e) => {
            con_out(st, MSG_MAP);
            return e;
        }
    };

    let info = ZirconBootInfo {
        magic: ZIRNON10_MAGIC,
        version: ZIRCON_BOOT_INFO_VERSION,
        reserved0: 0,
        mem_map: store,
        mem_map_count: count,
        mem_map_descriptor_size: dsz,
        framebuffer: fb,
        acpi_rsdp: acpi,
        kernel_entry_phys: KERNEL_ENTRY_PHYS,
        cmdline: [0; 256],
        initrd_phys: 0,
        initrd_size: 0,
        tpm_mmio_phys: 0,
        tpm_mmio_size: 0,
        smbios_anchor_phys: smbios,
        firmware_rng_seed,
        acpi_rsdp_revision: 0,
        _reserved_tail: [0; 7],
    };

    if !info.validate() {
        con_out(st, MSG_HANDOFF);
        return efi::Status::LOAD_ERROR;
    }

    unsafe {
        (*core::ptr::addr_of_mut!(HANDOFF)).write(info);
    }

    let exit_r = unsafe { ((*(*st).boot_services).exit_boot_services)(image, map_key) };
    if exit_r != efi::Status::SUCCESS {
        con_out(st, MSG_EBS);
        return exit_r;
    }

    let handoff_phys = core::ptr::addr_of_mut!(HANDOFF) as usize as u64;
    unsafe {
        jump_to_kernel(handoff_phys, KERNEL_ENTRY_PHYS);
    }
}
