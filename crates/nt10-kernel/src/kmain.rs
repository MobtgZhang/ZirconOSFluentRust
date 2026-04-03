//! Kernel entry after firmware or multiboot-style loader.
//!
//! # Documented bring-up order (ZirconOSFluent)
//!
//! 1. **Serial** — [`crate::hal::x86_64::serial::init`], first logs.
//! 2. **CPU tables** — GDT (built-in vs UEFI), [`crate::arch::x86_64::idt::init`], `#PF` + TLB IPI vectors,
//!    [`crate::arch::x86_64::syscall::enable_extension_stub`].
//! 3. **Early MM** — NX, low identity map ([`crate::arch::x86_64::paging::init_low_identity`]).
//! 4. **Handoff** — If `boot != null`: [`crate::mm::boot_mem::validate_boot_info`], PFN / heap probe,
//!    framebuffer parse, optional high-half mirror ([`crate::mm::high_half`]).
//!    Invalid magic/version/map is logged and **skipped** (no PFN init from corrupt data); see
//!    [`crate::infra_bringup::log_invalid_handoff`].
//! 5. **Subsystems** — CSRSS host smoke, timer/APC vs UEFI-safe path, optional ring-3 bring-up.
//! 6. **Idle** — `hlt` loop (UEFI desktop poll may run before this).
//!
//! Serial checkpoints: [`crate::infra_bringup`] (`kmain_phase_begin`, phase id).

use crate::arch::x86_64::{gdt, idt, paging};
use crate::hal::x86_64::serial;
use crate::hal::{Hal, X86Hal64};
use crate::handoff::ZirconBootInfo;
use crate::rtl::log::{
    log_line_hal, log_u64_hal, log_usize_hal, SUB_BOOT, SUB_KE, SUB_MM, SUB_SESS, SUB_SYSC,
    SUB_VID,
};
use nt10_boot_protocol::FramebufferInfo;

