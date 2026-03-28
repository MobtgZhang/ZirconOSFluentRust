//! IRQ / exception entry stubs (x86_64).

use core::arch::global_asm;

global_asm!(
    ".globl timer_irq_entry",
    ".align 16",
    "timer_irq_entry:",
    "sub rsp, 8",
    "push rax",
    "push rcx",
    "push rdx",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "call {rust}",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "add rsp, 8",
    "iretq",
    rust = sym nt10_irq0_rust,
);

#[unsafe(no_mangle)]
extern "C" fn nt10_irq0_rust() {
    let prev = unsafe { crate::ke::irql::raise(crate::ke::irql::CLOCK_LEVEL) };
    crate::ke::clock::tick();
    crate::ke::sched::on_timer_tick();
    unsafe {
        crate::ke::dpc::bsp_drain_pending();
        let apic = crate::hal::x86_64::apic::cached_mmio_phys();
        if apic != 0 {
            crate::hal::x86_64::apic::send_eoi(apic);
        } else {
            crate::hal::x86_64::pic::eoi_master();
        }
    }
    crate::ke::irql::lower(prev);
}

extern "C" {
    fn timer_irq_entry();
}

/// Address of the IRQ0 stub (vector 32 after PIC remap) for [`crate::arch::x86_64::idt`].
#[must_use]
pub fn timer_irq_entry_addr() -> usize {
    timer_irq_entry as *const () as usize
}
