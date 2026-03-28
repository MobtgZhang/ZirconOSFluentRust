//! Enlightenment capability bitmap — suggests preferred paravirtual behaviors (implementation may no-op).

/// Bit flags for TSC / APIC / other hints (ZirconOS-local numbering).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnlightenmentCaps(pub u32);

impl EnlightenmentCaps {
    pub const TSC_PARAVIRT_HINT: u32 = 1 << 0;
    pub const APIC_PARAVIRT_HINT: u32 = 1 << 1;

    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Until hypervisor detection is real, return an empty set.
    #[must_use]
    pub fn from_detect_stub() -> Self {
        Self::empty()
    }

    #[must_use]
    pub const fn contains(self, bit: u32) -> bool {
        (self.0 & bit) != 0
    }
}
