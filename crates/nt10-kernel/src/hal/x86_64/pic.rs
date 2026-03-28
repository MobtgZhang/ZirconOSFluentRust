//! 8259 PIC — minimal remap and IRQ mask (BSP bring-up; APIC preferred later).

use core::arch::asm;

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    v
}

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

/// Remap master to vectors `0x20`..`0x27`, slave to `0x28`..`0x2F`; **mask all** IRQ lines.
///
/// # Safety
/// Call once from BSP before routing timer through LAPIC (avoids double timer IRQ).
pub unsafe fn remap_all_masked() {
    let a1 = inb(PIC1_DATA);
    let a2 = inb(PIC2_DATA);

    outb(PIC1_CMD, 0x11);
    outb(PIC2_CMD, 0x11);
    outb(PIC1_DATA, 0x20);
    outb(PIC2_DATA, 0x28);
    outb(PIC1_DATA, 4);
    outb(PIC2_DATA, 2);
    outb(PIC1_DATA, 0x01);
    outb(PIC2_DATA, 0x01);

    outb(PIC1_DATA, a1);
    outb(PIC2_DATA, a2);

    outb(PIC1_DATA, 0xFF);
    outb(PIC2_DATA, 0xFF);
}

/// Unmask IRQ0 on the master PIC (8259 PIT timer fallback).
///
/// # Safety
/// PIC must be remapped.
pub unsafe fn unmask_master_irq0() {
    outb(PIC1_DATA, 0xFE);
}

/// Remap + unmask IRQ0 (legacy PIT path).
///
/// # Safety
/// Call once from BSP before unmasking interrupts.
pub unsafe fn init() {
    remap_all_masked();
    unmask_master_irq0();
}

/// Non-specific EOI for the master PIC (after servicing IRQ0..7).
///
/// # Safety
/// Only from an IRQ0 handler path.
pub unsafe fn eoi_master() {
    outb(PIC1_CMD, 0x20);
}
