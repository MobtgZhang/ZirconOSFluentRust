//! Section objects (file-backed or anonymous memory sections for mapping).
//!
//! **Limits:** [`SectionBacking::AnonymousPages`] is capped by [`SECTION_ANONYMOUS_PAGE_CAP`]; exceeding
//! it requires a future PFN list. Tear-down order: drop VAD mappings before [`SectionObject::release`].

use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use super::phys::pfn_bringup_alloc;
use super::user_va::{USER_BRINGUP_STACK_TOP, USER_BRINGUP_VA};
use super::vad::{PageProtect, VadEntry, VadKind, VadTable};
use super::PAGE_SIZE;

/// Max 4 KiB frames tracked inline per anonymous section (no `alloc` global allocator on bare metal).
pub const SECTION_ANONYMOUS_PAGE_CAP: usize = 256;

/// [`SectionObject::commit_anonymous_pages`] failures (explicit semantics vs silent `Err(())`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionCommitError {
    /// Inline [`SECTION_ANONYMOUS_PAGE_CAP`] exhausted; use a different backing strategy (future: spill list).
    AnonymousCapExceeded,
    /// Cannot grow anonymous storage on a file-backed section.
    FileBacked,
    /// PFN pool could not supply a frame.
    PfnExhausted,
}

/// Backing store for a section (clean-room layout).
#[derive(Clone, Debug)]
pub enum SectionBacking {
    /// Placeholder until per-PFN lists are wired for anonymous sections.
    None,
    /// Physical frames for an anonymous section (inline array; cap [`SECTION_ANONYMOUS_PAGE_CAP`]).
    AnonymousPages {
        phys: [u64; SECTION_ANONYMOUS_PAGE_CAP],
        count: usize,
    },
    /// FAT32 root short-name file on a bring-up [`crate::fs::vfs::VfsTable`] mount.
    FileBackedStub {
        mount_slot: u8,
        root_name11: [u8; 11],
        /// Byte offset in the file that maps to VAD `start_va`.
        offset: u64,
        size: u64,
    },
}

/// Kernel section object (reference-counted).
pub struct SectionObject {
    pub maximum_size: u64,
    pub image: bool,
    pub backing: SectionBacking,
    ref_count: AtomicU32,
}

impl SectionObject {
    #[must_use]
    pub fn new(maximum_size: u64, image: bool, backing: SectionBacking) -> Self {
        Self {
            maximum_size,
            image,
            backing,
            ref_count: AtomicU32::new(1),
        }
    }

    #[must_use]
    pub fn bringup_readonly_user_window() -> Self {
        Self {
            maximum_size: USER_BRINGUP_STACK_TOP - USER_BRINGUP_VA,
            image: false,
            backing: SectionBacking::None,
            ref_count: AtomicU32::new(1),
        }
    }

    pub fn retain(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns `true` if the object should be destroyed (ref count hit zero).
    pub fn release(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::Release) == 1
    }

    /// Bytes of anonymous storage recorded in [`SectionBacking::AnonymousPages`].
    #[must_use]
    pub fn committed_anonymous_bytes(&self) -> u64 {
        match &self.backing {
            SectionBacking::AnonymousPages { count, .. } => (*count as u64).saturating_mul(PAGE_SIZE),
            _ => 0,
        }
    }

    /// Grow anonymous backing by `pages` frames from the PFN pool.
    pub fn commit_anonymous_pages(&mut self, pages: usize) -> Result<(), SectionCommitError> {
        if pages == 0 {
            return Ok(());
        }
        match &mut self.backing {
            SectionBacking::AnonymousPages { phys, count } => {
                for _ in 0..pages {
                    if *count >= phys.len() {
                        return Err(SectionCommitError::AnonymousCapExceeded);
                    }
                    let p = pfn_bringup_alloc().ok_or(SectionCommitError::PfnExhausted)?;
                    phys[*count] = p;
                    *count += 1;
                }
                Ok(())
            }
            SectionBacking::None => {
                let mut phys = [0u64; SECTION_ANONYMOUS_PAGE_CAP];
                let mut count = 0usize;
                for _ in 0..pages {
                    if count >= phys.len() {
                        return Err(SectionCommitError::AnonymousCapExceeded);
                    }
                    phys[count] = pfn_bringup_alloc().ok_or(SectionCommitError::PfnExhausted)?;
                    count += 1;
                }
                self.backing = SectionBacking::AnonymousPages { phys, count };
                Ok(())
            }
            SectionBacking::FileBackedStub { .. } => Err(SectionCommitError::FileBacked),
        }
    }

