//! Control-flow Enforcement Technology (CET) — feature probing only.
//!
//! Real `CPUID` / MSR reads on `x86_64-unknown-none` stay in assembly stubs; this module holds
//! structured results for logging and future policy.

/// CPU / MSR capability snapshot (all `false` until real probes exist).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CetProbeResult {
    pub shadow_stack_supported: bool,
    pub indirect_branch_tracking_supported: bool,
    pub shadow_stack_enabled_in_hw: bool,
}

impl CetProbeResult {
    pub const fn unavailable() -> Self {
        Self {
            shadow_stack_supported: false,
            indirect_branch_tracking_supported: false,
            shadow_stack_enabled_in_hw: false,
        }
    }
}

#[must_use]
pub fn probe_cet_stub() -> CetProbeResult {
    CetProbeResult::unavailable()
}

#[must_use]
pub fn shadow_stack_supported() -> bool {
    probe_cet_stub().shadow_stack_supported
}
