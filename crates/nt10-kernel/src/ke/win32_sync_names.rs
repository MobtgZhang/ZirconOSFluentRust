//! Win32 synchronization API ↔ kernel dispatcher object naming (reference table).
//!
//! | Win32 | Kernel dispatcher (`ke/`) |
//! |-------|---------------------------|
//! | `CreateMutex` | [`super::mutex`] |
//! | `CreateEvent` | [`super::event`] |
//! | `CreateSemaphore` | [`super::semaphore`] |
//! | Wait APIs | [`super::waitobj`] |

/// Placeholder for future `\BaseNamedObjects\` resolution.
pub const BASE_NAMED_OBJECTS: &[u8] = br"\BaseNamedObjects\";
