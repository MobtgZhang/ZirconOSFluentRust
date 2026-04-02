//! Demand paging — page faults, prototype PTEs, and file-backed sections (stubs).

/// Placeholder page-fault handler entry until VFS + per-process PTE trees exist.
#[must_use]
pub fn demand_fault_stub(_fault_va: u64) -> bool {
    false
}
