//! Intel 8042 PS/2 controller (keyboard + auxiliary mouse) — **polling** only (no IRQ on UEFI path).

#[cfg(target_arch = "x86_64")]
use core::arch::asm;

pub const DATA_PORT: u16 = 0x60;
pub const CMD_STATUS_PORT: u16 = 0x64;

pub const STATUS_OBF: u8 = 1;
pub const STATUS_AUX: u8 = 0x20;

#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    unsafe {
        asm!("in al, dx", out("al") v, in("dx") port, options(nomem, nostack, preserves_flags));
    }
    v
}

#[inline]
#[cfg(target_arch = "x86_64")]
pub unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn inb(_port: u16) -> u8 {
    0
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn outb(_port: u16, _val: u8) {}

#[inline]
pub unsafe fn read_status() -> u8 {
    unsafe { inb(CMD_STATUS_PORT) }
}

/// Wait until controller input buffer is empty (bit 1 clear).
pub unsafe fn wait_input_empty() {
    for _ in 0..100_000 {
        if read_status() & 2 == 0 {
            return;
        }
        super::ps2::spin_short();
    }
}

/// Write command byte to **command** port (0x64).
pub unsafe fn write_cmd(cmd: u8) {
    wait_input_empty();
    unsafe {
        outb(CMD_STATUS_PORT, cmd);
    }
}

/// Write data to **data** port (0x60) after a command that expects data.
pub unsafe fn write_data(b: u8) {
    wait_input_empty();
    unsafe {
        outb(DATA_PORT, b);
    }
}

/// Read data from data port (caller must ensure OBF).
pub unsafe fn read_data() -> u8 {
    unsafe { inb(DATA_PORT) }
}

/// Try read keyboard or mouse byte; returns `Some((from_aux, data))` if OBF set.
pub unsafe fn try_read() -> Option<(bool, u8)> {
    let st = read_status();
    if st & STATUS_OBF == 0 {
        return None;
    }
    let aux = (st & STATUS_AUX) != 0;
    let d = read_data();
    Some((aux, d))
}

/// Minimal bring-up: enable **second PS/2 port** (mouse) if present.
pub unsafe fn init_ps2_ports_poll() {
    // Enable second port
    write_cmd(0xA8);
    // Read "byte 0" configuration
    write_cmd(0x20);
    super::ps2::spin_short();
    let mut cfg = 0u8;
    for _ in 0..256 {
        if let Some((false, b)) = try_read() {
            cfg = b;
            break;
        }
        super::ps2::spin_short();
    }
    // Enable IRQ bits not required for polling; enable auxiliary clock
    cfg |= 2u8;
    write_cmd(0x60);
    write_data(cfg);
}
