//! Session Manager Subsystem — bootstrap order (documentation + hooks).
//!
//! ## Documented NT order vs this kernel
//! Full SMSS → CSRSS → Winlogon sequencing is described in `docs/cn/Loader-Win32k-Desktop.md`.
//! **Implemented today**: phase enum + [`SmssPhaseHooks`] stubs; no real `smss.exe` image load.
//! **Win32 path**: [`crate::subsystems::win32::csrss_host::bringup_kernel_thread_smoke`] registers
//! the subsystem from kernel context instead of spawning CSRSS.

use crate::ob::namespace::NamespaceBuckets;
use crate::ps::process::{EProcess, ProcessId};
use core::sync::atomic::{AtomicU8, Ordering};

/// Ordered steps for NT-style startup; callers execute when dependencies exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmssPhase {
    /// Kernel executive ready; create initial system process.
    SystemProcess,
    /// Start CSRSS / Win32 subsystem registration.
    Win32Subsystem,
    /// User logon path (LSASS, Winlogon) — far future.
    InteractiveLogon,
}

/// Returns the next planned phase after `current` completes.
#[must_use]
pub fn next_phase(current: SmssPhase) -> Option<SmssPhase> {
    match current {
        SmssPhase::SystemProcess => Some(SmssPhase::Win32Subsystem),
        SmssPhase::Win32Subsystem => Some(SmssPhase::InteractiveLogon),
        SmssPhase::InteractiveLogon => None,
    }
}

/// Ordered native image names for subsystem bring-up (paths are ZirconOS-specific).
pub const SMSS_BOOT_CHAIN: &[&[u8]] = &[
    b"SystemRoot\\System32\\smss.exe",
    b"SystemRoot\\System32\\csrss.exe",
];

#[must_use]
pub fn boot_image_at(step: usize) -> Option<&'static [u8]> {
    SMSS_BOOT_CHAIN.get(step).copied()
}

/// Canonical SMSS ordering for documentation and tests (not executed by the kernel yet).
pub const SMSS_PHASE_ORDER: &[SmssPhase] = &[
    SmssPhase::SystemProcess,
    SmssPhase::Win32Subsystem,
    SmssPhase::InteractiveLogon,
];

/// Records the current SMSS phase and advances along [`SMSS_PHASE_ORDER`].
#[derive(Clone, Copy, Debug)]
pub struct SmssPhaseTracker {
    pub current: SmssPhase,
}

impl SmssPhaseTracker {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current: SmssPhase::SystemProcess,
        }
    }

    /// Returns the phase entered after advance, or `None` if already at the terminal phase.
    pub fn advance(&mut self) -> Option<SmssPhase> {
        let n = next_phase(self.current)?;
        self.current = n;
        Some(n)
    }

    /// Runs [`SmssPhaseHooks`] for the phase currently recorded (no state change).
    pub fn invoke_current(&self, hooks: &SmssPhaseHooks) {
        hooks.invoke(self.current);
    }

    /// Advances one step then invokes hooks for the new phase.
    pub fn advance_invoke(&mut self, hooks: &SmssPhaseHooks) -> Option<SmssPhase> {
        self.advance()?;
        hooks.invoke(self.current);
        Some(self.current)
    }
}

/// Optional per-phase callbacks (bring-up: logging or stub spawns). All pointers may be `None`.
pub struct SmssPhaseHooks {
    pub on_system_process: Option<fn()>,
    pub on_win32_subsystem: Option<fn()>,
    pub on_interactive_logon: Option<fn()>,
}

impl SmssPhaseHooks {
    pub const STUB: Self = Self {
        on_system_process: None,
        on_win32_subsystem: None,
        on_interactive_logon: None,
    };

    pub fn invoke(&self, phase: SmssPhase) {
        match phase {
            SmssPhase::SystemProcess => {
                if let Some(f) = self.on_system_process {
                    f();
                }
            }
            SmssPhase::Win32Subsystem => {
                if let Some(f) = self.on_win32_subsystem {
                    f();
                }
            }
            SmssPhase::InteractiveLogon => {
                if let Some(f) = self.on_interactive_logon {
                    f();
                }
            }
        }
    }
}

static SMSS_LAST_PHASE: AtomicU8 = AtomicU8::new(0);

fn phase_tag(p: SmssPhase) -> u8 {
    match p {
        SmssPhase::SystemProcess => 1,
        SmssPhase::Win32Subsystem => 2,
        SmssPhase::InteractiveLogon => 3,
    }
}

/// Bring-up: records last phase and a placeholder system [`ProcessId`] after stub system process creation.
pub fn smss_stub_run_system_phase(
    tracker: &mut SmssPhaseTracker,
    hooks: &SmssPhaseHooks,
    namespace: &mut NamespaceBuckets,
) -> Result<ProcessId, ()> {
    tracker.invoke_current(hooks);
    let proc = EProcess::new_bootstrap();
    let pid = proc.pid;
    SMSS_LAST_PHASE.store(phase_tag(tracker.current), Ordering::Release);
    let sentinel = core::ptr::NonNull::new(1usize as *mut ()).unwrap();
    let path = br"\Sessions\0\SMSS_StubSystem";
    namespace.insert_session_path(path, sentinel)?;
    let _ = proc;
    Ok(pid)
}

/// Last completed SMSS phase marker for diagnostics (`0` = none, `1`..=`3` = phase tags).
#[must_use]
pub fn smss_last_phase_tag() -> u8 {
    SMSS_LAST_PHASE.load(Ordering::Relaxed)
}
