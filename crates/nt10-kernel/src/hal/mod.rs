//! Hardware abstraction layer (HAL).

pub mod aarch64;
pub mod traits;
pub mod x86_64;

pub use traits::{Hal, X86Hal64};
