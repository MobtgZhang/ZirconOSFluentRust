//! DirectComposition ↔ kernel `dxgkrnl` boundary — ALPC-shaped message ids (ZirconOS-local).
//!
//! Real compositor trees would marshal surface handles across this port; bring-up only fixes opcodes.

/// Client → compositor service (namespaced to avoid csrss collisions).
pub const DCOMP_CREATE_VISUAL: u32 = 0x3001;
pub const DCOMP_SET_TRANSFORM: u32 = 0x3002;
pub const DCOMP_COMMIT: u32 = 0x3003;
pub const DCOMP_CREATE_SWAPCHAIN_FOR_HWND: u32 = 0x3004;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DcompMsgHeader {
    pub opcode: u32,
    pub hwnd_lo: u32,
    pub hwnd_hi: u32,
}

impl DcompMsgHeader {
    #[must_use]
    pub const fn new(op: u32, hwnd: u64) -> Self {
        Self {
            opcode: op,
            hwnd_lo: hwnd as u32,
            hwnd_hi: (hwnd >> 32) as u32,
        }
    }
}