/// Called with `boot == null` for QEMU `-kernel`; UEFI handoff passes physical `ZirconBootInfo *` in `%rdi`.
///
/// # Safety
/// If `boot` is non-null, it must point to a valid `ZirconBootInfo` from ZBM10.
#[no_mangle]
pub unsafe extern "C" fn kmain_entry(boot: *const ZirconBootInfo) -> ! {
    let hal = X86Hal64;
    serial::init();
    log_line_hal(&hal, SUB_BOOT, b"serial online (COM1)");
    crate::infra_bringup::run_serial_selftest_hooks(&hal);
    crate::infra_bringup::log_init_checkpoint(
        &hal,
        crate::infra_bringup::PHASE_SERIAL,
        b"phase_serial_done",
    );

    #[cfg(target_arch = "x86_64")]
    {
        if paging::using_builtin_page_tables() {
            gdt::install();
            log_line_hal(&hal, SUB_BOOT, b"GDT loaded (built-in layout)");
        } else {
            log_line_hal(&hal, SUB_BOOT, b"keeping firmware GDT (UEFI)");
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        gdt::install();
        log_line_hal(&hal, SUB_BOOT, b"GDT loaded");
    }

    #[cfg(target_arch = "x86_64")]
    {
        crate::arch::x86_64::syscall::enable_extension_stub();
        log_line_hal(&hal, SUB_BOOT, b"EFER.SCE enabled (syscall path prep)");
    }

    unsafe {
        idt::init();
        idt::set_interrupt_gate(14, crate::arch::x86_64::isr::page_fault_entry_addr());
        idt::set_interrupt_gate(
            usize::from(crate::arch::x86_64::tlb::TLB_FLUSH_IPI_VECTOR),
            crate::arch::x86_64::tlb::tlb_flush_ipi_entry_addr(),
        );
    }
    log_line_hal(&hal, SUB_BOOT, b"IDT loaded (#PF 14, TLB IPI)");
    crate::infra_bringup::log_init_checkpoint(
        &hal,
        crate::infra_bringup::PHASE_CPU_STATE,
        b"phase_cpu_state_done",
    );

    unsafe {
        paging::enable_nxe();
    }
    log_line_hal(&hal, SUB_BOOT, b"EFER.NXE enabled");

    unsafe {
        paging::init_low_identity();
    }
    log_line_hal(&hal, SUB_MM, b"early identity map (512 MiB, 2 MiB pages)");
    hal.flush_tlb_all();
    crate::infra_bringup::log_init_checkpoint(
        &hal,
        crate::infra_bringup::PHASE_EARLY_MM,
        b"phase_early_mm_done",
    );

    let mut uefi_desktop_poll_fb: Option<FramebufferInfo> = None;
    #[cfg(target_arch = "x86_64")]
    let mut uefi_high_half_ok = false;

    if !boot.is_null() {
        let info = &*boot;
        match crate::mm::boot_mem::validate_boot_info(info) {
            Ok(()) => {
                crate::infra_bringup::log_init_checkpoint(
                    &hal,
                    crate::infra_bringup::PHASE_BOOT_INFO,
                    b"phase_boot_info_ok",
                );
                log_line_hal(&hal, SUB_BOOT, b"ZirconBootInfo extended checks OK");
                log_usize_hal(&hal, SUB_MM, b"mem_map_count=", info.mem_map_count);
                let conv = unsafe { crate::mm::early_map::conventional_page_count(info) };
                log_u64_hal(&hal, SUB_MM, b"conventional_pages=", conv);
                let usable =
                    unsafe { crate::mm::boot_mem::total_usable_pages(info) };
                log_u64_hal(
                    &hal,
                    SUB_MM,
                    b"usable_conventional_pages_minus_kernel=",
                    usable,
                );
                let rsv = unsafe { crate::mm::early_map::reserved_firmware_page_count(info) };
                log_u64_hal(&hal, SUB_MM, b"acpi_runtime_pages=", rsv);
                if info.smbios_anchor_phys != 0 {
                    log_u64_hal(
                        &hal,
                        SUB_MM,
                        b"smbios_anchor_phys=",
                        info.smbios_anchor_phys,
                    );
                }
                let mut ranges = [crate::mm::boot_mem::UsablePhysRange {
                    base: 0,
                    page_count: 0,
                }; crate::mm::phys::USABLE_RANGE_SLOTS];
                let nr = unsafe {
                    crate::mm::boot_mem::usable_conventional_ranges(info, &mut ranges)
                };
                log_usize_hal(&hal, SUB_MM, b"usable_range_slots=", nr);
                if nr > 0 {
                    log_u64_hal(&hal, SUB_MM, b"usable[0].base=", ranges[0].base);
                    log_u64_hal(&hal, SUB_MM, b"usable[0].pages=", ranges[0].page_count);
                }
                unsafe {
                    crate::mm::phys::pfn_bringup_init(info);
                }
                if unsafe { crate::mm::heap::kernel_heap_bringup_reserve_pages(1) } {
                    log_line_hal(&hal, SUB_MM, b"PFN bump + 1-page kernel heap arena OK");
                } else {
                    log_line_hal(
                        &hal,
                        SUB_MM,
                        b"kernel heap arena init skipped (no PFN / non-contiguous)",
                    );
                }
                let probe = crate::mm::heap::kernel_bump_alloc(16, 64);
                if !probe.is_null() {
                    log_line_hal(&hal, SUB_MM, b"kernel_bump_alloc probe OK");
                }
                match crate::drivers::video::display_mgr::parse_framebuffer_handoff(&info.framebuffer) {
                    Ok(fb) => {
                        crate::infra_bringup::log_init_checkpoint(
                            &hal,
                            crate::infra_bringup::PHASE_VIDEO_HANDOFF,
                            b"phase_video_handoff_ok",
                        );
                        log_line_hal(&hal, SUB_VID, b"GOP framebuffer handoff OK");
                        log_u64_hal(&hal, SUB_VID, b"fb_base_phys=", fb.base_phys);
                        log_usize_hal(&hal, SUB_VID, b"fb_byte_len=", fb.byte_len);
                        log_usize_hal(
                            &hal,
                            SUB_VID,
                            b"fb_width_px=",
                            fb.width_px as usize,
                        );
                        log_usize_hal(
                            &hal,
                            SUB_VID,
                            b"fb_height_px=",
                            fb.height_px as usize,
                        );
                        // Previously we only validated GOP; the panel stayed black. Paint wallpaper + taskbar.
                        crate::desktop::fluent::shell::paint_uefi_desktop_shell(&info.framebuffer);
                        let _ = crate::drivers::video::display_mgr::register_uefi_framebuffer_stub(
                            &info.framebuffer,
                        );
                        if unsafe { crate::mm::high_half::try_map_uefi_framebuffer_wc(info) }.is_ok()
                        {
                            log_line_hal(&hal, SUB_VID, b"framebuffer WC vmap at high VA OK");
                        }
                        log_line_hal(&hal, SUB_SESS, b"desktop shell painted (UEFI bring-up)");
                        uefi_desktop_poll_fb = Some(info.framebuffer);
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::NullBase,
                    ) => {
                        log_line_hal(&hal, SUB_VID, b"GOP handoff rejected (null base)");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::ZeroSize,
                    ) => {
                        log_line_hal(&hal, SUB_VID, b"GOP handoff rejected (zero size)");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::ZeroWidth,
                    ) => {
                        log_line_hal(&hal, SUB_VID, b"GOP handoff rejected (zero width)");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::ZeroHeight,
                    ) => {
                        log_line_hal(&hal, SUB_VID, b"GOP handoff rejected (zero height)");
                    }
                    Err(
                        crate::drivers::video::display_mgr::FramebufferHandoffError::StrideTooSmall,
                    ) => {
                        log_line_hal(&hal, SUB_VID, b"GOP handoff rejected (stride)");
                    }
                }
                #[cfg(target_arch = "x86_64")]
                {
                    if unsafe {
                        crate::mm::high_half::try_uefi_add_kernel_direct_map_mirror_and_switch()
                    }
                    .is_ok()
                    {
                        log_line_hal(
                            &hal,
                            SUB_MM,
                            b"PML4[256] high-half 512MiB mirror + CR3 switch OK",
                        );
                        uefi_high_half_ok = true;
                    }
                }
            }
            Err(e) => {
                crate::infra_bringup::log_invalid_handoff(&hal, e);
                match e {
                    crate::mm::boot_mem::BootInfoError::MagicOrVersion => {
                        log_line_hal(&hal, SUB_BOOT, b"handoff magic/version invalid");
                    }
                    crate::mm::boot_mem::BootInfoError::NullMemoryMap => {
                        log_line_hal(&hal, SUB_BOOT, b"handoff mem_map null with count>0");
                    }
                    crate::mm::boot_mem::BootInfoError::BadDescriptorSize => {
                        log_line_hal(&hal, SUB_BOOT, b"handoff descriptor size != 40");
                    }
                }
            }
        }
    } else {
        log_line_hal(&hal, SUB_BOOT, b"no UEFI handoff (null pointer)");
    }

    crate::infra_bringup::log_init_checkpoint(
        &hal,
        crate::infra_bringup::PHASE_SUBSYSTEMS,
        b"phase_subsystems_begin",
    );
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
            log_line_hal(
                &hal,
                SUB_BOOT,
                b"UEFI path - skip HW timer and sti (avoid firmware IRQ clash)",
            );
            crate::ke::apc::enqueue_bringup_sample();
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        crate::ke::sched::bringup_timer_and_idle(&hal);
    }

    crate::ke::apc::deliver_pending_at_passive();
    log_line_hal(&hal, SUB_KE, b"KAPC drained at PASSIVE_LEVEL");

    #[cfg(target_arch = "x86_64")]
    {
        if paging::using_builtin_page_tables() {
            unsafe {
                crate::arch::x86_64::syscall::install_syscall_msrs_bringup();
            }
            log_line_hal(&hal, SUB_SYSC, b"syscall MSRs STAR/LSTAR/FMASK");

            let mut proc = crate::ps::process::EProcess::new_bootstrap();
            let _ = crate::mm::bringup_user::register_bringup_vad(&mut proc.vad_root);
            crate::mm::page_fault::bind_page_fault_to_process_vad(&proc);
            proc.peb = crate::ps::peb::PebRef::bringup_smoke();

            let stub = crate::ke::sched::ThreadStub::new(8);
            let tid = stub.id;
            let _ = crate::ke::sched::rr_register_thread(stub);
            let _ethread =
                crate::ps::thread::EThread::new_system_thread(proc.pid, tid);
            let _proc_keepalive = &proc;
            let _ethread_keepalive = &_ethread;

            unsafe {
                crate::mm::bringup_user::copy_user_smoke_code(
                    crate::mm::user_va::USER_BRINGUP_VA as *mut u8,
                );
            }
            log_line_hal(&hal, SUB_SYSC, b"entering ring3 syscall smoke");
            let rsp = crate::mm::user_va::USER_BRINGUP_STACK_TOP - 16;
            let rip = crate::mm::bringup_user::user_code_entry_va();
            unsafe {
                crate::arch::x86_64::user_enter::zircon_enter_ring3(rip, rsp);
            }
        } else if uefi_high_half_ok && crate::mm::phys::pfn_pool_initialized() {
            let existing = crate::servers::smss::ring3_placeholder_cr3_phys();
            let new_cr3_opt = if existing != 0 {
                log_line_hal(&hal, SUB_SYSC, b"UEFI ring3 reusing SMSS placeholder CR3");
                Some(existing)
            } else if let Some(c) = unsafe { crate::mm::uefi_user_cr3::build_uefi_first_user_cr3() } {
                let _ = crate::servers::smss::try_set_ring3_placeholder_cr3(c);
                log_line_hal(
                    &hal,
                    SUB_SYSC,
                    b"UEFI first-user CR3 published to SMSS placeholder slot",
                );
                Some(c)
            } else {
                log_line_hal(&hal, SUB_SYSC, b"UEFI first-user CR3 clone failed");
                None
            };
            if let Some(new_cr3) = new_cr3_opt {
                let mut proc = crate::ps::process::EProcess::new_bootstrap();
                proc.cr3_phys = new_cr3;
                proc.peb = crate::ps::peb::PebRef::bringup_smoke();
                if crate::mm::bringup_user::install_uefi_user_bringup_vads(&mut proc.vad_root).is_ok() {
                    crate::mm::page_fault::bind_page_fault_to_process_vad(&proc);
                    let code = crate::mm::bringup_user::USER_RING3_UEFI_PROBE_SYSCALL;
                    log_line_hal(
                        &hal,
                        SUB_SYSC,
                        b"UEFI iretq + syscall probe (ZR_UEFI_R3_PROBE)",
                    );
                    let map_ok = unsafe {
                        crate::mm::uefi_user_cr3::map_uefi_bringup_user_code_and_stack(
                            new_cr3,
                            code.as_ptr(),
                            code.len(),
                        )
                    };
                    if map_ok.is_ok() {
                        unsafe {
                            paging::write_cr3(new_cr3);
                        }
                        hal.flush_tlb_all();
                        unsafe {
                            crate::arch::x86_64::syscall::install_syscall_msrs_bringup();
                        }
                        log_line_hal(&hal, SUB_SYSC, b"syscall MSRs STAR/LSTAR/FMASK (UEFI)");
                        let stub = crate::ke::sched::ThreadStub::new(8);
                        let tid = stub.id;
                        let _ = crate::ke::sched::rr_register_thread(stub);
                        let _ethread =
                            crate::ps::thread::EThread::new_system_thread(proc.pid, tid);
                        let _proc_keepalive = &proc;
                        let _ethread_keepalive = &_ethread;
                        log_line_hal(
                            &hal,
                            SUB_SYSC,
                            b"UEFI user thread starting (ring3 + demand stack)",
                        );
                        let rsp = crate::mm::user_va::USER_BRINGUP_STACK_TOP - 16;
                        let rip = crate::mm::bringup_user::user_code_entry_va();
                        unsafe {
                            crate::arch::x86_64::user_enter::zircon_enter_ring3(rip, rsp);
                        }
                    } else {
                        log_line_hal(&hal, SUB_SYSC, b"UEFI user map failed");
                    }
                } else {
                    log_line_hal(&hal, SUB_SYSC, b"UEFI user VAD install failed");
                }
            }
        } else {
            log_line_hal(&hal, SUB_SYSC, b"skip ring3 smoke (not built-in CR3)");
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Poll session draws software cursor + input; runs whenever GOP was handed off. The built-in
        // page-table path jumps to ring3 above and does not return here.
        if let Some(fb) = uefi_desktop_poll_fb {
            log_line_hal(&hal, SUB_SESS, b"entering UEFI desktop poll session");
            crate::desktop::fluent::session::run_uefi_desktop_poll_session(&hal, fb);
        }
    }

    log_line_hal(&hal, SUB_BOOT, b"entering idle (hlt)");
    loop {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
