//! KTimer / kernel timers (bring-up).
//!
//! IRQ-backed ticks ultimately funnel into [`super::sched`] / DPC-style work from
//! [`crate::arch::x86_64::isr`] when the LAPIC timer path is armed (see [`crate::hal::x86_64::apic`]).
//!
//! ## UEFI Phase 5 path (`SetTimer` / `KillTimer` bring-up)
//!
//! There is no IRQ timer delivery into the single-threaded UEFI poll loop. The formal integration
//! point is:
//!
//! 1. **Cooperative quanta**: [`crate::desktop::fluent::session_win32::maybe_post_timer`] runs every
//!    ~1024 [`DesktopSession::poll_seq`](crate::desktop::fluent::session::DesktopSession::poll_seq)
//!    steps (see `session_win32.rs`) and posts one `WM_TIMER` to the taskbar HWND when armed.
//! 2. **Executive hooks**: when a real tick source exists, [`super::sched::timer_quanta`] /
//!    [`super::sched::on_timer_tick`] remain the place to convert hardware ticks into scheduler work;
//!    wiring those ticks to `WM_TIMER` is future work.
//!
//! Arm / disarm: [`crate::desktop::fluent::session_win32::set_timer_taskbar_bringup`] and
//! [`crate::desktop::fluent::session_win32::kill_timer_taskbar_bringup`].

pub use super::sched::{on_timer_tick, timer_quanta};
