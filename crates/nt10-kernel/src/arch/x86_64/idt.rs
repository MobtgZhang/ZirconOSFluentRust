//! 256-entry IDT; all vectors share a halt stub until per-vector handlers land.

use core::arch::{asm, global_asm};

#[repr(C, packed)]
struct IdtDesc {
    limit: u16,
    base: u64,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    _zero: u32,
}

const IDT_ENTRIES: usize = 256;
#[repr(C, align(16))]
struct Idt([IdtEntry; IDT_ENTRIES]);

static mut IDT: Idt = Idt([IdtEntry {
    offset_low: 0,
    selector: 0,
    ist: 0,
    type_attr: 0,
    offset_mid: 0,
    offset_high: 0,
    _zero: 0,
}; IDT_ENTRIES]);

global_asm!(
    ".section .text",
    ".globl idt_default_stub",
    "idt_default_stub:",
    "cli",
    "1:",
    "hlt",
    "jmp 1b",
);

unsafe extern "C" {
    fn idt_default_stub();
}

/// # Safety
/// Call once from BSP with interrupts cleared; overwrites `IDT`.
pub unsafe fn init() {
    let handler = idt_default_stub as *const () as usize;
    let sel = crate::arch::x86_64::gdt::read_cs();
    let attr = 0x8Eu8; // interrupt gate, present, ring 0

    let idt = &mut *core::ptr::addr_of_mut!(IDT);
    for i in 0..IDT_ENTRIES {
        idt.0[i] = IdtEntry {
            offset_low: handler as u16,
            selector: sel,
            ist: 0,
            type_attr: attr,
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            _zero: 0,
        };
    }

    let base = core::ptr::addr_of_mut!(IDT) as u64;
    let limit = (core::mem::size_of::<Idt>() - 1) as u16;
    let desc = IdtDesc { limit, base };
    asm!("lidt [{}]", in(reg) &desc, options(readonly, nostack));
}

/// Install a 64-bit interrupt gate for `vector` (0..256).
///
/// # Safety
/// `handler` must be a valid interrupt entry point (`iretq` return path).
pub unsafe fn set_interrupt_gate(vector: usize, handler: usize) {
    if vector >= IDT_ENTRIES {
        return;
    }
    let sel = crate::arch::x86_64::gdt::read_cs();
    let attr = 0x8Eu8;
    let idt = &mut *core::ptr::addr_of_mut!(IDT);
    idt.0[vector] = IdtEntry {
        offset_low: handler as u16,
        selector: sel,
        ist: 0,
        type_attr: attr,
        offset_mid: (handler >> 16) as u16,
        offset_high: (handler >> 32) as u32,
        _zero: 0,
    };
}
