//! Per-thread posted message queues, desktop HWND routing, and cooperative GetMessage wait.
//!
//! Bring-up: fixed thread slots (`tid % N`); real CSRSS would attach queues to ETHREAD.

use core::ptr::NonNull;
use core::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use crate::ke::msg_wait::MsgWaitGen;
use crate::ke::spinlock::SpinLock;
use crate::libs::win32_abi::{Hwnd, LParam, LResult, WParam};
use crate::ob::winsta::DesktopObject;

/// Syscall / bring-up numbers (not Windows NT API numbers).
pub const ZR_SYSCALL_CREATE_WINDOW_EX: u16 = 0x100;
pub const ZR_SYSCALL_POST_MESSAGE: u16 = 0x101;
pub const ZR_SYSCALL_GET_MESSAGE: u16 = 0x102;
pub const ZR_SYSCALL_DISPATCH_MESSAGE: u16 = 0x103;
pub const ZR_SYSCALL_SEND_MESSAGE: u16 = 0x104;

const THREAD_SLOTS: usize = 8;
const QUEUE_CAP: usize = 32;

/// Posted message (kernel copy of user32 MSG subset).
#[derive(Clone, Copy, Debug)]
pub struct PostedMessageKernel {
    pub hwnd: Hwnd,
    pub msg: u32,
    pub wparam: WParam,
    pub lparam: LParam,
    pub time: u32,
}

struct ThreadMsgSlot {
    desktop_ptr: AtomicUsize,
    teb_user_va: AtomicU64,
    queue: [PostedMessageKernel; QUEUE_CAP],
    head: u8,
    len: u8,
    wait: MsgWaitGen,
    /// Cross-thread `SendMessage`: receiver dequeues and dispatches in [`process_pending_sends`].
    pending_cross_send: SpinLock<Option<(PostedMessageKernel, u32)>>,
    /// Woken when a cross-thread send this thread issued has been answered.
    send_reply_wake: MsgWaitGen,
    /// Truncated [`LResult`] from the last answered cross-thread send (bring-up).
    send_reply_result: AtomicI64,
}

impl ThreadMsgSlot {
    const fn new() -> Self {
        Self {
            desktop_ptr: AtomicUsize::new(0),
            teb_user_va: AtomicU64::new(0),
            queue: [PostedMessageKernel {
                hwnd: 0,
                msg: 0,
                wparam: 0,
                lparam: 0,
                time: 0,
            }; QUEUE_CAP],
            head: 0,
            len: 0,
            wait: MsgWaitGen::new(),
            pending_cross_send: SpinLock::new(None),
            send_reply_wake: MsgWaitGen::new(),
            send_reply_result: AtomicI64::new(0),
        }
    }

    fn push(&mut self, m: PostedMessageKernel) -> Result<(), ()> {
        if self.len as usize >= QUEUE_CAP {
            return Err(());
        }
        let idx = (self.head as usize + self.len as usize) % QUEUE_CAP;
        self.queue[idx] = m;
        self.len += 1;
        self.wait.wake_one();
        Ok(())
    }

    fn pop(&mut self) -> Option<PostedMessageKernel> {
        if self.len == 0 {
            return None;
        }
        let m = self.queue[self.head as usize % QUEUE_CAP];
        self.head = self.head.wrapping_add(1);
        self.len -= 1;
        Some(m)
    }
}

static SLOTS: [SpinLock<ThreadMsgSlot>; THREAD_SLOTS] =
    [const { SpinLock::new(ThreadMsgSlot::new()) }; THREAD_SLOTS];

static BRINGUP_CURRENT_TID: AtomicU32 = AtomicU32::new(1);

static LAST_SYSCALL_MSG: SpinLock<Option<PostedMessageKernel>> = SpinLock::new(None);

#[inline]
fn slot_index(tid: u32) -> usize {
    (tid as usize) % THREAD_SLOTS
}

/// Expected thread for Win32 bring-up (CSRSS path, syscall path, tests).
pub fn set_current_thread_for_win32(tid: u32) {
    BRINGUP_CURRENT_TID.store(tid, Ordering::Release);
}

#[must_use]
pub fn current_thread_for_win32() -> u32 {
    BRINGUP_CURRENT_TID.load(Ordering::Relaxed)
}

/// Bind `tid` to a desktop object (kernel address stored as truncated `u32` — bring-up low memory only).
pub fn thread_bind_desktop(tid: u32, desktop: NonNull<DesktopObject>) {
    thread_bind_win32(tid, desktop, 0);
}

