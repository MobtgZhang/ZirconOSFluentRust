//! Virtual address space helpers on top of [`super::vad::VadTable`] (ZirconOSFluent naming).

use super::user_va;
use super::vad::{PageProtect, VadEntry, VadKind, VadTable};
use super::PAGE_SIZE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmError {
    BadRange,
    OutOfNodes,
    NotFound,
    InconsistentState,
}

#[must_use]
pub const fn align_down(va: u64, align: u64) -> u64 {
    if align == 0 {
        return va;
    }
    va / align * align
}

#[must_use]
pub const fn align_up(va: u64, align: u64) -> u64 {
    if align == 0 {
        return va;
    }
    (va + align - 1) / align * align
}

/// Reserve `[start, start+len)` as [`VadKind::Reserve`], not committed, page-aligned.
pub fn reserve_user_range(
    vad: &mut VadTable,
    start_va: u64,
    byte_len: u64,
    protect: PageProtect,
) -> Result<(), VmError> {
    if byte_len == 0 {
        return Err(VmError::BadRange);
    }
    let start = align_down(start_va, PAGE_SIZE);
    let end = align_up(start_va.saturating_add(byte_len), PAGE_SIZE);
    if !user_va::user_range_ok(start, end) {
        return Err(VmError::BadRange);
    }
    let e = VadEntry::new_range(start, end, VadKind::Reserve, protect, false);
    vad.insert(e).map_err(|_| VmError::OutOfNodes)
}

/// Turn an exact reserved `[start, end)` region into committed private memory.
pub fn commit_reserved_range(
    vad: &mut VadTable,
    start: u64,
    end: u64,
) -> Result<(), VmError> {
    let start = align_down(start, PAGE_SIZE);
    let end = align_up(end, PAGE_SIZE);
    if start >= end {
        return Err(VmError::BadRange);
    }
    let e = *vad.find_by_va(start).ok_or(VmError::NotFound)?;
    if e.start_va != start || e.end_va != end {
        return Err(VmError::BadRange);
    }
    if e.kind != VadKind::Reserve || e.committed {
        return Err(VmError::InconsistentState);
    }
    vad.remove(start).map_err(|_| VmError::NotFound)?;
    let ne = VadEntry {
        start_va: start,
        end_va: end,
        kind: VadKind::Private,
        protect: e.protect,
        committed: true,
        section: e.section,
    };
    vad.insert(ne).map_err(|_| VmError::OutOfNodes)
}

/// Replace protection on the VAD that **starts** at `start_va`.
pub fn protect_vad_at_start(
    vad: &mut VadTable,
    start_va: u64,
    protect: PageProtect,
) -> Result<(), VmError> {
    let e = *vad
        .find_by_va(start_va)
        .filter(|x| x.start_va == start_va)
        .ok_or(VmError::NotFound)?;
    vad.remove(start_va).map_err(|_| VmError::NotFound)?;
    let ne = VadEntry {
        protect,
        ..e
    };
    vad.insert(ne).map_err(|_| VmError::OutOfNodes)
}

/// Copy of the containing descriptor for `va`, if any.
#[must_use]
pub fn query_region(vad: &VadTable, va: u64) -> Option<VadEntry> {
    vad.find_by_va(va).copied()
}

/// Phase 6 bring-up: associates a user VAD tree with a future per-process CR3 (Ring-3 csrss/smss).
///
/// Today only [`Self::cr3`] is carried for documentation; install it when user threads gain a private
/// page table (see `docs/cn/Phase6-Routing.md`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessAddressSpaceBringup {
    pub cr3: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::vad::VadKind;

    #[test]
    fn reserve_commit_query() {
        let mut t = VadTable::new();
        assert!(reserve_user_range(&mut t, 0x200_0000, 0x4000, PageProtect::ReadWrite).is_ok());
        let q = query_region(&t, 0x200_1000).unwrap();
        assert_eq!(q.kind, VadKind::Reserve);
        assert!(!q.committed);
        assert!(commit_reserved_range(&mut t, 0x200_0000, 0x200_0000 + 0x4000).is_ok());
        let q2 = query_region(&t, 0x200_1000).unwrap();
        assert_eq!(q2.kind, VadKind::Private);
        assert!(q2.committed);
    }

    #[test]
    fn protect_vad_at_start_changes_entry() {
        let mut t = VadTable::new();
        reserve_user_range(&mut t, 0x300_0000, 0x1000, PageProtect::ReadOnly).unwrap();
        commit_reserved_range(&mut t, 0x300_0000, 0x300_1000).unwrap();
        protect_vad_at_start(&mut t, 0x300_0000, PageProtect::ReadWrite).unwrap();
        assert_eq!(
            query_region(&t, 0x300_0000).unwrap().protect,
            PageProtect::ReadWrite
        );
    }
}
