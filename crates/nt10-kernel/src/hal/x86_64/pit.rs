//! 8254 PIT channel 0 — system timer (legacy PC; HPET/APIC timer later).

use core::arch::asm;

unsafe fn outb(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

const CH0_DATA: u16 = 0x40;
const CMD: u16 = 0x43;

/// Channel 0, lobyte/hibyte, square wave (mode 3), binary.
const CMD_CH0_BOTH: u8 = 0x36;

/// Program channel 0 in mode 3 with the given **divisor** (1..65535).
/// Interrupt rate ≈ 1193182 / divisor Hz.
///
/// # Safety
/// PIC must be initialized and vector for IRQ0 installed before `sti`.
pub unsafe fn init_channel0_periodic(divisor: u16) {
    let div = divisor.max(1);
    outb(CMD, CMD_CH0_BOTH);
    outb(CH0_DATA, (div & 0xFF) as u8);
    outb(CH0_DATA, ((div >> 8) & 0xFF) as u8);
}
