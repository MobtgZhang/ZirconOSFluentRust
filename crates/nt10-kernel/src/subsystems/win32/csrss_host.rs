//! In-kernel CSRSS stub host: bounded rings for [`super::csrss_proto::CsrMessageEnvelope`] and ACK generation.
//! **Target architecture:** a real `csrss.exe` user process speaking ALPC to a slim kernel façade.
//!
//! **Phase 6:** WinSta/Desktop objects move to Ring-3 csrss while syscalls stay in-kernel; until then
//! [`crate::servers::smss::NT10_PHASE6_RING3_CSRSS_FALLBACK_TO_KERNEL_HOST`] keeps this path for QEMU/CI
//! (see `docs/cn/Phase6-Routing.md`).

/// `false` = kernel host still registers desktops; `true` = csrss owns them (future).
pub const PHASE6_CSRSS_OWNS_WINSTA_IN_RING3: bool = false;

#[cfg(target_arch = "x86_64")]
use core::ptr::NonNull;
#[cfg(target_arch = "x86_64")]
use core::sync::atomic::{AtomicBool, Ordering};

use crate::ke::spinlock::SpinLock;
use super::csrss_proto::{
    CsrMessageEnvelope, CSR_CONNECT, CSR_CREATE_DESKTOP, CSR_CREATE_WINSTA, CSR_GET_MESSAGE,
    CSR_OPEN_WINSTA, CSR_SET_PROCESS_WINSTA,
};
use super::register::{self, Win32SubsystemState};

const RING_CAP: usize = 8;

struct MsgRing {
    buf: [CsrMessageEnvelope; RING_CAP],
    head: u8,
    len: u8,
}

