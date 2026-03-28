//! Access tokens (DAC + MIC hooks).

use crate::se::integrity::SECURITY_MANDATORY_MEDIUM_RID;
use crate::se::integrity::SECURITY_MANDATORY_SYSTEM_RID;
use crate::se::sid::Sid;

#[derive(Clone, Copy, Debug)]
pub struct SecurityToken {
    pub id: u64,
    pub user: Sid,
    pub integrity_level: u8,
    /// Terminal Services session id (`0` = console / single-session bring-up).
    pub session_id: u32,
}

impl SecurityToken {
    #[must_use]
    pub fn system_bootstrap() -> Self {
        Self {
            id: 0x1000,
            user: Sid::well_known_world(),
            integrity_level: SECURITY_MANDATORY_SYSTEM_RID,
            session_id: 0,
        }
    }

    #[must_use]
    pub fn medium_integrity_user() -> Self {
        Self {
            id: 0x2000,
            user: Sid::well_known_world(),
            integrity_level: SECURITY_MANDATORY_MEDIUM_RID,
            session_id: 0,
        }
    }
}
