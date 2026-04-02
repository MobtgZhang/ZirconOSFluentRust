//! user32 — message pump façade (kernel stubs).
//!
//! Post vs send: bring-up uses only **posted** messages ([`post_message_zr`]); synchronous
//! `SendMessage` waits on another thread are not modeled until a real Win32k scheduler exists.

use crate::ke::spinlock::SpinLock;
use crate::libs::win32_abi::{Hwnd, LParam, LResult, WParam};
use super::csrss_host;
use super::msg_dispatch;
use super::csrss_proto::{
    CsrConnectMsg, CsrMessageEnvelope, CSR_GET_MESSAGE, CSR_SERVER_TICK,
};
use super::register::{self, Win32SubsystemState};
pub use super::windowing::{create_window_ex_bringup, def_window_proc_bringup, register_class_ex_bringup, wm};

const USER32_Q_CAP: usize = 32;

#[derive(Clone, Copy, Debug, Default)]
pub struct ZrUser32Message {
    pub hwnd: Hwnd,
    pub msg: u32,
    pub wparam: u64,
    pub lparam: LParam,
}

impl ZrUser32Message {
    pub const fn zero() -> Self {
        Self {
            hwnd: 0,
            msg: 0,
            wparam: 0,
            lparam: 0,
        }
    }
}

struct User32Ring {
    buf: [ZrUser32Message; USER32_Q_CAP],
    head: u8,
    len: u8,
}

impl User32Ring {
    const fn new() -> Self {
        Self {
            buf: [ZrUser32Message::zero(); USER32_Q_CAP],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, m: ZrUser32Message) -> Result<(), ()> {
        if self.len as usize >= USER32_Q_CAP {
            return Err(());
        }
        let idx = (self.head as usize + self.len as usize) % USER32_Q_CAP;
        self.buf[idx] = m;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<ZrUser32Message> {
        if self.len == 0 {
            return None;
        }
        let m = self.buf[self.head as usize % USER32_Q_CAP];
        self.head = self.head.wrapping_add(1);
        self.len -= 1;
        Some(m)
    }
}

static USER32_QUEUE: SpinLock<User32Ring> = SpinLock::new(User32Ring::new());

/// Post a message: when the current thread has a bound desktop, routes via
/// [`msg_dispatch::post_message_kernel`] (same path as Win32 syscalls); otherwise falls back to the
/// legacy process-wide ring buffer.
pub fn post_message_zr(m: ZrUser32Message) -> Result<(), ()> {
    let tid = msg_dispatch::current_thread_for_win32();
    if let Some(dptr) = msg_dispatch::thread_desktop_ptr(tid) {
        return msg_dispatch::post_message_kernel(
            unsafe { dptr.as_ref() },
            m.hwnd,
            m.msg,
            m.wparam,
            m.lparam,
        );
    }
    USER32_QUEUE.lock().push(m)
}

#[must_use]
pub fn peek_pop_message_zr() -> Option<ZrUser32Message> {
    USER32_QUEUE.lock().pop()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MsgPumpStub {
    pub pending: u32,
}

impl MsgPumpStub {
    #[must_use]
    pub const fn new() -> Self {
        Self { pending: 0 }
    }

    /// CSRSS-style poll: when subsystem is ready, drains CSR host-visible queue and local FIFO.
    pub fn get_message(&mut self) -> bool {
        if register::win32_subsystem_state() != Win32SubsystemState::Ready {
            return false;
        }
        if let Some(tick) = csrss_host::poll_get_message(0) {
            if tick.opcode == CSR_SERVER_TICK {
                return true;
            }
        }
        let get = CsrMessageEnvelope::empty(CSR_GET_MESSAGE);
        let _ = csrss_host::post_from_client(get);
        let _ = csrss_host::pump_one();
        if let Some(ack) = csrss_host::take_last_ack() {
            if ack.payload_len > 0 && ack.payload[0] == 1 {
                return true;
            }
        }
        let tid = msg_dispatch::current_thread_for_win32();
        if msg_dispatch::try_get_message_kernel(tid).is_some() {
            return true;
        }
        if let Some(_m) = peek_pop_message_zr() {
            return true;
        }
        if self.pending > 0 {
            self.pending -= 1;
            return true;
        }
        false
    }

    #[must_use]
    pub fn connect_token(pid: u64) -> CsrConnectMsg {
        CsrConnectMsg::new(pid)
    }
}

/// Placeholder for `TranslateAcceleratorW` — returns `false` (no translation).
#[inline]
pub fn translate_accelerator_stub(_hwnd: Hwnd, _table: u64, _msg: &mut ZrUser32Message) -> bool {
    false
}

/// Dispatch to [`def_window_proc_bringup`] when the app returns `0` / default.
#[inline]
pub fn dispatch_default_bringup(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    def_window_proc_bringup(hwnd, msg, wp, lp)
}