    /// Read up to `page.len()` bytes from the backing file at absolute `file_offset` (demand paging).
    pub fn read_file_backed_page(&self, file_offset: u64, page: &mut [u8]) -> Result<usize, ()> {
        let SectionBacking::FileBackedStub {
            mount_slot,
            root_name11,
            offset: map_base,
            size,
        } = &self.backing
        else {
            return Err(());
        };
        if file_offset < *map_base || file_offset >= map_base.saturating_add(*size) {
            return Ok(0);
        }
        let vfs = crate::fs::vfs::vfs_bringup_ptr().ok_or(())?;
        let vfs = unsafe { vfs.as_ref() };
        crate::fs::vfs::vfs_read_fat32_root_file_partial(
            vfs,
            *mount_slot as usize,
            root_name11,
            file_offset,
            page,
        )
    }
}

/// Registers a [`VadEntry`] for the built-in user region using `section` size and protection.
pub fn install_bringup_section_vad(vad: &mut VadTable, section: &SectionObject) -> Result<(), ()> {
    let span = section
        .maximum_size
        .min(USER_BRINGUP_STACK_TOP.saturating_sub(USER_BRINGUP_VA));
    let end = USER_BRINGUP_VA.saturating_add(span);
    let has_anon_pfns = matches!(
        &section.backing,
        SectionBacking::AnonymousPages { count, .. } if *count > 0
    );
    let protect = if has_anon_pfns {
        PageProtect::ReadWrite
    } else {
        PageProtect::ReadOnly
    };
    let entry = VadEntry::new_range(
        USER_BRINGUP_VA,
        end,
        VadKind::Mapped,
        protect,
        true,
    );
    vad.insert(entry)?;
    if crate::mm::phys::pfn_pool_initialized() {
        let cr3 = crate::arch::x86_64::paging::read_cr3();
        unsafe {
            if let Some(e) = vad.find_by_va(USER_BRINGUP_VA) {
                match &section.backing {
                    SectionBacking::AnonymousPages { phys, count } if *count > 0 => {
                        let _ = super::pt::map_committed_range_to_pfns(
                            cr3,
                            USER_BRINGUP_VA,
                            &phys[..*count],
                            e,
                        );
                    }
                    _ => {
                        let _ = super::pt::apply_vad_to_page_tables(cr3, e);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Write-on-copy view with refcounted section reference (opaque pointer until full OB wiring).
pub struct CowView {
    pub section: NonNull<SectionObject>,
}

impl CowView {
    /// # Safety
    /// `section` must point to a live [`SectionObject`] for the `'a` lifetime.
    pub unsafe fn from_section_ptr(section: *mut SectionObject) -> Option<Self> {
        NonNull::new(section).map(|p| Self { section: p })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_anonymous_none_is_zero() {
        let s = SectionObject::new(4096, false, SectionBacking::None);
        assert_eq!(s.committed_anonymous_bytes(), 0);
    }

    #[test]
    fn file_backed_rejects_anonymous_commit() {
        let mut s = SectionObject::new(
            4096,
            false,
            SectionBacking::FileBackedStub {
                mount_slot: 0,
                root_name11: *b"X       YYY",
                offset: 0,
                size: 4096,
            },
        );
        assert_eq!(
            s.commit_anonymous_pages(1),
            Err(super::SectionCommitError::FileBacked)
        );
    }
}
