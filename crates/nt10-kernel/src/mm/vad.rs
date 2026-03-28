//! Virtual Address Descriptors — fixed-capacity table until AVL lands.
//!
//! User addresses should stay inside [`crate::mm::user_va`] bounds once user mode is enabled.

const MAX_VADS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VadKind {
    Private,
    Mapped,
    Reserve,
}

#[derive(Clone, Copy, Debug)]
pub struct VadEntry {
    pub start_va: u64,
    pub end_va: u64,
    pub kind: VadKind,
}

/// Per-process region list (bring-up; no tree balancing yet).
pub struct VadTable {
    entries: [Option<VadEntry>; MAX_VADS],
    count: usize,
}

impl VadTable {
    pub const fn new() -> Self {
        const NONE: Option<VadEntry> = None;
        Self {
            entries: [NONE; MAX_VADS],
            count: 0,
        }
    }

    pub fn insert(&mut self, e: VadEntry) -> Result<(), ()> {
        if self.count >= MAX_VADS {
            return Err(());
        }
        self.entries[self.count] = Some(e);
        self.count += 1;
        Ok(())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &VadEntry> {
        self.entries[..self.count].iter().filter_map(|x| x.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_insert_and_len() {
        let mut t = VadTable::new();
        let e = VadEntry {
            start_va: 0x10_0000,
            end_va: 0x20_0000,
            kind: VadKind::Private,
        };
        assert!(t.insert(e).is_ok());
        assert_eq!(t.len(), 1);
        assert!(!t.is_empty());
    }
}
