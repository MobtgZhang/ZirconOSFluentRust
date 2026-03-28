//! AppContainer-style sandbox profile (ZirconOS-local; not a Windows catalog clone).

use crate::se::acl::{access_check_integrity_write, AccessCheckResult};
use crate::se::integrity::SECURITY_MANDATORY_MEDIUM_RID;

/// Capability / package bitmask for bring-up (opaque bits).
#[derive(Clone, Copy, Debug)]
pub struct AppContainerProfile {
    pub capability_bits: u32,
    /// Minimum MIC required to write objects owned by this profile.
    pub resource_integrity_floor: u8,
}

/// Example packaged-app profile used in unit tests and serial diagnostics.
pub const APPCONTAINER_TEST_PROFILE: AppContainerProfile = AppContainerProfile {
    capability_bits: 1,
    resource_integrity_floor: SECURITY_MANDATORY_MEDIUM_RID,
};

#[must_use]
pub fn check_appcontainer_write(token_integrity: u8, profile: &AppContainerProfile) -> AccessCheckResult {
    access_check_integrity_write(token_integrity, profile.resource_integrity_floor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::se::integrity::SECURITY_MANDATORY_LOW_RID;

    #[test]
    fn low_token_denied_against_test_profile() {
        assert_eq!(
            check_appcontainer_write(SECURITY_MANDATORY_LOW_RID, &APPCONTAINER_TEST_PROFILE),
            AccessCheckResult::Denied
        );
    }
}
