//! Cross-session ALPC rules — Session 0 vs interactive session isolation (design hooks).
//!
//! Windows blocks interactive users from opening service ports in session 0 without ACLs.
//! ZirconOS bring-up: reject `client_session_id != 0` on ports marked `PORT_FLAGS_SYSTEM_ONLY`
//! when that check is wired into the connection path.

/// Port created only for session 0 / system callers (SCM, LSASS-class).
pub const PORT_FLAGS_SYSTEM_ONLY: u32 = 0x0000_0001;
/// Port accepts any session (use sparingly; default deny for cross-session).
pub const PORT_FLAGS_ANY_SESSION: u32 = 0x0000_0002;

/// Returns `false` when `port_flags` requires system session and `client_session != 0`.
#[must_use]
pub fn alpc_allow_connection(port_flags: u32, client_session_id: u32) -> bool {
    if port_flags & PORT_FLAGS_SYSTEM_ONLY != 0 && client_session_id != 0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_session0_only_port_from_interactive() {
        assert!(!alpc_allow_connection(PORT_FLAGS_SYSTEM_ONLY, 1));
        assert!(alpc_allow_connection(PORT_FLAGS_SYSTEM_ONLY, 0));
        assert!(alpc_allow_connection(PORT_FLAGS_ANY_SESSION, 1));
    }
}
