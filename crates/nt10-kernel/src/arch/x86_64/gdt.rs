//! Minimal flat 64-bit ring-0 GDT.

use core::arch::asm;

#[repr(C, packed)]
struct GdtDesc {
    limit: u16,
    base: u64,
}

#[repr(C, align(16))]
struct Gdt([u64; 5]);

/// Current `CS` selector (firmware or our GDT). Use for IDT gates when not reloading GDT under UEFI.
#[inline]
pub fn read_cs() -> u16 {
    let cs: u16;
    unsafe {
        core::arch::asm!("mov {0:x}, cs", out(reg) cs, options(nomem, nostack));
    }
    cs
}

static GDT: Gdt = Gdt([
    0,
    0x00af9a000000ffff, // 64-bit code, ring 0 (0x08)
    0x00af92000000ffff, // 64-bit data, ring 0 (0x10)
    0x00cff2000000ffff, // 64-bit data, ring 3 (0x18)
    0x00affa000000ffff, // 64-bit code, ring 3 (0x20)
]);

#[inline(never)]
extern "C" fn after_reload() {}

pub fn install() {
    let base = core::ptr::addr_of!(GDT) as u64;
    let limit = (core::mem::size_of_val(&GDT) - 1) as u16;
    let desc = GdtDesc { limit, base };
    let ret = after_reload as *const () as usize;

    unsafe {
        asm!(
            "lgdt [{}]",
            "mov ax, 0x10",
            "mov ds, ax",
            "mov es, ax",
            "mov ss, ax",
            "mov fs, ax",
            "mov gs, ax",
            "push 0x08",
            "push {ret}",
            "retfq",
            in(reg) &desc,
            ret = in(reg) ret,
            out("ax") _,
            options(nostack),
        );
    }
}
