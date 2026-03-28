//! Win32 x64 handle and message types for FFI alignment (`HWND`/`WPARAM`/`LPARAM` are pointer-sized).

/// Window handle (opaque; never assume non-zero except after create).
pub type Hwnd = usize;
pub type Hinstance = usize;
pub type Hdc = usize;
pub type Hmenu = usize;
/// Message parameter types — 64-bit width on x64.
pub type WParam = u64;
pub type LParam = i64;
pub type LResult = i64;
