//! Kernel entry after firmware or multiboot-style loader.

use crate::arch::x86_64::{gdt, idt, paging};
use crate::hal::x86_64::serial;
use crate::hal::{Hal, X86Hal64};
use crate::handoff::ZirconBootInfo;
use nt10_boot_protocol::FramebufferInfo;

/// Called with `boot == null` for QEMU `-kernel`; UEFI handoff passes physical `ZirconBootInfo *` in `%rdi`.
///
/// # Safety
/// If `boot` is non-null, it must point to a valid `ZirconBootInfo` from ZBM10.
#[no_mangle]
pub unsafe extern "C" fn kmain_entry(boot: *const ZirconBootInfo) -> ! {
    let hal = X86Hal64;
    serial::init();
    hal.debug_write(b"nt10-kernel: serial online (COM1)\r\n");

    #[cfg(target_arch = "x86_64")]
    {
        if paging::using_builtin_page_tables() {
            gdt::install();
            hal.debug_write(b"nt10-kernel: GDT loaded (built-in layout)\r\n");
        } else {
            hal.debug_write(b"nt10-kernel: keeping firmware GDT (UEFI)\r\n");
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        gdt::install();
        hal.debug_write(b"nt10-kernel: GDT loaded\r\n");
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::arch::x86_64::syscall::enable_extension_stub();
        hal.debug_write(b"nt10-kernel: EFER.SCE enabled (syscall path prep)\r\n");
    }

    unsafe {
        idt::init();
    }
    hal.debug_write(b"nt10-kernel: IDT loaded\r\n");

    unsafe {
        paging::init_low_identity();
    }
    hal.debug_write(b"nt10-kernel: early identity map (512 MiB, 2 MiB pages)\r\n");
    hal.flush_tlb_all();

    let mut uefi_desktop_poll_fb: Option<FramebufferInfo> = None;

    if !boot.is_null() {
        let info = &*boot;
        match crate::mm::boot_mem::validate_boot_info(info) {
            Ok(()) => {
                hal.debug_write(b"nt10-kernel: ZirconBootInfo extended checks OK\r\n");
                log_usize_hal(&hal, b"nt10-kernel: mem_map_count=", info.mem_map_count);
                let conv = unsafe { crate::mm::early_map::conventional_page_count(info) };
                log_u64_hal(&hal, b"nt10-kernel: conventional_pages=", conv);
                let usable =
                    unsafe { crate::mm::boot_mem::total_usable_pages(info) };
                log_u64_hal(
                    &hal,
                    b"nt10-kernel: usable_conventional_pages_minus_kernel=",
                    usable,
                );
                let rsv = unsafe { crate::mm::early_map::reserved_firmware_page_count(info) };
                log_u64_hal(&hal, b"nt10-kernel: acpi_runtime_pages=", rsv);
                if info.smbios_anchor_phys != 0 {
                    log_u64_hal(
                        &hal,
                        b"nt10-kernel: smbios_anchor_phys=",
                        info.smbios_anchor_phys,
                    );
                }
                let mut ranges = [crate::mm::boot_mem::UsablePhysRange {
                    base: 0,
                    page_count: 0,
                }; 8];
                let nr = unsafe {
                    crate::mm::boot_mem::usable_conventional_ranges(info, &mut ranges)
                };
                log_usize_hal(&hal, b"nt10-kernel: usable_range_slots=", nr);
                if nr > 0 {
                    log_u64_hal(&hal, b"nt10-kernel: usable[0].base=", ranges[0].base);
                    log_u64_hal(&hal, b"nt10-kernel: usable[0].pages=", ranges[0].page_count);
                }
                match crate::drivers::video::display_mgr::parse_framebuffer_handoff(&info.framebuffer) {
                    Ok(fb) => {
                        hal.debug_write(b"nt10-kernel: GOP framebuffer handoff OK\r\n");
                        log_u64_hal(&hal, b"nt10-kernel: fb_base_phys=", fb.base_phys);
                        log_usize_hal(&hal, b"nt10-kernel: fb_byte_len=", fb.byte_len);
                        log_usize_hal(
                            &hal,
                            b"nt10-kernel: fb_width_px=",
                            fb.width_px as usize,
                        );
                        log_usize_hal(
                            &hal,
                            b"nt10-kernel: fb_height_px=",
                            fb.height_px as usize,
                        );
                        // Previously we only validated GOP; the panel stayed black. Paint wallpaper + taskbar.
                        crate::desktop::fluent::shell::paint_uefi_desktop_shell(&info.framebuffer);
                        let _ = crate::drivers::video::display_mgr::register_uefi_framebuffer_stub(
                            &info.framebuffer,
                        );
                        hal.debug_write(b"nt10-kernel: desktop shell painted (UEFI bring-up)\r\n");
                        uefi_desktop_poll_fb = Some(info.framebuffer);
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::NullBase,
                    ) => {
                        hal.debug_write(b"nt10-kernel: GOP handoff rejected (null base)\r\n");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::ZeroSize,
                    ) => {
                        hal.debug_write(b"nt10-kernel: GOP handoff rejected (zero size)\r\n");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::ZeroWidth,
                    ) => {
                        hal.debug_write(b"nt10-kernel: GOP handoff rejected (zero width)\r\n");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::ZeroHeight,
                    ) => {
                        hal.debug_write(b"nt10-kernel: GOP handoff rejected (zero height)\r\n");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::StrideTooSmall,
                    ) => {
                        hal.debug_write(b"nt10-kernel: GOP handoff rejected (stride)\r\n");
                    }
                }
            }
            Err(crate::mm::boot_mem::BootInfoError::MagicOrVersion) => {
                hal.debug_write(b"nt10-kernel: handoff magic/version invalid\r\n");
            }
            Err(crate::mm::boot_mem::BootInfoError::NullMemoryMap) => {
                hal.debug_write(b"nt10-kernel: handoff mem_map null with count>0\r\n");
            }
            Err(crate::mm::boot_mem::BootInfoError::BadDescriptorSize) => {
                hal.debug_write(b"nt10-kernel: handoff descriptor size != 40\r\n");
            }
        }
    } else {
        hal.debug_write(b"nt10-kernel: no UEFI handoff (null pointer)\r\n");
    }

    crate::subsystems::win32::csrss_host::bringup_kernel_thread_smoke();

    // QEMU `-kernel` / no firmware paging: we own PIC+LAPIC bring-up and enable IRQs.
    // UEFI: firmware left IOAPIC/virtual-wire + LAPIC in its own state; arming PIT/PIC or
    // reprogramming LAPIC then `sti` often causes IRQ storms or bad frames → triple fault
    // and an apparent “reboot loop” under OVMF.
    #[cfg(target_arch = "x86_64")]
    {
        if paging::using_builtin_page_tables() {
            crate::ke::sched::bringup_timer_and_idle(&hal);
        } else {
            hal.debug_write(
                b"nt10-kernel: UEFI path - skip HW timer and sti (avoid firmware IRQ clash)\r\n",
            );
            crate::ke::apc::enqueue_bringup_sample();
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        crate::ke::sched::bringup_timer_and_idle(&hal);
    }

    crate::ke::apc::deliver_pending_at_passive();
    hal.debug_write(b"nt10-kernel: KAPC drained at PASSIVE_LEVEL\r\n");

    #[cfg(target_arch = "x86_64")]
    {
        if paging::using_builtin_page_tables() {
            unsafe {
                crate::arch::x86_64::syscall::install_syscall_msrs_bringup();
            }
            hal.debug_write(b"nt10-kernel: syscall MSRs STAR/LSTAR/FMASK\r\n");

            let mut proc = crate::ps::process::EProcess::new_bootstrap();
            let _ = crate::mm::bringup_user::register_bringup_vad(&mut proc.vad_root);
            proc.peb = crate::ps::peb::PebRef::bringup_smoke();

            let stub = crate::ke::sched::ThreadStub::new(8);
            let tid = stub.id;
            let _ = crate::ke::sched::rr_register_thread(stub);
            let _ethread =
                crate::ps::thread::EThread::new_system_thread(proc.pid, tid);
            let _ = (_ethread, proc);

            unsafe {
                crate::mm::bringup_user::copy_user_smoke_code(
                    crate::mm::user_va::USER_BRINGUP_VA as *mut u8,
                );
            }
            hal.debug_write(b"nt10-kernel: entering ring3 syscall smoke\r\n");
            let rsp = crate::mm::user_va::USER_BRINGUP_STACK_TOP - 16;
            let rip = crate::mm::bringup_user::user_code_entry_va();
            unsafe {
                crate::arch::x86_64::user_enter::zircon_enter_ring3(rip, rsp);
            }
        } else {
            hal.debug_write(b"nt10-kernel: skip ring3 smoke (not built-in CR3)\r\n");
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Poll session draws software cursor + input; runs whenever GOP was handed off. The built-in
        // page-table path jumps to ring3 above and does not return here.
        if let Some(fb) = uefi_desktop_poll_fb {
            hal.debug_write(b"nt10-kernel: entering UEFI desktop poll session\r\n");
            crate::desktop::fluent::session::run_uefi_desktop_poll_session(&hal, fb);
        }
    }

    hal.debug_write(b"nt10-kernel: entering idle (hlt)\r\n");
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}

fn log_usize_hal<H: Hal + ?Sized>(hal: &H, prefix: &[u8], n: usize) {
    hal.debug_write(prefix);
    let mut buf = [0u8; 24];
    let mut i = buf.len();
    let mut v = n;
    if v == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while v > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
    hal.debug_write(b"\r\n");
}

fn log_u64_hal<H: Hal + ?Sized>(hal: &H, prefix: &[u8], n: u64) {
    hal.debug_write(prefix);
    let mut buf = [0u8; 24];
    let mut i = buf.len();
    let mut v = n;
    if v == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while v > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
    hal.debug_write(b"\r\n");
}
