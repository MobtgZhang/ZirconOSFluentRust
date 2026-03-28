//! ALPC message envelope (layout TBD; keep sized for tests).

/// Fixed small buffer for early IPC experiments.
pub const ALPC_INLINE_BYTES: usize = 128;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AlpcInlineMessage {
    pub len: u32,
    pub _pad: u32,
    pub data: [u8; ALPC_INLINE_BYTES],
}

impl AlpcInlineMessage {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            _pad: 0,
            data: [0; ALPC_INLINE_BYTES],
        }
    }
}
