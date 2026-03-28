//! COM1 (0x3F8) polled UART — early debug (QEMU `-serial stdio`).

const COM1: u16 = 0x3F8;

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack));
    v
}

#[inline]
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

/// 8n1, 38400 baud (divisor 3 @ ~115200 base — common QEMU default).
pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x80);
        outb(COM1, 0x03);
        outb(COM1 + 1, 0x00);
        outb(COM1 + 3, 0x03);
        outb(COM1 + 2, 0xc7);
        outb(COM1 + 4, 0x0b);
    }
}

#[inline]
pub fn write_byte(b: u8) {
    unsafe {
        while (inb(COM1 + 5) & 0x20) == 0 {}
        outb(COM1, b);
    }
}

pub fn write_bytes(s: &[u8]) {
    for &b in s {
        write_byte(b);
    }
}

pub fn write_line(s: &[u8]) {
    write_bytes(s);
    write_byte(b'\r');
    write_byte(b'\n');
}
