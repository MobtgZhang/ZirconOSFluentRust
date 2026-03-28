//! USB HID boot protocol — map 8-byte keyboard / 3–4-byte mouse reports to [`super::input_mgr`] events.

use super::input_mgr::{KeyEvent, PointerEvent};
use super::vkey;

#[inline]
fn hid_usage_to_vkey(u: u8) -> u8 {
    match u {
        0xE3 | 0xE7 => vkey::VK_WIN,
        0x52 => vkey::VK_UP,
        0x51 => vkey::VK_DOWN,
        0x50 => vkey::VK_LEFT,
        0x4F => vkey::VK_RIGHT,
        0x28 => vkey::VK_ENTER,
        0x29 => vkey::VK_ESC,
        0x2B => vkey::VK_TAB,
        _ => u,
    }
}

/// Boot keyboard: modifiers `[0]`, keys `[2..8]`. Normalizes common HID usages to [`vkey`] ids.
pub fn boot_keyboard_fill(out: &mut [KeyEvent], report: &[u8]) -> usize {
    if report.len() < 8 {
        return 0;
    }
    let mut n = 0usize;
    for k in report[2..8].iter().copied() {
        if k != 0 && k != 0xE3 && k != 0xE7 {
            if n >= out.len() {
                break;
            }
            out[n] = KeyEvent {
                code: hid_usage_to_vkey(k),
                down: true,
            };
            n += 1;
        }
    }
    n
}

/// Boot mouse: 3 bytes `[buttons, dx, dy]` or 4 bytes with either a leading Report ID or a trailing wheel byte.
/// 5 bytes: `[report_id, buttons, dx, dy, wheel]` (boot protocol + wheel).
pub fn boot_mouse_report(report: &[u8]) -> Option<PointerEvent> {
    let (buttons, dx_b, dy_b) = match report.len() {
        3 => (report[0], report[1], report[2]),
        4 => {
            if report[0] != 0 && (report[1] & 0xF8) == 0 {
                (report[1], report[2], report[3])
            } else {
                (report[0], report[1], report[2])
            }
        }
        5 => (report[1], report[2], report[3]),
        _ => return None,
    };
    Some(PointerEvent {
        dx: dx_b as i8 as i16,
        dy: dy_b as i8 as i16,
        buttons,
    })
}

const QEMU_TABLET_XY_MAX: u32 = 0x7fff;

/// QEMU [`usb-tablet`](https://github.com/qemu/qemu/blob/master/hw/usb/dev-hid.c): absolute X/Y 0..0x7fff,
/// 6 bytes `buttons, x_lo, x_hi, y_lo, y_hi, wheel` or 7 with a leading report ID.
#[must_use]
pub fn qemu_usb_tablet_pointer(report: &[u8], screen_w: u32, screen_h: u32) -> Option<(u8, u32, u32)> {
    if screen_w == 0 || screen_h == 0 {
        return None;
    }
    let parse = |btn_i: usize, x_i: usize, y_i: usize, buf: &[u8]| -> Option<(u8, u32, u32)> {
        if buf.len() < y_i + 2 {
            return None;
        }
        let buttons = buf[btn_i] & 0x1f;
        let x_raw = u16::from_le_bytes([buf[x_i], buf[x_i + 1]]) as u32;
        let y_raw = u16::from_le_bytes([buf[y_i], buf[y_i + 1]]) as u32;
        if x_raw > QEMU_TABLET_XY_MAX || y_raw > QEMU_TABLET_XY_MAX {
            return None;
        }
        let px = (x_raw * screen_w.saturating_sub(1)) / QEMU_TABLET_XY_MAX;
        let py = (y_raw * screen_h.saturating_sub(1)) / QEMU_TABLET_XY_MAX;
        Some((
            buttons,
            px.min(screen_w.saturating_sub(1)),
            py.min(screen_h.saturating_sub(1)),
        ))
    };
    match report.len() {
        6 => parse(0, 1, 3, report),
        n if n >= 7 => parse(1, 2, 4, report).or_else(|| {
            report
                .get(..6)
                .and_then(|prefix| parse(0, 1, 3, prefix))
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qemu_tablet_maps_corners() {
        let r = [0u8, 0xff, 0x7f, 0xff, 0x7f, 0];
        let (b, px, py) = qemu_usb_tablet_pointer(&r, 100, 50).unwrap();
        assert_eq!(b, 0);
        assert_eq!(px, 99);
        assert_eq!(py, 49);
    }

    #[test]
    fn boot_mouse_five_byte_with_report_id() {
        let r = [1u8, 0, 5, -3i8 as u8, 0];
        let p = boot_mouse_report(&r).unwrap();
        assert_eq!(p.buttons, 0);
        assert_eq!(p.dx, 5);
        assert_eq!(p.dy, -3);
    }
}
