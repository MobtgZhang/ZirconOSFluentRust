//! Demand paging — delegates fault handling to [`crate::mm::page_fault`].
//!
//! Order inside [`crate::mm::page_fault::try_dispatch_page_fault`]: **file-backed** mapped VADs
//! (see [`crate::mm::section::SectionBacking::FileBackedStub`]) then demand-zero for anonymous
//! committed regions.
//!
//! Large PE hint (not yet wired to skip [`crate::loader::pe_load::map_pe_image_sections_bringup`]):
//! [`crate::loader::pe_load::pe_lazy_map_recommended`] and [`crate::loader::pe_load::PE_LAZY_MAP_THRESHOLD_PAGES`].

/// Best-effort demand fault (e.g. loader paths); uses current [`crate::mm::page_fault`] VAD binding.
/// `error_code` should match CPU `#PF` bits (U/W/P) when known; `0` implies **supervisor** read on not-present.
#[must_use]
pub fn demand_fault_with_code(fault_va: u64, error_code: u64) -> bool {
    crate::mm::page_fault::try_dispatch_page_fault(fault_va, error_code) != 0
}

/// Same as [`demand_fault_with_code`] with `error_code == 0` (legacy smoke).
#[must_use]
pub fn demand_fault_stub(fault_va: u64) -> bool {
    demand_fault_with_code(fault_va, 0)
}