impl MsgRing {
    const fn new() -> Self {
        Self {
            buf: [CsrMessageEnvelope::empty(0); RING_CAP],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, m: CsrMessageEnvelope) -> Result<(), ()> {
        if self.len as usize >= RING_CAP {
            return Err(());
        }
        let idx = (self.head as usize + self.len as usize) % RING_CAP;
        self.buf[idx] = m;
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<CsrMessageEnvelope> {
        if self.len == 0 {
            return None;
        }
        let m = self.buf[self.head as usize % RING_CAP];
        self.head = self.head.wrapping_add(1);
        self.len -= 1;
        Some(m)
    }
}

static CSR_CLIENT_TO_HOST: SpinLock<MsgRing> = SpinLock::new(MsgRing::new());
static CSR_HOST_TO_CLIENT: SpinLock<MsgRing> = SpinLock::new(MsgRing::new());
static CSR_LAST_ACK: SpinLock<Option<CsrMessageEnvelope>> = SpinLock::new(None);

/// Client → host queue (ALPC-style bring-up).
pub fn post_from_client(msg: CsrMessageEnvelope) -> Result<(), ()> {
    CSR_CLIENT_TO_HOST.lock().push(msg)
}

/// Host injects a message visible to [`poll_get_message`] (e.g. [`CSR_SERVER_TICK`]).
pub fn inject_server_visible(msg: CsrMessageEnvelope) -> Result<(), ()> {
    CSR_HOST_TO_CLIENT.lock().push(msg)
}

/// Last ACK produced by [`pump_one`]; consume once per processed request.
pub fn take_last_ack() -> Option<CsrMessageEnvelope> {
    CSR_LAST_ACK.lock().take()
}

/// Dequeue one client request, dispatch, store ACK. Returns `true` if work was done.
pub fn pump_one() -> bool {
    let req = { CSR_CLIENT_TO_HOST.lock().pop() };
    let Some(req) = req else {
        return false;
    };
    let ack = dispatch(&req);
    *CSR_LAST_ACK.lock() = Some(ack);
    true
}

fn dispatch(req: &CsrMessageEnvelope) -> CsrMessageEnvelope {
    match req.opcode {
        CSR_CONNECT => {
            register::win32_subsystem_begin_connect();
            register::win32_subsystem_mark_ready();
            CsrMessageEnvelope::ack_ok(req.opcode)
        }
        CSR_CREATE_WINSTA
        | CSR_CREATE_DESKTOP
        | CSR_OPEN_WINSTA
        | CSR_SET_PROCESS_WINSTA => CsrMessageEnvelope::ack_ok(req.opcode),
        CSR_GET_MESSAGE => {
            let mut ack = CsrMessageEnvelope::ack_ok(CSR_GET_MESSAGE);
            if let Some(pending) = CSR_HOST_TO_CLIENT.lock().pop() {
                let n = (pending.payload_len as usize).min(59);
                ack.payload[0] = 1;
                ack.payload[1..5].copy_from_slice(&pending.opcode.to_le_bytes());
                ack.payload[5..5 + n].copy_from_slice(&pending.payload[..n]);
                ack.payload_len = (5 + n) as u32;
            } else {
                ack.payload[0] = 0;
                ack.payload_len = 1;
            }
            ack
        }
        _ => CsrMessageEnvelope::ack_error(req.opcode),
    }
}

/// `CSR_GET_MESSAGE`-style poll: when subsystem is [`Win32SubsystemState::Ready`], returns one host-visible message if any.
pub fn poll_get_message(_timeout_spins: u32) -> Option<CsrMessageEnvelope> {
    if register::win32_subsystem_state() != Win32SubsystemState::Ready {
        return None;
    }
    CSR_HOST_TO_CLIENT.lock().pop()
}

#[cfg(target_arch = "x86_64")]
static PHASE3_WNDPROC_HIT: AtomicBool = AtomicBool::new(false);

#[cfg(target_arch = "x86_64")]
fn phase3_probe_wndproc(
    hwnd: crate::libs::win32_abi::Hwnd,
    msg: u32,
    wp: crate::libs::win32_abi::WParam,
    lp: crate::libs::win32_abi::LParam,
) -> crate::libs::win32_abi::LResult {
    if msg == super::windowing::wm::WM_USER {
        PHASE3_WNDPROC_HIT.store(true, Ordering::SeqCst);
    }
    super::windowing::def_window_proc_bringup(hwnd, msg, wp, lp)
}

/// Serial-only Phase 3 acceptance (CreateWindow → Post WM_USER → GetMessage → Dispatch); x86_64 only.
#[cfg(target_arch = "x86_64")]
pub fn phase3_message_pump_serial_smoke() {
    use super::msg_dispatch;
    use super::windowing::{create_window_ex_on_desktop, register_class_ex_bringup};
    use crate::ob::winsta::DesktopObject;
    use crate::rtl::log::{log_line_serial, SUB_SUBS};

    PHASE3_WNDPROC_HIT.store(false, Ordering::SeqCst);
    log_line_serial(SUB_SUBS, b"Phase3 msg pump smoke begin");
    let mut desktop = DesktopObject::new();
    let dptr = NonNull::from(&mut desktop);
    let tid = 1u32;
    msg_dispatch::set_current_thread_for_win32(tid);
    msg_dispatch::thread_bind_desktop(tid, dptr);
    let Ok(atom) = register_class_ex_bringup(0, 0x70) else {
        log_line_serial(SUB_SUBS, b"Phase3 msg pump smoke FAIL (class)");
        return;
    };
    let Ok(hwnd) = create_window_ex_on_desktop(
        unsafe { dptr.as_ref() },
        atom,
        0,
        tid,
        phase3_probe_wndproc,
    ) else {
        log_line_serial(SUB_SUBS, b"Phase3 msg pump smoke FAIL (create)");
        return;
    };
    if msg_dispatch::phase3_message_pump_integration(tid, unsafe { dptr.as_ref() }, hwnd).is_err() {
        log_line_serial(SUB_SUBS, b"Phase3 msg pump smoke FAIL (integration)");
        return;
    }
    if !PHASE3_WNDPROC_HIT.load(Ordering::SeqCst) {
        log_line_serial(SUB_SUBS, b"Phase3 msg pump smoke FAIL (wndproc)");
        return;
    }
    log_line_serial(SUB_SUBS, b"Phase3 WndProc dispatched");
    log_line_serial(SUB_SUBS, b"Phase3 msg pump smoke OK");
}

/// Phase 4: offscreen surface + bitmap text + software composite into a mock framebuffer (x86_64 serial).
#[cfg(target_arch = "x86_64")]
pub fn phase4_compositor_serial_smoke() {
    use super::compositor;
    use super::msg_dispatch;
    use super::text_bringup::text_out_ascii;
    use super::windowing::{create_window_ex_on_desktop, register_class_ex_bringup};
    use crate::ob::winsta::DesktopObject;
    use crate::rtl::log::{log_line_serial, SUB_SUBS};

    log_line_serial(SUB_SUBS, b"Phase4 compositor smoke begin");
    let mut desktop = DesktopObject::new();
    let dptr = NonNull::from(&mut desktop);
    let tid = 1u32;
    msg_dispatch::set_current_thread_for_win32(tid);
    msg_dispatch::thread_bind_desktop(tid, dptr);
    let Ok(atom) = register_class_ex_bringup(0, 0x71) else {
        log_line_serial(SUB_SUBS, b"Phase4 compositor smoke FAIL (class)");
        return;
    };
    let Ok(hwnd) = create_window_ex_on_desktop(
        unsafe { dptr.as_ref() },
        atom,
        0,
        tid,
        super::windowing::def_window_proc_bringup,
    ) else {
        log_line_serial(SUB_SUBS, b"Phase4 compositor smoke FAIL (create)");
        return;
    };
    let Some(si) = desktop.hwnd_slot_index(hwnd) else {
        log_line_serial(SUB_SUBS, b"Phase4 compositor smoke FAIL (slot)");
        return;
    };
    super::window_surface::fill_surface_solid(si as usize, [40, 80, 120, 255]);
    text_out_ascii(si as usize, 4, 4, b"Phase4", [255, 255, 255, 255]);
    let mut fb = [0u8; 256 * 64 * 4];
    if compositor::composite_desktop_to_framebuffer(
        unsafe { dptr.as_ref() },
        &mut fb,
        256,
        64,
        256,
        0,
        8,
    )
    .is_err()
        || !fb.iter().any(|&b| b != 0)
    {
        log_line_serial(SUB_SUBS, b"Phase4 compositor smoke FAIL (composite)");
        return;
    }
    log_line_serial(SUB_SUBS, b"Phase4 compositor smoke OK");
}

/// Bring-up: Win32 syscalls, CSR connect, Phase 3/4 serial smoke (x86_64).
pub fn bringup_kernel_thread_smoke() {
    super::syscall_win32::register_win32_syscalls_bringup();
    let _ = post_from_client(CsrMessageEnvelope::empty(CSR_CONNECT));
    let _ = pump_one();
    #[cfg(target_arch = "x86_64")]
    {
        phase3_message_pump_serial_smoke();
        phase4_compositor_serial_smoke();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::csrss_proto::{CSR_ACK_OK, CSR_GET_MESSAGE, CSR_SERVER_TICK};

    #[test]
    fn connect_marks_ready_and_ack() {
        register::win32_subsystem_disconnect();
        let _ = post_from_client(CsrMessageEnvelope::empty(CSR_CONNECT));
        assert!(pump_one());
        let ack = take_last_ack().expect("ack");
        assert_eq!(ack.opcode, CSR_ACK_OK);
        assert_eq!(register::win32_subsystem_state(), register::Win32SubsystemState::Ready);
    }

    #[test]
    fn get_message_delivers_pending_server_envelope() {
        register::win32_subsystem_disconnect();
        let _ = post_from_client(CsrMessageEnvelope::empty(CSR_CONNECT));
        let _ = pump_one();
        let _ = take_last_ack();
        let _ = inject_server_visible(CsrMessageEnvelope::empty(CSR_SERVER_TICK));
        let _ = post_from_client(CsrMessageEnvelope::empty(CSR_GET_MESSAGE));
        assert!(pump_one());
        let ack = take_last_ack().expect("ack");
        assert_eq!(ack.payload[0], 1);
    }
}
