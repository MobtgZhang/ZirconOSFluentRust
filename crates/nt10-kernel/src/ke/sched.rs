//! Scheduler (multi-level feedback) — bring-up timer + minimal round-robin index.
//!
//! ## Bring-up invariants (ZirconOSFluent)
//! - Each hardware timer IRQ calls [`on_timer_tick`], which increments [`TIMER_QUANTA`]. When
//!   [`RR_LEN`] > 0, the RR cursor advances once every [`RR_TICKS_PER_SCHED_SLICE`] ticks (software quantum).
//! - **Preemption:** the ISR does **not** perform a full context switch; cooperative [`yield_message_wait`]
//!   and DPC drain advance multi-threaded bring-up tests. MLFQ and real context-switch preemption are future work.

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use crate::hal::Hal;
use crate::rtl::log::{log_line_hal, SUB_KE};
use crate::ke::spinlock::SpinLock;

#[cfg(target_arch = "x86_64")]
fn bringup_noop_dpc(_: *mut ()) {}

#[cfg(target_arch = "x86_64")]
static mut SAMPLE_DPC: crate::ke::dpc::DpcObject =
    crate::ke::dpc::DpcObject::new(bringup_noop_dpc, core::ptr::null_mut());

/// NT-style band: 0..=15 dynamic, 16..=31 real-time class (documentation mirror).
pub const MAX_PRIORITY: u8 = 31;

/// Opaque thread id for bring-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ThreadId(pub u32);

/// Minimal runnable record until `ETHREAD` exists.
#[derive(Clone, Copy, Debug)]
pub struct ThreadStub {
    pub id: ThreadId,
    pub priority: u8,
}

static NEXT_THREAD: AtomicU32 = AtomicU32::new(1);

/// Quantum counter (incremented each timer IRQ); future: pick next `ThreadStub`.
static TIMER_QUANTA: AtomicU32 = AtomicU32::new(0);

/// Timer ticks per RR cursor step (software scheduling slice for bring-up).
pub const RR_TICKS_PER_SCHED_SLICE: u32 = 4;

/// Back-compat alias: prefer [`RR_TICKS_PER_SCHED_SLICE`].
pub const BRINGUP_QUANTUM_TICKS: u32 = RR_TICKS_PER_SCHED_SLICE;

static RR_TICK_ACCUM: AtomicU32 = AtomicU32::new(0);

const RR_CAP: usize = 8;
static RR_READY: SpinLock<[Option<ThreadStub>; RR_CAP]> = SpinLock::new([None; RR_CAP]);
static RR_LEN: AtomicUsize = AtomicUsize::new(0);
static RR_INDEX: AtomicUsize = AtomicUsize::new(0);

impl ThreadStub {
    #[must_use]
    pub fn new(priority: u8) -> Self {
        let id = NEXT_THREAD.fetch_add(1, Ordering::Relaxed);
        Self {
            id: ThreadId(id),
            priority: priority.min(MAX_PRIORITY),
        }
    }
}

/// True when `tick_n` (1-based count since boot) completes a new RR slice.
#[must_use]
pub const fn rr_should_rotate_at_tick(tick_n: u32) -> bool {
    tick_n != 0 && tick_n % RR_TICKS_PER_SCHED_SLICE == 0
}

