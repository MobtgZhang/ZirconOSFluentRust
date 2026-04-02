//! NUMA — node-aware allocation hooks (single-node bring-up).

/// Preferred node for a new physical frame (`0` until ACPI/SRAT lands).
pub trait NumaPolicy {
    #[must_use]
    fn preferred_node(&self) -> u32 {
        0
    }
}

/// Default: one logical node.
#[derive(Clone, Copy, Debug, Default)]
pub struct SingleNode;

impl NumaPolicy for SingleNode {}
