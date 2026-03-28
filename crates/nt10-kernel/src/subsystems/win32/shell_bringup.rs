//! CMD / PowerShell-style hosts — bring-up entry points only (no real line editor).

/// Human-readable tag for logs and documentation.
pub const SHELL_BRINGUP_TAG: &[u8] = b"ZirconOS bring-up shell (echo/exit stubs only)\r\n";

/// Placeholder `cmd.exe`-style main: returns `0` until a user-mode host is linked.
#[must_use]
pub fn cmd_bringup_main_stub() -> u32 {
    0
}

/// Placeholder PowerShell host entry.
#[must_use]
pub fn powershell_bringup_main_stub() -> u32 {
    0
}
