//! Hypervisor-related CPUID bits — bare-metal path stays stubbed until asm leaf helpers land.

/// Bits and leaves that would come from CPUID (ZirconOS-local aggregation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuidHypervisorBits {
    /// Intel/AMD manual: leaf 1 ECX bit 31 when a hypervisor is present.
    pub leaf1_ecx_hypervisor_present: bool,
    pub max_hypervisor_leaf: u32,
    pub vendor_id: [u8; 12],
}

impl Default for CpuidHypervisorBits {
    fn default() -> Self {
        Self::bare_metal_stub()
    }
}

impl CpuidHypervisorBits {
    #[must_use]
    pub const fn bare_metal_stub() -> Self {
        Self {
            leaf1_ecx_hypervisor_present: false,
            max_hypervisor_leaf: 0,
            vendor_id: [0u8; 12],
        }
    }

    #[must_use]
    pub const fn test_fake_zircon_hv() -> Self {
        Self {
            leaf1_ecx_hypervisor_present: true,
            max_hypervisor_leaf: 0x4000_0000,
            vendor_id: [
                b'Z', b'i', b'r', b'c', b'o', b'n', b'O', b'S', b'H', b'V', 0, 0,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_bits_mark_present() {
        let b = CpuidHypervisorBits::test_fake_zircon_hv();
        assert!(b.leaf1_ecx_hypervisor_present);
        assert_eq!(&b.vendor_id[..10], b"ZirconOSHV");
    }
}
