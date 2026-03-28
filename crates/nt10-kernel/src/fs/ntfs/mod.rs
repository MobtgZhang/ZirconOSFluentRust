//! NTFS — on-disk structures; read-only bring-up paths align with [Roadmap Phase 6](../../../../../docs/en/Roadmap-and-TODO.md).

pub mod attr;
pub mod index;
pub mod log;
pub mod mft;
#[allow(clippy::module_inception)]
pub mod ntfs;
pub mod usn;
