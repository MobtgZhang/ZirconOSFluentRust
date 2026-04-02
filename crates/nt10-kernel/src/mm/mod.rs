//! Memory manager (MM).

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1u64 << PAGE_SHIFT;

pub mod boot_mem;
pub mod buddy;
pub mod bringup_user;
pub mod early_map;
pub mod heap;
pub mod large_page;
pub mod nx_image;
pub mod numa;
pub mod pagefile;
pub mod page_fault;
pub mod paging;
pub mod pfn;
pub mod phys;
pub mod pool;
pub mod pt;
pub mod section;
pub mod user_va;
pub mod uefi_user_cr3;
pub mod vad;
pub mod vm;
pub mod working_set;

pub mod high_half;
