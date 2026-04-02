//! Access tokens (DAC + MIC hooks).

use crate::se::acl::{access_check_integrity_write, access_check_sid_equal, AccessCheckResult};
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

    /// DAC-style check: subject [`Sid`] must match the resource owner.
    #[must_use]
    pub fn access_check_vs_owner(&self, resource_owner: &Sid) -> AccessCheckResult {
        access_check_sid_equal(&self.user, resource_owner)
    }

    /// MIC-style write check against a resource’s minimum integrity floor.
    #[must_use]
    pub fn write_integrity_check(&self, object_min_integrity: u8) -> AccessCheckResult {
        access_check_integrity_write(self.integrity_level, object_min_integrity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::se::integrity::SECURITY_MANDATORY_HIGH_RID;

    #[test]
    fn token_owner_check_delegates_to_acl() {
        let t = SecurityToken::system_bootstrap();
        let world = Sid::well_known_world();
        assert_eq!(t.access_check_vs_owner(&world), AccessCheckResult::Granted);
    }

    #[test]
    fn token_integrity_write_delegates_to_acl() {
        let t = SecurityToken::medium_integrity_user();
        assert_eq!(
            t.write_integrity_check(SECURITY_MANDATORY_HIGH_RID),
            AccessCheckResult::Denied
        );
    }
}
