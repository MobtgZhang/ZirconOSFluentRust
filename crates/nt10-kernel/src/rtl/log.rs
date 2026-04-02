//! Serial debug lines: fixed `[ZFOS]` product prefix + short subsystem tag (no crate name in output).

use crate::hal::Hal;

#[cfg(target_arch = "x86_64")]
use crate::hal::x86_64::serial;

/// Product prefix for all kernel serial logs (ZirconOSFluent).
pub const PREFIX: &[u8] = b"[ZFOS]";

pub const SUB_BOOT: &[u8] = b"[BOOT]";
pub const SUB_MM: &[u8] = b"[MM]";
pub const SUB_KE: &[u8] = b"[KE]";
pub const SUB_OB: &[u8] = b"[OB]";
pub const SUB_SYSC: &[u8] = b"[SYSC]";
pub const SUB_VID: &[u8] = b"[VID]";
pub const SUB_SUBS: &[u8] = b"[SUBS]";
pub const SUB_SESS: &[u8] = b"[SESS]";
pub const SUB_INPT: &[u8] = b"[INPT]";

/// `[ZFOS][sub] message\r\n` via [`Hal::debug_write`].
pub fn log_line_hal<H: Hal + ?Sized>(hal: &H, sub: &[u8], msg: &[u8]) {
    hal.debug_write(PREFIX);
    hal.debug_write(sub);
    hal.debug_write(b" ");
    hal.debug_write(msg);
    hal.debug_write(b"\r\n");
}

/// Same layout as [`log_line_hal`], direct COM1 (`serial`). x86_64 only; no-op on other targets.
#[cfg(target_arch = "x86_64")]
pub fn log_line_serial(sub: &[u8], msg: &[u8]) {
    serial::write_bytes(PREFIX);
    serial::write_bytes(sub);
    serial::write_byte(b' ');
    serial::write_bytes(msg);
    serial::write_byte(b'\r');
    serial::write_byte(b'\n');
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
pub fn log_line_serial(_sub: &[u8], _msg: &[u8]) {}

fn write_decimal_u64_hal<H: Hal + ?Sized>(hal: &H, mut v: u64) {
    let mut buf = [0u8; 24];
    let mut i = buf.len();
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
}

/// `[ZFOS][sub] label` + decimal + `\r\n`.
pub fn log_u64_hal<H: Hal + ?Sized>(hal: &H, sub: &[u8], label: &[u8], n: u64) {
    hal.debug_write(PREFIX);
    hal.debug_write(sub);
    hal.debug_write(b" ");
    hal.debug_write(label);
    write_decimal_u64_hal(hal, n);
    hal.debug_write(b"\r\n");
}

/// [`log_u64_hal`] for `usize`.
pub fn log_usize_hal<H: Hal + ?Sized>(hal: &H, sub: &[u8], label: &[u8], n: usize) {
    log_u64_hal(hal, sub, label, n as u64);
}

/// Begin a multi-field line: `[ZFOS][sub] ` — append with [`write_u32_dec_hal`] / [`write_usize_dec_hal`], then [`log_endline_hal`].
pub fn log_compound_begin_hal<H: Hal + ?Sized>(hal: &H, sub: &[u8]) {
    hal.debug_write(PREFIX);
    hal.debug_write(sub);
    hal.debug_write(b" ");
}

/// End a line started with [`log_compound_begin_hal`].
pub fn log_endline_hal<H: Hal + ?Sized>(hal: &H) {
    hal.debug_write(b"\r\n");
}

/// Decimal `u32` for compound lines (no prefix).
pub fn write_u32_dec_hal<H: Hal + ?Sized>(hal: &H, mut n: u32) {
    let mut buf = [0u8; 12];
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while n > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
}

/// Decimal `usize` for compound lines (no prefix).
pub fn write_usize_dec_hal<H: Hal + ?Sized>(hal: &H, mut n: usize) {
    let mut buf = [0u8; 24];
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while n > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
}

#[cfg(target_arch = "x86_64")]
fn write_decimal_u64_serial(mut v: u64) {
    let mut buf = [0u8; 24];
    let mut i = buf.len();
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
    serial::write_bytes(&buf[i..]);
}

/// `[ZFOS][sub] label` + decimal + `\r\n` on serial only (x86_64).
#[cfg(target_arch = "x86_64")]
pub fn log_u64_serial(sub: &[u8], label: &[u8], n: u64) {
    serial::write_bytes(PREFIX);
    serial::write_bytes(sub);
    serial::write_byte(b' ');
    serial::write_bytes(label);
    write_decimal_u64_serial(n);
    serial::write_byte(b'\r');
    serial::write_byte(b'\n');
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
pub fn log_u64_serial(_sub: &[u8], _label: &[u8], _n: u64) {}

/// [`log_u64_serial`] for `usize`.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn log_usize_serial(sub: &[u8], label: &[u8], n: usize) {
    log_u64_serial(sub, label, n as u64);
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
pub fn log_usize_serial(_sub: &[u8], _label: &[u8], _n: usize) {}
