//! Virtual key ids (PS/2 set-1 + USB HID normalized) for desktop shell.

pub const VK_ESC: u8 = 0x01;
pub const VK_ENTER: u8 = 0x1C;
/// USB HID usage 0x2B (PS/2 set-1 `0x0F` normalized in decoder).
pub const VK_TAB: u8 = 0x2B;
pub const VK_WIN: u8 = 0x5B;
pub const VK_UP: u8 = 0x80;
pub const VK_DOWN: u8 = 0x81;
pub const VK_LEFT: u8 = 0x82;
pub const VK_RIGHT: u8 = 0x83;
