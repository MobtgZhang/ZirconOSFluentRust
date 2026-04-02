//! Directory objects — byte-name insertion and lookup (ZirconOS naming; not SDK literals).

use core::ptr::NonNull;

const MAX_ENTRIES: usize = 32;
const MAX_NAME: usize = 64;

#[derive(Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; MAX_NAME],
    pub name_len: usize,
    pub object: NonNull<()>,
}

/// Fixed-capacity directory (bring-up).
pub struct DirectoryObject {
    entries: [Option<DirEntry>; MAX_ENTRIES],
    count: usize,
}

impl DirectoryObject {
    #[must_use]
    pub const fn new() -> Self {
        const NONE: Option<DirEntry> = None;
        Self {
            entries: [NONE; MAX_ENTRIES],
            count: 0,
        }
    }

    pub fn insert(&mut self, name: &[u8], object: NonNull<()>) -> Result<(), ()> {
        if name.is_empty() || name.len() > MAX_NAME || self.count >= MAX_ENTRIES {
            return Err(());
        }
        if self.lookup(name).is_some() {
            return Err(());
        }
        let mut buf = [0u8; MAX_NAME];
        buf[..name.len()].copy_from_slice(name);
        self.entries[self.count] = Some(DirEntry {
            name: buf,
            name_len: name.len(),
            object,
        });
        self.count += 1;
        Ok(())
    }

    #[must_use]
    pub fn lookup(&self, name: &[u8]) -> Option<NonNull<()>> {
        for e in self.entries[..self.count].iter().flatten() {
            if name.len() == e.name_len && name == &e.name[..e.name_len] {
                return Some(e.object);
            }
        }
        None
    }

    pub fn remove(&mut self, name: &[u8]) -> Result<(), ()> {
        for i in 0..self.count {
            let matches = self.entries[i].as_ref().is_some_and(|e| {
                name.len() == e.name_len && name == &e.name[..e.name_len]
            });
            if matches {
                self.entries[i] = self.entries[self.count - 1].take();
                self.count -= 1;
                return Ok(());
            }
        }
        Err(())
    }

    pub fn iter_objects(&self) -> impl Iterator<Item = NonNull<()>> + '_ {
        self.entries[..self.count]
            .iter()
            .flatten()
            .map(|e| e.object)
    }

    /// Clear slots without invoking per-object delete (caller tears children down first).
    pub fn clear_for_teardown(&mut self) {
        const NONE: Option<DirEntry> = None;
        self.entries = [NONE; MAX_ENTRIES];
        self.count = 0;
    }
}
