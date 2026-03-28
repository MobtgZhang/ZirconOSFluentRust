//! Pointer / high-DPI mapping for Win32-style messages (Fluent `session` alignment).

/// `WM_POINTER*` family — values follow public Win32 numbering (subset used for routing tables).
pub mod wm_pointer {
    pub const WM_POINTERENTER: u32 = 0x0249;
    pub const WM_POINTERLEAVE: u32 = 0x024A;
    pub const WM_POINTERDOWN: u32 = 0x0246;
    pub const WM_POINTERUP: u32 = 0x0247;
    pub const WM_POINTERUPDATE: u32 = 0x0245;
}

/// `POINTER_MESSAGE_FLAG` bits (bring-up subset).
pub mod pointer_flags {
    pub const POINTER_MESSAGE_FLAG_NEW: u32 = 0x0000_0001;
    pub const POINTER_MESSAGE_FLAG_INRANGE: u32 = 0x0000_0002;
    pub const POINTER_MESSAGE_FLAG_INCONTACT: u32 = 0x0000_0004;
    pub const POINTER_MESSAGE_FLAG_FIRSTBUTTON: u32 = 0x0000_0010;
    pub const POINTER_MESSAGE_FLAG_SECONDBUTTON: u32 = 0x0000_0020;
    pub const POINTER_MESSAGE_FLAG_THIRDBUTTON: u32 = 0x0000_0040;
}

/// Convert client DIP X to framebuffer X (`dpi` = logical pixels per inch, e.g. 96).
#[must_use]
pub fn dip_x_to_physical_px(dip_x: i32, dpi: u32, scale_percent: u32) -> i32 {
    if dpi == 0 {
        return dip_x;
    }
    let num = dip_x as i64 * dpi as i64 * scale_percent as i64;
    (num / (96 * 100)) as i32
}

/// Physical pixel Y from screen DIP (top-left origin, Y down — matches LearnWin32 mouse coords).
#[must_use]
pub fn dip_y_to_physical_px(dip_y: i32, dpi: u32, scale_percent: u32) -> i32 {
    dip_x_to_physical_px(dip_y, dpi, scale_percent)
}

/// Map USB-style button mask to `POINTER_MESSAGE_FLAG` bits (left=first, right=second).
#[must_use]
pub fn pointer_flags_from_buttons(left: bool, right: bool, _middle: bool) -> u32 {
    let mut f = pointer_flags::POINTER_MESSAGE_FLAG_INRANGE;
    if left {
        f |= pointer_flags::POINTER_MESSAGE_FLAG_FIRSTBUTTON | pointer_flags::POINTER_MESSAGE_FLAG_INCONTACT;
    }
    if right {
        f |= pointer_flags::POINTER_MESSAGE_FLAG_SECONDBUTTON;
    }
    f
}
