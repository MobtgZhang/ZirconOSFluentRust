//! Fluent Design desktop (NT 10 default).
//!
//! ## Compliance (Win32 docs)
//! `references/win32` (e.g. LearnWin32) is used only for **conceptual** alignment (input routing,
//! layering). Taskbar / notification semantics follow
//! `references/win32/desktop-src/shell/taskbar.md` and `shell/notification-area.md`; UX notes in
//! `uxguide/winenv-taskbar.md` / `uxguide/winenv-notification.md`. Do **not** copy Microsoft sample
//! code or copyrighted assets. UI uses Zircon Fluent resources under `resources/` and **OFL** fonts
//! via `build.rs` / `third_party/fonts/`.
//!
//! ## Ring3 migration
//! Hosted apps below are kernel-drawn for UEFI bring-up; each `hosted_apps` / `app_host` surface
//! should move behind **PE + user32** when the Win32k stack is ready (see per-module notes).

pub mod acrylic;
pub mod app_host;
pub mod clipboard;
pub mod context_menu_registry;
pub mod dwm;
pub mod explorer_view;
pub mod font_stub;
pub mod hosted_apps;
pub mod known_folder;
pub mod lnk;
pub mod mica;
pub mod resources;
pub mod session;
mod session_win32;
pub mod shell;
pub mod shell_namespace;
pub mod taskbar;
pub mod wall_clock;
