#![no_std]

//! ZBM10 UEFI boot manager library surface.
//!
//! The `zbm10` binary implements the firmware entry; this crate re-exports the handoff protocol.

pub use nt10_boot_protocol::{ZirconBootInfo, ZIRNON10_MAGIC};
