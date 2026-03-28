//! PEB/TEB references (user-mode layout is documented publicly; sizes are bring-up placeholders).
//!
//! ## Bring-up PEB layout (ZirconOS NT10 smoke, **not** a Win32 SDK layout)
//!
//! | Field (conceptual) | Offset | Size | Notes |
//! |--------------------|--------|------|-------|
//! | `BeingDebugged`    | 0x02   | 1    | `u8`, 0 in smoke |
//! | `ImageBaseAddress` | 0x10   | 8    | `u64` VA of module base |
//!
//! ### WOW64 (32-bit user view, ZirconOS bring-up placeholders)
//!
//! | Field (conceptual)   | Offset | Size | Notes |
//! |----------------------|--------|------|-------|
//! | `BeingDebugged`      | 0x02   | 1    | `u8` |
//! | `ImageBaseAddress`   | 0x08   | 4    | `u32` VA in 32-bit space |
//! | `Ldr`                | 0x0C   | 4    | `u32` stub pointer |
//!
//! Only offsets used by the kernel today are listed; the user buffer may be sparse.
//!
//! ### 32-bit TEB (smoke)
//!
//! | Field       | Offset | Size | Notes        |
//! |-------------|--------|------|--------------|
//! | `Self` ptr  | 0x18   | 4    | `u32` linear |
//! | `ClientId`  | 0x20   | 8    | two `u32`    |

use crate::mm::user_va::USER_BRINGUP_VA;

/// User-mode PEB pointer (kernel stores VA once user space exists).
#[derive(Clone, Copy, Debug)]
pub struct PebRef {
    pub user_va: u64,
}

impl PebRef {
    pub const fn none() -> Self {
        Self { user_va: 0 }
    }

    /// Fixed PEB VA for the built-in ring-3 smoke region (inside the same 2 MiB user page as code).
    #[must_use]
    pub const fn bringup_smoke() -> Self {
        Self {
            user_va: USER_BRINGUP_VA + 0x1000,
        }
    }

    /// PEB user VA after the first image is mapped (`image_base` is the preferred/load base for documentation).
    #[must_use]
    pub const fn after_first_image(peb_user_va: u64, _image_base: u64) -> Self {
        Self {
            user_va: peb_user_va,
        }
    }
}

/// Thread environment block reference.
#[derive(Clone, Copy, Debug)]
pub struct TebRef {
    pub user_va: u64,
}

impl TebRef {
    pub const fn none() -> Self {
        Self { user_va: 0 }
    }
}
