//! Single-producer ring buffers for keyboard and pointer (BSP poll loop, no locks).
//!
//! Event shapes mirror bring-up needs; see also `references/win32/desktop-src/inputdev/keyboard-input.md`
//! and `references/win32/desktop-src/LearnWin32/mouse-movement.md` for Win32-side messaging analogues.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct KeyEvent {
    pub code: u8,
    pub down: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerEvent {
    pub dx: i16,
    pub dy: i16,
    pub buttons: u8,
}

const KEY_CAP: usize = 64;
const PTR_CAP: usize = 64;

pub struct InputManager {
    keys: [KeyEvent; KEY_CAP],
    k_head: usize,
    k_tail: usize,
    k_len: usize,
    ptrs: [PointerEvent; PTR_CAP],
    p_head: usize,
    p_tail: usize,
    p_len: usize,
}

impl InputManager {
    pub const fn new() -> Self {
        const ZK: KeyEvent = KeyEvent {
            code: 0,
            down: false,
        };
        const ZP: PointerEvent = PointerEvent {
            dx: 0,
            dy: 0,
            buttons: 0,
        };
        Self {
            keys: [ZK; KEY_CAP],
            k_head: 0,
            k_tail: 0,
            k_len: 0,
            ptrs: [ZP; PTR_CAP],
            p_head: 0,
            p_tail: 0,
            p_len: 0,
        }
    }

    pub fn push_key(&mut self, e: KeyEvent) {
        if self.k_len >= KEY_CAP {
            return;
        }
        self.keys[self.k_tail] = e;
        self.k_tail = (self.k_tail + 1) % KEY_CAP;
        self.k_len += 1;
    }

    pub fn push_pointer(&mut self, e: PointerEvent) {
        if self.p_len >= PTR_CAP {
            return;
        }
        self.ptrs[self.p_tail] = e;
        self.p_tail = (self.p_tail + 1) % PTR_CAP;
        self.p_len += 1;
    }

    pub fn pop_key(&mut self) -> Option<KeyEvent> {
        if self.k_len == 0 {
            return None;
        }
        let e = self.keys[self.k_head];
        self.k_head = (self.k_head + 1) % KEY_CAP;
        self.k_len -= 1;
        Some(e)
    }

    pub fn pop_pointer(&mut self) -> Option<PointerEvent> {
        if self.p_len == 0 {
            return None;
        }
        let e = self.ptrs[self.p_head];
        self.p_head = (self.p_head + 1) % PTR_CAP;
        self.p_len -= 1;
        Some(e)
    }
}
