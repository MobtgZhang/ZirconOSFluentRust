//! Virtual Address Descriptors — non-overlapping intervals in an array-backed AVL tree.

use core::ptr::NonNull;

const MAX_NODES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadKind {
    Private,
    Mapped,
    Reserve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageProtect {
    NoAccess,
    ReadOnly,
    ReadWrite,
    ExecuteRead,
    ExecuteReadWrite,
    WriteCopy,
}

#[derive(Clone, Copy, Debug)]
pub struct VadEntry {
    pub start_va: u64,
    pub end_va: u64,
    pub kind: VadKind,
    pub protect: PageProtect,
    pub committed: bool,
    pub section: Option<NonNull<()>>,
}

impl VadEntry {
    #[must_use]
    pub const fn new_range(
        start_va: u64,
        end_va: u64,
        kind: VadKind,
        protect: PageProtect,
        committed: bool,
    ) -> Self {
        Self {
            start_va,
            end_va,
            kind,
            protect,
            committed,
            section: None,
        }
    }
}

struct Node {
    key: VadEntry,
    left: i16,
    right: i16,
    height: i8,
    in_use: bool,
}

impl Node {
    const fn empty() -> Self {
        Self {
            key: VadEntry {
                start_va: 0,
                end_va: 0,
                kind: VadKind::Reserve,
                protect: PageProtect::NoAccess,
                committed: false,
                section: None,
            },
            left: -1,
            right: -1,
            height: 0,
            in_use: false,
        }
    }
}

/// Per-process VAD AVL (fixed pool).
pub struct VadTable {
    nodes: [Node; MAX_NODES],
    root: i16,
}

fn vad_entries_mergeable(a: &VadEntry, b: &VadEntry) -> bool {
    a.kind == b.kind
        && a.protect == b.protect
        && a.committed == b.committed
        && match (a.section, b.section) {
            (None, None) => true,
            (Some(x), Some(y)) => x.as_ptr() == y.as_ptr(),
            _ => false,
        }
}

impl VadTable {
    pub const fn new() -> Self {
        const E: Node = Node::empty();
        Self {
            nodes: [E; MAX_NODES],
            root: -1,
        }
    }

    #[must_use]
    pub fn range_overlaps_existing(&self, start: u64, end: u64) -> bool {
        if start >= end {
            return false;
        }
        for n in self.nodes.iter().filter(|n| n.in_use) {
            let k = &n.key;
            if start < k.end_va && end > k.start_va {
                return true;
            }
        }
        false
    }

    fn alloc_slot(&mut self) -> Option<i16> {
        for i in 0..MAX_NODES {
            if !self.nodes[i].in_use {
                self.nodes[i].in_use = true;
                self.nodes[i].left = -1;
                self.nodes[i].right = -1;
                self.nodes[i].height = 1;
                return Some(i as i16);
            }
        }
        None
    }

    fn height(&self, i: i16) -> i8 {
        if i < 0 || !self.nodes[i as usize].in_use {
            0
        } else {
            self.nodes[i as usize].height
        }
    }

    fn update_height(&mut self, i: i16) {
        if i < 0 {
            return;
        }
        let l = self.nodes[i as usize].left;
        let r = self.nodes[i as usize].right;
        self.nodes[i as usize].height = 1 + self.height(l).max(self.height(r));
    }

    fn balance_factor(&self, i: i16) -> i8 {
        if i < 0 {
            0
        } else {
            self.height(self.nodes[i as usize].left) - self.height(self.nodes[i as usize].right)
        }
    }

    fn rotate_right(&mut self, y: i16) -> i16 {
        let x = self.nodes[y as usize].left;
        let t2 = self.nodes[x as usize].right;
        self.nodes[x as usize].right = y;
        self.nodes[y as usize].left = t2;
        self.update_height(y);
        self.update_height(x);
        x
    }

    fn rotate_left(&mut self, x: i16) -> i16 {
        let y = self.nodes[x as usize].right;
        let t2 = self.nodes[y as usize].left;
        self.nodes[y as usize].left = x;
        self.nodes[x as usize].right = t2;
        self.update_height(x);
        self.update_height(y);
        y
    }

    fn insert_rec(&mut self, root: i16, key: VadEntry) -> Result<i16, ()> {
        if root < 0 {
            let i = self.alloc_slot().ok_or(())?;
            self.nodes[i as usize].key = key;
            return Ok(i);
        }
        let nk = self.nodes[root as usize].key.start_va;
        if key.start_va < nk {
            let l = self.nodes[root as usize].left;
            let nl = self.insert_rec(l, key)?;
            self.nodes[root as usize].left = nl;
        } else if key.start_va > nk {
            let r = self.nodes[root as usize].right;
            let nr = self.insert_rec(r, key)?;
            self.nodes[root as usize].right = nr;
        } else {
            return Err(());
        }
        self.update_height(root);
        let bf = self.balance_factor(root);
        if bf > 1 && key.start_va < self.nodes[self.nodes[root as usize].left as usize].key.start_va {
            return Ok(self.rotate_right(root));
        }
        if bf > 1 && key.start_va > self.nodes[self.nodes[root as usize].left as usize].key.start_va {
            let l = self.nodes[root as usize].left;
            let rotated = self.rotate_left(l);
            self.nodes[root as usize].left = rotated;
            return Ok(self.rotate_right(root));
        }
        if bf < -1 && key.start_va > self.nodes[self.nodes[root as usize].right as usize].key.start_va {
            return Ok(self.rotate_left(root));
        }
        if bf < -1 && key.start_va < self.nodes[self.nodes[root as usize].right as usize].key.start_va {
            let r = self.nodes[root as usize].right;
            let rotated = self.rotate_right(r);
            self.nodes[root as usize].right = rotated;
            return Ok(self.rotate_left(root));
        }
        Ok(root)
    }

    pub fn insert(&mut self, e: VadEntry) -> Result<(), ()> {
        if e.start_va >= e.end_va {
            return Err(());
        }
        if self.range_overlaps_existing(e.start_va, e.end_va) {
            return Err(());
        }
        self.root = self.insert_rec(self.root, e)?;
        Ok(())
    }

    /// Merge adjacent intervals that share kind, protection, commit state, and optional section.
    pub fn coalesce_adjacent_compatible(&mut self) -> Result<(), ()> {
        let mut buf = [VadEntry::new_range(
            0,
            1,
            VadKind::Reserve,
            PageProtect::NoAccess,
            false,
        ); MAX_NODES];
        let mut n = 0usize;
        for e in self.iter() {
            if n >= MAX_NODES {
                return Err(());
            }
            buf[n] = *e;
            n += 1;
        }
        for i in 1..n {
            let mut j = i;
            while j > 0 && buf[j - 1].start_va > buf[j].start_va {
                buf.swap(j - 1, j);
                j -= 1;
            }
        }
        let mut merged = [VadEntry::new_range(
            0,
            1,
            VadKind::Reserve,
            PageProtect::NoAccess,
            false,
        ); MAX_NODES];
        let mut m = 0usize;
        let mut i = 0usize;
        while i < n {
            let mut cur = buf[i];
            i += 1;
            while i < n {
                let next = buf[i];
                if cur.end_va == next.start_va && vad_entries_mergeable(&cur, &next) {
                    cur.end_va = next.end_va;
                    i += 1;
                } else {
                    break;
                }
            }
            if m >= MAX_NODES {
                return Err(());
            }
            merged[m] = cur;
            m += 1;
        }
        self.reset_tree();
        for j in 0..m {
            self.insert(merged[j])?;
        }
        Ok(())
    }

    #[must_use]
    pub fn find_by_va(&self, va: u64) -> Option<&VadEntry> {
        let mut cur = self.root;
        while cur >= 0 && self.nodes[cur as usize].in_use {
            let n = &self.nodes[cur as usize];
            if va < n.key.start_va {
                cur = n.left;
            } else if va >= n.key.end_va {
                cur = n.right;
            } else {
                return Some(&n.key);
            }
        }
        None
    }

    fn reset_tree(&mut self) {
        self.root = -1;
        for n in &mut self.nodes {
            *n = Node::empty();
        }
    }

    /// Remove the VAD whose **start** VA matches exactly, then rebuild the AVL from remaining entries.
    pub fn remove(&mut self, start_va: u64) -> Result<(), ()> {
        let mut buf = [VadEntry::new_range(
            0,
            1,
            VadKind::Reserve,
            PageProtect::NoAccess,
            false,
        ); MAX_NODES];
        let mut count = 0usize;
        let mut found = false;
        for e in self.iter() {
            if e.start_va == start_va {
                found = true;
                continue;
            }
            if count >= MAX_NODES {
                return Err(());
            }
            buf[count] = *e;
            count += 1;
        }
        if !found {
            return Err(());
        }
        self.reset_tree();
        for i in 0..count {
            self.insert(buf[i])?;
        }
        Ok(())
    }

    /// Split the containing VAD at `va` into `[start, va)` and `[va, end)` (strictly interior only).
    pub fn split_at_va(&mut self, va: u64) -> Result<(), ()> {
        let e = *self.find_by_va(va).ok_or(())?;
        if va <= e.start_va || va >= e.end_va {
            return Err(());
        }
        self.remove(e.start_va)?;
        self.insert(VadEntry {
            start_va: e.start_va,
            end_va: va,
            kind: e.kind,
            protect: e.protect,
            committed: e.committed,
            section: e.section,
        })?;
        self.insert(VadEntry {
            start_va: va,
            end_va: e.end_va,
            kind: e.kind,
            protect: e.protect,
            committed: e.committed,
            section: e.section,
        })?;
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|n| n.in_use).count()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root < 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &VadEntry> {
        self.nodes
            .iter()
            .filter(|n| n.in_use)
            .map(|n| &n.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_avl_insert_find() {
        let mut t = VadTable::new();
        let e = VadEntry::new_range(
            0x10_0000,
            0x20_0000,
            VadKind::Private,
            PageProtect::ReadWrite,
            true,
        );
        assert!(t.insert(e).is_ok());
        assert!(t.find_by_va(0x15_0000).is_some());
        assert!(t.find_by_va(0x09_0000).is_none());
    }

    #[test]
    fn vad_remove_and_split() {
        let mut t = VadTable::new();
        let e = VadEntry::new_range(
            0x10_0000,
            0x40_0000,
            VadKind::Private,
            PageProtect::ReadOnly,
            true,
        );
        assert!(t.insert(e).is_ok());
        assert!(t.split_at_va(0x20_0000).is_ok());
        assert_eq!(t.len(), 2);
        assert!(t.find_by_va(0x15_0000).is_some());
        assert!(t.find_by_va(0x30_0000).is_some());
        assert!(t.remove(0x10_0000).is_ok());
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn vad_insert_rejects_overlap() {
        let mut t = VadTable::new();
        assert!(t
            .insert(VadEntry::new_range(
                0x10_0000,
                0x20_0000,
                VadKind::Private,
                PageProtect::ReadWrite,
                true,
            ))
            .is_ok());
        assert!(t
            .insert(VadEntry::new_range(
                0x1F_0000,
                0x30_0000,
                VadKind::Private,
                PageProtect::ReadWrite,
                true,
            ))
            .is_err());
    }

    #[test]
    fn vad_coalesce_adjacent() {
        let mut t = VadTable::new();
        assert!(t
            .insert(VadEntry::new_range(
                0x10_0000,
                0x20_0000,
                VadKind::Private,
                PageProtect::ReadWrite,
                true,
            ))
            .is_ok());
        assert!(t
            .insert(VadEntry::new_range(
                0x20_0000,
                0x30_0000,
                VadKind::Private,
                PageProtect::ReadWrite,
                true,
            ))
            .is_ok());
        assert_eq!(t.len(), 2);
        assert!(t.coalesce_adjacent_compatible().is_ok());
        assert_eq!(t.len(), 1);
        let e = t.find_by_va(0x15_0000).unwrap();
        assert_eq!(e.start_va, 0x10_0000);
        assert_eq!(e.end_va, 0x30_0000);
    }
}
