//! Win32 subsystem registration (CSRSS path).

/// Subsystem type recorded in the (future) image header / process creation path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubsystemKind {
    Native,
    Win32Cui,
    Win32Gui,
}

/// CSRSS connection token — opaque until ALPC wiring exists.
#[derive(Clone, Copy, Debug)]
pub struct CsrssSession {
    pub id: u32,
}

impl CsrssSession {
    #[must_use]
    pub fn bootstrap_stub() -> Self {
        Self { id: 1 }
    }
}
