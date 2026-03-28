//! Access control lists (ACL) and access checks — bring-up allows all until [`Token`]/`SID` wiring lands.

use super::sid::Sid;

/// Result of an access check against an ACL + security descriptor (simplified).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessCheckResult {
    Granted,
    Denied,
}

/// Temporary policy: permit every request so I/O and OB paths can be exercised without a full SE graph.
#[must_use]
pub const fn access_check_bringup_allow_all() -> AccessCheckResult {
    AccessCheckResult::Granted
}

/// Non-trivial check: grant only when subject SID equals the resource owner SID.
#[must_use]
pub fn access_check_sid_equal(subject: &Sid, resource_owner: &Sid) -> AccessCheckResult {
    if subject == resource_owner {
        AccessCheckResult::Granted
    } else {
        AccessCheckResult::Denied
    }
}

/// Write check: deny when the writer MIC is **below** the object's minimum integrity floor.
#[must_use]
pub fn access_check_integrity_write(writer_level: u8, object_min_integrity: u8) -> AccessCheckResult {
    if writer_level < object_min_integrity {
        AccessCheckResult::Denied
    } else {
        AccessCheckResult::Granted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::se::integrity::SECURITY_MANDATORY_HIGH_RID;
    use crate::se::integrity::SECURITY_MANDATORY_LOW_RID;

    #[test]
    fn low_writer_denied_on_high_floor() {
        assert_eq!(
            access_check_integrity_write(SECURITY_MANDATORY_LOW_RID, SECURITY_MANDATORY_HIGH_RID),
            AccessCheckResult::Denied
        );
    }

    #[test]
    fn high_writer_granted_on_high_floor() {
        assert_eq!(
            access_check_integrity_write(SECURITY_MANDATORY_HIGH_RID, SECURITY_MANDATORY_HIGH_RID),
            AccessCheckResult::Granted
        );
    }
}