/// Called from the timer ISR on x86_64 (PIC IRQ0 or LAPIC timer).
pub fn on_timer_tick() {
    TIMER_QUANTA.fetch_add(1, Ordering::Relaxed);
    let n = RR_LEN.load(Ordering::Acquire);
    if n > 0 {
        let prev = RR_TICK_ACCUM.fetch_add(1, Ordering::Relaxed);
        let t = prev.wrapping_add(1);
        if rr_should_rotate_at_tick(t) {
            RR_INDEX.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[must_use]
pub fn timer_quanta() -> u32 {
    TIMER_QUANTA.load(Ordering::Relaxed)
}

#[must_use]
pub fn rr_current_index() -> usize {
    let n = RR_LEN.load(Ordering::Relaxed);
    if n == 0 {
        return 0;
    }
    RR_INDEX.load(Ordering::Relaxed) % n
}

#[must_use]
pub fn rr_thread_at_cursor() -> Option<ThreadStub> {
    let n = RR_LEN.load(Ordering::Acquire);
    if n == 0 {
        return None;
    }
    let i = RR_INDEX.load(Ordering::Relaxed) % n;
    let g = RR_READY.lock();
    g[i]
}

/// Extra runnable for bring-up (e.g. ties [`crate::ps::thread::EThread`] to the RR cursor).
pub fn rr_register_thread(stub: ThreadStub) -> Result<(), ()> {
    rr_push(stub)
}

fn rr_push(stub: ThreadStub) -> Result<(), ()> {
    let mut g = RR_READY.lock();
    let len = RR_LEN.load(Ordering::Relaxed);
    if len >= RR_CAP {
        return Err(());
    }
    g[len] = Some(stub);
    RR_LEN.store(len + 1, Ordering::Release);
    Ok(())
}

/// Copy up to `out.len()` RR stubs for Task Manager (read-only bring-up).
#[must_use]
pub fn rr_ready_snapshot(out: &mut [Option<ThreadStub>]) -> usize {
    let g = RR_READY.lock();
    let len = RR_LEN.load(Ordering::Acquire) as usize;
    let n = len.min(RR_CAP).min(out.len());
    for i in 0..n {
        out[i] = g[i];
    }
    n
}

/// Two-thread RR bring-up (timer rotates [`RR_INDEX`]).
pub fn rr_bringup_two_threads() {
    let _ = rr_push(ThreadStub::new(8));
    let _ = rr_push(ThreadStub::new(8));
}

/// Placeholder until real thread records participate in scheduling.
pub fn yield_stub() {
    rr_bringup_two_threads();
}

/// Advance the bring-up RR cursor and pause briefly so another runnable (or ISR) can run.
/// Used by [`crate::subsystems::win32::msg_dispatch`] instead of a tight spin on empty queues.
pub fn yield_message_wait() {
    let n = RR_LEN.load(Ordering::Acquire);
    if n > 1 {
        RR_INDEX.fetch_add(1, Ordering::Relaxed);
    }
    core::hint::spin_loop();
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack));
    }
}

/// Cooperative “block” for message waits: run any queued BSP DPCs (timer may enqueue work), then
/// [`yield_message_wait`]. Single-thread bring-up still progresses without a tight `spin_loop` only.
pub fn block_cooperative_idle() {
    #[cfg(target_arch = "x86_64")]
    {
        crate::ke::dpc::bsp_drain_pending();
    }
    yield_message_wait();
}

/// x86_64: PIC + PIT + IDT vector 32 + `sti`. Other arches: [`yield_stub`] only.
pub fn bringup_timer_and_idle<H: Hal + ?Sized>(hal: &H) {
    yield_stub();
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            crate::hal::x86_64::pic::remap_all_masked();
            crate::ke::dpc::bsp_enqueue_dpc(core::ptr::addr_of_mut!(SAMPLE_DPC));
            let addr = crate::arch::x86_64::isr::timer_irq_entry_addr();
            crate::arch::x86_64::idt::set_interrupt_gate(32, addr);
            let lapic = crate::hal::x86_64::apic::try_init_bsp_timer(32, 0x2_0000);
            if !lapic {
                crate::hal::x86_64::pic::unmask_master_irq0();
                crate::hal::x86_64::pit::init_channel0_periodic(11932);
                log_line_hal(hal, SUB_KE, b"PIT+PIC IRQ0 timer (vector 32)");
            } else {
                log_line_hal(hal, SUB_KE, b"LAPIC periodic timer (vector 32)");
            }
            log_line_hal(
                hal,
                SUB_KE,
                b"RR bring-up: software slice = RR_TICKS_PER_SCHED_SLICE timer ticks per cursor step",
            );
            crate::ke::apc::enqueue_bringup_sample();
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = hal;
    }
}

#[cfg(test)]
mod quantum_tests {
    use super::*;

    #[test]
    fn rr_slice_period_matches_constant() {
        assert!(!rr_should_rotate_at_tick(0));
        assert!(!rr_should_rotate_at_tick(1));
        assert!(!rr_should_rotate_at_tick(3));
        assert!(rr_should_rotate_at_tick(4));
        assert!(rr_should_rotate_at_tick(8));
    }
}
