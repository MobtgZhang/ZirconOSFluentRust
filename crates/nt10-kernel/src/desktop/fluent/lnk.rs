//! Windows `.lnk` (shell link) — binary layout not parsed yet; hook for loader or user-mode library.

/// Placeholder result once LINKHEADER + FileLocationTable are implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LnkTargetHint {
    pub is_unicode_path: bool,
}

/// Returns `Err` until PE/resource integration lands (`extensions/phase-07-shell-environment.md`).
pub fn parse_shell_link_stub(_image: &[u8]) -> Result<LnkTargetHint, ()> {
    Err(())
}
