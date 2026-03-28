//! In-kernel CSRSS stub host: bounded rings for [`super::csrss_proto::CsrMessageEnvelope`] and ACK generation.

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

/// Bring-up: enqueue connect and drain one request (marks subsystem ready).
pub fn bringup_kernel_thread_smoke() {
    let _ = post_from_client(CsrMessageEnvelope::empty(CSR_CONNECT));
    let _ = pump_one();
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
