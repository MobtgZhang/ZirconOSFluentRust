//! Section objects (file-backed or anonymous memory sections for mapping).
//!
//! Backed views are installed through the memory manager and [`super::vad::VadTable`].

use super::user_va::{USER_BRINGUP_STACK_TOP, USER_BRINGUP_VA};
use super::vad::{VadEntry, VadKind, VadTable};

/// Kernel section object (anonymous or file-backed).
#[derive(Clone, Copy, Debug)]
pub struct SectionObject {
    pub maximum_size: u64,
    pub image: bool,
}

impl SectionObject {
    #[must_use]
    pub const fn new(maximum_size: u64, image: bool) -> Self {
        Self {
            maximum_size,
            image,
        }
    }

    /// Read-only anonymous section covering the built-in user smoke VA window (identity-mapped).
    #[must_use]
    pub const fn bringup_readonly_user_window() -> Self {
        Self {
            maximum_size: USER_BRINGUP_STACK_TOP - USER_BRINGUP_VA,
            image: false,
        }
    }
}

/// Registers a [`VadEntry`] for the built-in user region and ties it to a read-only section (bring-up).
pub fn install_bringup_section_vad(vad: &mut VadTable, section: &SectionObject) -> Result<(), ()> {
    let _ = section;
    vad.insert(VadEntry {
        start_va: USER_BRINGUP_VA,
        end_va: USER_BRINGUP_STACK_TOP,
        kind: VadKind::Mapped,
    })
}

/// Write-on-copy view (placeholder).
#[derive(Clone, Copy, Debug)]
pub struct CowView {
    pub section: *mut SectionObject,
}
