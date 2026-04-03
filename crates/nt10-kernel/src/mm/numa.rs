//! NUMA — node-aware allocation hooks (single-node bring-up).
//!
//! Multi-node ACPI/SRAT placement is **not** implemented; callers must not assume Windows-internal
//! node structures. See [MM-Goals-and-Invariants.md](../../../../docs/en/MM-Goals-and-Invariants.md).
//!
//! **Roadmap (post architecture-freeze):** SRAT parsing, per-node free lists, and integration with PFN /
//! buddy **after** bring-up MM invariants are stable. Production **lookaside / leak-grade pool** telemetry
//! belongs in a separate milestone once [`pool`](crate::mm::pool) tag stats are settled — avoid duplicating
//! two pool semantics.

/// Whether the kernel can place frames on more than one node (always false until SRAT parsing exists).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumaTopologyKind {
    SingleNode,
    UnsupportedMultiNode,
}

#[must_use]
pub const fn numa_topology_bringup() -> NumaTopologyKind {
    NumaTopologyKind::SingleNode
}

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
