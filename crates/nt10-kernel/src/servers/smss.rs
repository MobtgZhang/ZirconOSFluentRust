//! Session Manager Subsystem — bootstrap order (documentation + hooks).
//!
//! ## Documented NT order vs this kernel
//! Full SMSS → CSRSS → Winlogon sequencing is described in `docs/cn/Loader-Win32k-Desktop.md`.
//! **Implemented today**: phase enum + [`SmssPhaseHooks`] stubs; no real `smss.exe` image load.
//! **Win32 path**: [`crate::subsystems::win32::csrss_host::bringup_kernel_thread_smoke`] registers
//! the subsystem from kernel context instead of spawning CSRSS.

use crate::ob::namespace::NamespaceBuckets;
use crate::ps::process::{EProcess, ProcessId};
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

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

/// Ordered native image names for subsystem bring-up (paths are ZirconOSFluent-specific).
pub const SMSS_BOOT_CHAIN: &[&[u8]] = &[
    b"SystemRoot\\System32\\smss.exe",
    b"SystemRoot\\System32\\csrss.exe",
];

#[must_use]
pub fn boot_image_at(step: usize) -> Option<&'static [u8]> {
    SMSS_BOOT_CHAIN.get(step).copied()
}

/// Future: load native SMSS/CSRSS image from VFS for `phase`. Bring-up returns `false`.
#[must_use]
pub fn try_load_native_image_stub(_phase: SmssPhase) -> bool {
    false
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

/// Walk [`SMSS_PHASE_ORDER`]: system phase (namespace hook) then each [`next_phase`] with hooks.
/// Does **not** load `smss.exe`; suitable for serial-ordered bring-up tests.
pub fn smss_run_documented_phase_chain(
    tracker: &mut SmssPhaseTracker,
    hooks: &SmssPhaseHooks,
    namespace: &mut NamespaceBuckets,
) -> Result<(), ()> {
    smss_stub_run_system_phase(tracker, hooks, namespace)?;
    while tracker.advance_invoke(hooks).is_some() {}
    Ok(())
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

/// QEMU / CI: keep [`crate::subsystems::win32::csrss_host`] until a Ring-3 csrss image loads.
pub const NT10_PHASE6_RING3_CSRSS_FALLBACK_TO_KERNEL_HOST: bool = true;

static RING3_PLACEHOLDER_CR3: AtomicU64 = AtomicU64::new(0);

/// CR3 allocated by [`try_register_ring3_placeholder_process`] (0 if none).
#[must_use]
pub fn ring3_placeholder_cr3_phys() -> u64 {
    RING3_PLACEHOLDER_CR3.load(Ordering::Relaxed)
}

/// Publish `cr3_phys` as the shared bring-up user CR3 when none is set yet, or succeed if it already matches.
pub fn try_set_ring3_placeholder_cr3(cr3_phys: u64) -> Result<(), ()> {
    if cr3_phys == 0 {
        return Err(());
    }
    match RING3_PLACEHOLDER_CR3.compare_exchange(0, cr3_phys, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => Ok(()),
        Err(v) if v == cr3_phys => Ok(()),
        Err(_) => Err(()),
    }
}

/// Allocates a user page table via [`crate::mm::uefi_user_cr3::build_uefi_first_user_cr3`] and records it for SMSS/ALPC bring-up.
/// Does **not** load `smss.exe`; see [`try_launch_ring3_smss_from_vfs`].
#[must_use]
pub fn try_register_ring3_placeholder_process() -> Result<ProcessId, ()> {
    #[cfg(all(target_arch = "x86_64", not(test)))]
    {
        if !crate::mm::phys::pfn_pool_initialized() {
            return Err(());
        }
        if RING3_PLACEHOLDER_CR3.load(Ordering::Acquire) != 0 {
            return Err(());
        }
        let cr3 = unsafe { crate::mm::uefi_user_cr3::build_uefi_first_user_cr3() }.ok_or(())?;
        RING3_PLACEHOLDER_CR3.store(cr3, Ordering::Release);
        let p = EProcess::new_bootstrap();
        return Ok(p.pid);
    }
    #[cfg(any(not(target_arch = "x86_64"), test))]
    {
        Err(())
    }
}

/// Phase 6 scaffold: prefer a real `smss.exe` load from VFS; otherwise register a Ring-3 CR3 placeholder when on bare-metal x86_64.
#[must_use]
pub fn try_launch_ring3_smss_from_vfs() -> Result<ProcessId, ()> {
    if try_load_native_image_stub(SmssPhase::SystemProcess) {
        return Ok(EProcess::new_bootstrap().pid);
    }
    if NT10_PHASE6_RING3_CSRSS_FALLBACK_TO_KERNEL_HOST {
        return try_register_ring3_placeholder_process();
    }
    Err(())
}

/// smss → csrss hand-off over ALPC (stub; see [`crate::alpc::phase6_csrss`]).
#[must_use]
pub fn try_smss_alpc_start_csrss_stub(parent: ProcessId) -> Result<(), ()> {
    crate::alpc::phase6_csrss::try_alpc_handoff_csrss_spawn_stub(parent, 0)
}

#[cfg(test)]
mod phase6_tests {
    use super::*;

    #[test]
    fn ring3_smss_and_alpc_stubs_return_err() {
        assert!(try_register_ring3_placeholder_process().is_err());
        assert!(try_launch_ring3_smss_from_vfs().is_err());
        assert!(try_smss_alpc_start_csrss_stub(ProcessId(1)).is_err());
    }

    #[test]
    fn documented_phase_chain_runs_hooks_in_order() {
        static HIT: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
        let hooks = SmssPhaseHooks {
            on_system_process: Some(|| {
                HIT.fetch_or(1, Ordering::SeqCst);
            }),
            on_win32_subsystem: Some(|| {
                HIT.fetch_or(2, Ordering::SeqCst);
            }),
            on_interactive_logon: Some(|| {
                HIT.fetch_or(4, Ordering::SeqCst);
            }),
        };
        let mut tr = SmssPhaseTracker::new();
        let mut ns = NamespaceBuckets::new();
        smss_run_documented_phase_chain(&mut tr, &hooks, &mut ns).expect("chain");
        let v = HIT.load(Ordering::SeqCst);
        assert_eq!(v, 7, "expected all three hooks");
    }
}
