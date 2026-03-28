//! Roadmap phase markers from [docs/en/Roadmap-and-TODO.md](../../../docs/en/Roadmap-and-TODO.md).
//! These constants are for documentation, logging, and future feature gates — not a runtime state machine.

/// Phase 10 — Win32k / user-mode graphics path.
pub const PHASE_WIN32K_GRAPHICS: u8 = 10;
/// Phase 11 — WOW64-style thunking.
pub const PHASE_WOW64: u8 = 11;
/// Phase 12 — CFG, CET, DEP, MIC hardening.
pub const PHASE_MODERN_SECURITY: u8 = 12;
/// Phase 13 — Hyper-V awareness.
pub const PHASE_HYPERV: u8 = 13;
/// Phase 14 — Fluent shell / DWM integration.
pub const PHASE_FLUENT_DESKTOP: u8 = 14;
/// Phase 15 — WinRT / UWP protocol stubs.
pub const PHASE_WINRT: u8 = 15;
