//! Section objects (file-backed or anonymous memory sections for mapping).

use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};

use super::user_va::{USER_BRINGUP_STACK_TOP, USER_BRINGUP_VA};
use super::vad::{PageProtect, VadEntry, VadKind, VadTable};

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