/// Desktop + optional user TEB base for Win32 routing (mirrors [`crate::ps::thread::EThread`] fields).
pub fn thread_bind_win32(tid: u32, desktop: NonNull<DesktopObject>, teb_user_va: u64) {
    let p = desktop.as_ptr() as usize;
    let s = SLOTS[slot_index(tid)].lock();
    s.desktop_ptr.store(p, Ordering::Release);
    s.teb_user_va.store(teb_user_va, Ordering::Release);
}

/// Push [`crate::ps::thread::EThread`] Win32 routing into the per-tid slot.
pub fn apply_ethread_routing(e: &crate::ps::thread::EThread) {
    let Some(p) = NonNull::new(e.desktop_kernel_ptr as *mut DesktopObject) else {
        return;
    };
    thread_bind_win32(e.tid.0, p, e.teb_user_va);
}

#[must_use]
pub fn thread_desktop_ptr(tid: u32) -> Option<NonNull<DesktopObject>> {
    let v = SLOTS[slot_index(tid)]
        .lock()
        .desktop_ptr
        .load(Ordering::Acquire);
    NonNull::new(v as *mut DesktopObject)
}

#[must_use]
pub fn thread_teb_user_va(tid: u32) -> u64 {
    SLOTS[slot_index(tid)]
        .lock()
        .teb_user_va
        .load(Ordering::Acquire)
}

/// Last message retrieved by [`get_message_wait_kernel`] / syscall GET_MESSAGE (single-threaded bring-up).
pub fn take_last_get_message() -> Option<PostedMessageKernel> {
    LAST_SYSCALL_MSG.lock().take()
}

fn push_for_tid(tid: u32, m: PostedMessageKernel) -> Result<(), ()> {
    SLOTS[slot_index(tid)].lock().push(m)
}

/// Drain at most one inbound cross-thread `SendMessage` for `tid` and reply to the caller.
fn process_pending_sends(tid: u32) {
    let pending = {
        let outer = SLOTS[slot_index(tid)].lock();
        let p = {
            let mut inner = outer.pending_cross_send.lock();
            inner.take()
        };
        p
    };
    let Some((m, from_tid)) = pending else {
        return;
    };
    let Some(dptr) = thread_desktop_ptr(tid) else {
        return;
    };
    let desktop = unsafe { dptr.as_ref() };
    let r = dispatch_message_kernel(desktop, m);
    let ss = SLOTS[slot_index(from_tid)].lock();
    ss.send_reply_result.store(r, Ordering::Release);
    ss.send_reply_wake.wake_one();
}

/// Synchronous send: same-thread calls `WndProc` inline; cross-thread queues to owner and blocks until
/// the owner thread runs [`get_message_wait_kernel`] (or [`process_pending_sends`] on that tid).
pub fn send_message_kernel(
    sender_tid: u32,
    desktop: &DesktopObject,
    hwnd: Hwnd,
    msg: u32,
    wparam: WParam,
    lparam: LParam,
) -> Result<LResult, ()> {
    let Some((owner, proc)) = desktop.lookup_hwnd(hwnd) else {
        return Err(());
    };
    if owner == sender_tid {
        return Ok(proc(hwnd, msg, wparam, lparam));
    }
    let time = crate::ke::sched::timer_quanta();
    let pm = PostedMessageKernel {
        hwnd,
        msg,
        wparam,
        lparam,
        time,
    };
    let gen = SLOTS[slot_index(sender_tid)]
        .lock()
        .send_reply_wake
        .current();
    {
        let ro = SLOTS[slot_index(owner)].lock();
        let mut p = ro.pending_cross_send.lock();
        if p.is_some() {
            return Err(());
        }
        *p = Some((pm, sender_tid));
    }
    SLOTS[slot_index(owner)].lock().wait.wake_one();
    {
        let s = SLOTS[slot_index(sender_tid)].lock();
        s.send_reply_wake.wait_until_changed(gen);
    }
    Ok(
        SLOTS[slot_index(sender_tid)]
            .lock()
            .send_reply_result
            .load(Ordering::Acquire),
    )
}

/// Route `PostMessage` to the owning thread queue; bumps desktop posted counter.
pub fn post_message_kernel(
    desktop: &DesktopObject,
    hwnd: Hwnd,
    msg: u32,
    wparam: WParam,
    lparam: LParam,
) -> Result<(), ()> {
    let Some((owner_tid, _)) = desktop.lookup_hwnd(hwnd) else {
        return Err(());
    };
    let time = crate::ke::sched::timer_quanta();
    push_for_tid(
        owner_tid,
        PostedMessageKernel {
            hwnd,
            msg,
            wparam,
            lparam,
            time,
        },
    )?;
    desktop
        .posted_message_count
        .fetch_add(1, Ordering::Relaxed);
    Ok(())
}

