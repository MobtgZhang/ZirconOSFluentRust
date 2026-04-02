//! Section objects (file-backed or anonymous memory sections for mapping).

use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use super::phys::pfn_bringup_alloc;
use super::user_va::{USER_BRINGUP_STACK_TOP, USER_BRINGUP_VA};
use super::vad::{PageProtect, VadEntry, VadKind, VadTable};
use super::PAGE_SIZE;

/// Backing store for a section (clean-room layout).
#[derive(Clone, Debug)]
pub enum SectionBacking {
    /// Placeholder until per-PFN lists are wired for anonymous sections.
    None,
    /// Physical frames for an anonymous section (small inline pool).
    AnonymousPages {
        phys: [u64; 32],
        count: usize,
    },
    /// File-backed stub (handle is opaque).
    FileBackedStub {
        file_handle: u64,
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
    pub fn commit_anonymous_pages(&mut self, pages: usize) -> Result<(), ()> {
        if pages == 0 {
            return Ok(());
        }
        match &mut self.backing {
            SectionBacking::AnonymousPages { phys, count } => {
                for _ in 0..pages {
                    if *count >= phys.len() {
                        return Err(());
                    }
                    let p = pfn_bringup_alloc().ok_or(())?;
                    phys[*count] = p;
                    *count += 1;
                }
                Ok(())
            }
            SectionBacking::None => {
                let mut phys = [0u64; 32];
                let mut count = 0usize;
                for _ in 0..pages {
                    if count >= phys.len() {
                        return Err(());
                    }
                    phys[count] = pfn_bringup_alloc().ok_or(())?;
                    count += 1;
                }
                self.backing = SectionBacking::AnonymousPages { phys, count };
                Ok(())
            }
            SectionBacking::FileBackedStub { .. } => Err(()),
        }
    }
}

/// Registers a [`VadEntry`] for the built-in user region using `section` size and protection.
pub fn install_bringup_section_vad(vad: &mut VadTable, section: &SectionObject) -> Result<(), ()> {
    let span = section
        .maximum_size
        .min(USER_BRINGUP_STACK_TOP.saturating_sub(USER_BRINGUP_VA));
    let end = USER_BRINGUP_VA.saturating_add(span);
    let entry = VadEntry::new_range(
        USER_BRINGUP_VA,
        end,
        VadKind::Mapped,
        PageProtect::ReadOnly,
        true,
    );
    vad.insert(entry)?;
    if crate::mm::phys::pfn_pool_initialized() {
        let cr3 = crate::arch::x86_64::paging::read_cr3();
        unsafe {
            if let Some(e) = vad.find_by_va(USER_BRINGUP_VA) {
                let _ = super::pt::apply_vad_to_page_tables(cr3, e);
            }
        }
    }
    let _ = section;
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
}
