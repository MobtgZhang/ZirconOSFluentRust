//! PS/2 scan code set 1 (subset) + 3-byte mouse packet parsing.

use super::i8042;
use super::input_mgr::KeyEvent;
use super::vkey;

#[inline]
pub fn spin_short() {
    for _ in 0..64 {
        core::hint::spin_loop();
    }
}

/// Enable PS/2 auxiliary device streaming (expects `0xFA` ack, consumed by [`MouseStreamState`]).
pub unsafe fn enable_mouse_streaming() {
    i8042::write_cmd(0xD4);
    i8042::write_data(0xF4);
}

pub const SC1_ESC: u8 = 0x01;
pub const SC1_ENTER: u8 = 0x1C;

/// Decode one keyboard byte (no `0xE0` prefix handling).
pub fn keyboard_byte_to_key(sc: u8) -> Option<(u8, bool)> {
    if sc == 0xE0 || sc == 0xE1 {
        return None;
    }
    let release = (sc & 0x80) != 0;
    let code = sc & 0x7F;
    Some((code, !release))
}

/// Handles Set-1 `0xE0` extended prefixes (arrows, Win).
#[derive(Clone, Copy, Debug)]
pub struct Ps2ScanDecoder {
    prefix_e0: bool,
}

impl Ps2ScanDecoder {
    pub const fn new() -> Self {
        Self { prefix_e0: false }
    }

    pub fn feed(&mut self, raw: u8) -> Option<KeyEvent> {
        if raw == 0xE1 {
            self.prefix_e0 = false;
            return None;
        }
        if raw == 0xE0 {
            self.prefix_e0 = true;
            return None;
        }
        if self.prefix_e0 {
            self.prefix_e0 = false;
            let release = (raw & 0x80) != 0;
            let c = raw & 0x7F;
            let code = match c {
                0x5B | 0x5C => vkey::VK_WIN,
                0x48 => vkey::VK_UP,
                0x50 => vkey::VK_DOWN,
                0x4B => vkey::VK_LEFT,
                0x4D => vkey::VK_RIGHT,
                _ => return None,
            };
            return Some(KeyEvent {
                code,
                down: !release,
            });
        }
        let release = (raw & 0x80) != 0;
        let mut code = raw & 0x7F;
        if code == 0x0F {
            code = vkey::VK_TAB;
        }
        Some(KeyEvent {
            code,
            down: !release,
        })
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ps2MousePacket {
    pub buttons: u8,
    pub dx: i16,
    pub dy: i16,
}

pub fn parse_mouse_packet(b0: u8, b1: u8, b2: u8) -> Ps2MousePacket {
    let buttons = b0 & 0x07;
    let x_sign = (b0 & 0x10) != 0;
    let y_sign = (b0 & 0x20) != 0;
    let mut dx = b1 as i16;
    let mut dy = b2 as i16;
    if x_sign {
        dx -= 256;
    }
    if y_sign {
        dy -= 256;
    }
    Ps2MousePacket {
        buttons,
        dx,
        dy: -dy,
    }
}

pub struct MouseStreamState {
    pkt: [u8; 3],
    idx: u8,
}

impl MouseStreamState {
    pub const fn new() -> Self {
        Self {
            pkt: [0; 3],
            idx: 0,
        }
    }

    pub fn feed_aux_byte(&mut self, b: u8) -> Option<Ps2MousePacket> {
        if b == 0xFA {
            return None;
        }
        if self.idx == 0 && (b & 8) == 0 {
            return None;
        }
        self.pkt[self.idx as usize] = b;
        self.idx += 1;
        if self.idx >= 3 {
            self.idx = 0;
            return Some(parse_mouse_packet(self.pkt[0], self.pkt[1], self.pkt[2]));
        }
        None
    }
}