/// Pop one posted message for `tid`, or block cooperatively until one arrives.
///
/// Blocking uses [`MsgWaitGen::wait_until_changed`] → [`crate::ke::sched::block_cooperative_idle`]
/// (DPC drain + RR yield + `pause`), not a bare tight spin.
pub fn get_message_wait_kernel(tid: u32) -> PostedMessageKernel {
    loop {
        process_pending_sends(tid);
        let gen = {
            let mut g = SLOTS[slot_index(tid)].lock();
            if let Some(m) = g.pop() {
                *LAST_SYSCALL_MSG.lock() = Some(m);
                return m;
            }
            g.wait.current()
        };
        SLOTS[slot_index(tid)]
            .lock()
            .wait
            .wait_until_changed(gen);
    }
}

/// Non-blocking peek/pop for pump stubs.
#[must_use]
pub fn try_get_message_kernel(tid: u32) -> Option<PostedMessageKernel> {
    process_pending_sends(tid);
    let m = SLOTS[slot_index(tid)].lock().pop();
    if let Some(mm) = m {
        *LAST_SYSCALL_MSG.lock() = Some(mm);
        return Some(mm);
    }
    None
}

/// Invoke registered window procedure for the message.
#[must_use]
pub fn dispatch_message_kernel(desktop: &DesktopObject, m: PostedMessageKernel) -> LResult {
    let Some((_owner, proc)) = desktop.lookup_hwnd(m.hwnd) else {
        return 0;
    };
    proc(m.hwnd, m.msg, m.wparam, m.lparam)
}

/// End-to-end bring-up check (host tests + optional serial smoke).
pub fn phase3_message_pump_integration(tid: u32, desktop: &DesktopObject, hwnd: Hwnd) -> Result<(), ()> {
    set_current_thread_for_win32(tid);
    post_message_kernel(desktop, hwnd, super::windowing::wm::WM_USER, 0x55, 0x66)?;
    let m = get_message_wait_kernel(tid);
    if m.msg != super::windowing::wm::WM_USER {
        return Err(());
    }
    let _ = dispatch_message_kernel(desktop, m);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ob::winsta::DesktopObject;
    use crate::subsystems::win32::windowing::{create_window_ex_on_desktop, register_class_ex_bringup};

    #[test]
    fn post_get_dispatch_wm_user() {
        use crate::ke::sched::ThreadId;
        use crate::ps::process::ProcessId;
        use crate::ps::thread::EThread;

        let tid = 7u32;
        set_current_thread_for_win32(tid);
        let mut desktop = DesktopObject::new();
        let dptr = core::ptr::NonNull::from(&mut desktop);
        let ethread = EThread::new_system_thread(ProcessId(1), ThreadId(tid))
            .with_win32_routing(0x0000_7ffe_0000, dptr.as_ptr() as usize);
        apply_ethread_routing(&ethread);
        assert_eq!(thread_teb_user_va(tid), 0x0000_7ffe_0000);
        let atom = register_class_ex_bringup(0, 0x41).expect("class");
        let hwnd = create_window_ex_on_desktop(
            unsafe { dptr.as_ref() },
            atom,
            0,
            tid,
            crate::subsystems::win32::windowing::def_window_proc_bringup,
        )
        .expect("hwnd");
        phase3_message_pump_integration(tid, unsafe { dptr.as_ref() }, hwnd).expect("flow");
    }

    #[test]
    fn send_message_same_thread_inline_wndproc() {
        fn wnd(_hwnd: Hwnd, msg: u32, _wp: WParam, _lp: LParam) -> LResult {
            if msg == 0x999 {
                42
            } else {
                0
            }
        }
        let tid = 8u32;
        set_current_thread_for_win32(tid);
        let mut desktop = DesktopObject::new();
        let dptr = core::ptr::NonNull::from(&mut desktop);
        thread_bind_desktop(tid, dptr);
        let atom = register_class_ex_bringup(0, 0x52).expect("class");
        let hwnd = create_window_ex_on_desktop(unsafe { dptr.as_ref() }, atom, 0, tid, wnd).expect("hwnd");
        let r = send_message_kernel(tid, unsafe { dptr.as_ref() }, hwnd, 0x999, 0, 0).expect("send");
        assert_eq!(r, 42);
    }
}
