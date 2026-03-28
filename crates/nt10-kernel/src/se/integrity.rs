//! Mandatory Integrity Control (MIC) levels — symbolic constants.
//!
//! DEP / NX image hints are recorded separately in [`crate::mm::nx_image`] from PE optional headers.

pub const SECURITY_MANDATORY_UNTRUSTED_RID: u8 = 0;
pub const SECURITY_MANDATORY_LOW_RID: u8 = 1;
pub const SECURITY_MANDATORY_MEDIUM_RID: u8 = 2;
pub const SECURITY_MANDATORY_HIGH_RID: u8 = 3;
pub const SECURITY_MANDATORY_SYSTEM_RID: u8 = 4;
pub const SECURITY_MANDATORY_PROTECTED_PROCESS_RID: u8 = 5;
