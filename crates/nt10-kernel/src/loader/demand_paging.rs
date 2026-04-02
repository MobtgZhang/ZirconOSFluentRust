//! Demand paging — delegates fault handling to [`crate::mm::page_fault`].

/// Best-effort demand fault (e.g. loader paths); uses current [`crate::mm::page_fault`] VAD binding.
#[must_use]
pub fn demand_fault_stub(fault_va: u64) -> bool {
    crate::mm::page_fault::try_dispatch_page_fault(fault_va, 0) != 0
}
