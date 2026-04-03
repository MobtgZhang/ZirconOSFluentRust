//! Minimal dual-process ALPC path (single kernel): two [`crate::ps::process::ProcessId`] values and
//! paired bounded queues (`client -> server`, `server -> client`).

use core::sync::atomic::{AtomicU32, Ordering};

use crate::ke::spinlock::SpinLock;
use crate::ps::process::ProcessId;

use super::message::AlpcInlineMessage;
use super::port::AlpcPort;

const CROSS_AS_BOUNCE_CAP: usize = 4096;

#[inline]
#[must_use]
fn user_canonical_dest_va(va: u64) -> bool {
    va < 0x0000_8000_0000_0000
}
static CROSS_AS_BOUNCE: SpinLock<[u8; CROSS_AS_BOUNCE_CAP]> = SpinLock::new([0u8; CROSS_AS_BOUNCE_CAP]);
static CROSS_AS_BOUNCE_LEN: AtomicU32 = AtomicU32::new(0);

/// Two-way mailbox between a designated server process and client process (bring-up).
#[derive(Debug)]
pub struct AlpcDuplexLink {
    pub server_proc: ProcessId,
    pub client_proc: ProcessId,
    server_inbound: AlpcPort,
    client_inbound: AlpcPort,
}

impl AlpcDuplexLink {
    #[must_use]
    pub fn new(server_proc: ProcessId, client_proc: ProcessId) -> Self {
        Self {
            server_proc,
            client_proc,
            server_inbound: AlpcPort::new(),
            client_inbound: AlpcPort::new(),
        }
    }

    #[must_use]
    pub fn server_port_id(&self) -> super::port::AlpcPortId {
        self.server_inbound.id
    }

    #[must_use]
    pub fn client_side_port_id(&self) -> super::port::AlpcPortId {
        self.client_inbound.id
    }

    /// Client sends to server's receive queue.
    pub fn post_from_client(&mut self, from: ProcessId, payload: &[u8]) -> Result<(), ()> {
        if from != self.client_proc {
            return Err(());
        }
        self.server_inbound.try_send(payload)
    }

    /// Server sends to client's receive queue.
    pub fn post_from_server(&mut self, from: ProcessId, payload: &[u8]) -> Result<(), ()> {
        if from != self.server_proc {
            return Err(());
        }
        self.client_inbound.try_send(payload)
    }

    pub fn recv_at_server(&mut self) -> Result<AlpcInlineMessage, ()> {
        self.server_inbound.try_recv()
    }

    pub fn recv_at_client(&mut self) -> Result<AlpcInlineMessage, ()> {
        self.client_inbound.try_recv()
    }
}

/// Copy `src` into `remote_user_dest` while the CPU uses `remote_cr3` (x86_64 bring-up).
///
/// `src` is read under the **current** CR3; data is staged in the kernel bounce buffer, then copied
/// to the remote VA after a temporary `CR3` switch. Caller must ensure `remote_user_dest` is writable
/// in the remote map and that the bounce lock is not contended across CPUs.
///
/// Unavailable in `cfg(test)` host builds (returns [`Err`]); use [`post_cross_address_space`] for bounce-only tests.
#[cfg(all(target_arch = "x86_64", not(test)))]
pub fn post_cross_address_space_into_remote_va(
    remote_cr3: u64,
    remote_user_dest: u64,
    src: &[u8],
) -> Result<(), ()> {
    if src.is_empty() {
        return Ok(());
    }
    if src.len() > CROSS_AS_BOUNCE_CAP || remote_cr3 == 0 {
        return Err(());
    }
    if !user_canonical_dest_va(remote_user_dest) {
        return Err(());
    }
    let mut g = CROSS_AS_BOUNCE.lock();
    g[..src.len()].copy_from_slice(src);
    let n = src.len();
    let p = g.as_mut_ptr();
    unsafe {
        let old = crate::arch::x86_64::paging::read_cr3();
        crate::arch::x86_64::paging::write_cr3(remote_cr3);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        core::ptr::copy_nonoverlapping(p, remote_user_dest as *mut u8, n);
        crate::arch::x86_64::paging::write_cr3(old);
    }
    Ok(())
}

#[cfg(any(not(target_arch = "x86_64"), test))]
pub fn post_cross_address_space_into_remote_va(
    _remote_cr3: u64,
    remote_user_dest: u64,
    src: &[u8],
) -> Result<(), ()> {
    if !src.is_empty() && !user_canonical_dest_va(remote_user_dest) {
        return Err(());
    }
    if !src.is_empty() {
        return Err(());
    }
    Ok(())
}

/// Copy `src` into a kernel bounce buffer when the target matches the running address space.
///
/// - `target_cr3 == 0`: always accepted (bring-up “current / kernel” path).
/// - x86_64: `target_cr3 == read_cr3()` is accepted.
/// - Any other `target_cr3`: returns [`Err`] until per-process page-table switching is wired.
pub fn post_cross_address_space(target_cr3: u64, src: &[u8]) -> Result<(), ()> {
    if src.is_empty() {
        CROSS_AS_BOUNCE_LEN.store(0, Ordering::Release);
        return Ok(());
    }
    if src.len() > CROSS_AS_BOUNCE_CAP {
        return Err(());
    }
    let ok = if target_cr3 == 0 {
        true
    } else if cfg!(test) {
        // Host `cargo test` must not execute `read_cr3` (invalid outside the bare-metal kernel).
        false
    } else {
        #[cfg(target_arch = "x86_64")]
        {
            target_cr3 == crate::arch::x86_64::paging::read_cr3()
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = target_cr3;
            false
        }
    };
    if !ok {
        return Err(());
    }
    let mut g = CROSS_AS_BOUNCE.lock();
    g[..src.len()].copy_from_slice(src);
    CROSS_AS_BOUNCE_LEN.store(src.len() as u32, Ordering::Release);
    Ok(())
}

/// Test / diagnostics: last successful [`post_cross_address_space`] payload length.
#[cfg(test)]
pub fn cross_as_bounce_len_for_test() -> usize {
    CROSS_AS_BOUNCE_LEN.load(Ordering::Acquire) as usize
}

/// Test / diagnostics: snapshot bounce bytes (length from [`cross_as_bounce_len_for_test`]).
#[cfg(test)]
pub fn cross_as_bounce_bytes_for_test(out: &mut [u8]) -> usize {
    let n = cross_as_bounce_len_for_test().min(out.len()).min(CROSS_AS_BOUNCE_CAP);
    let g = CROSS_AS_BOUNCE.lock();
    out[..n].copy_from_slice(&g[..n]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps::process::EProcess;

    #[test]
    fn duplex_round_trip() {
        let mut srv = EProcess::new_bootstrap();
        let mut cli = EProcess::new_bootstrap();
        let sp = srv.pid;
        let cp = cli.pid;
        let _ = (&mut srv, &mut cli);
        let mut link = AlpcDuplexLink::new(sp, cp);
        assert!(link.post_from_client(cp, b"hello").is_ok());
        let m = link.recv_at_server().unwrap();
        assert_eq!(m.len as usize, 5);
        assert_eq!(&m.data[..5], b"hello");
        assert!(link.post_from_server(sp, b"ack").is_ok());
        let r = link.recv_at_client().unwrap();
        assert_eq!(&r.data[..3], b"ack");
    }

    #[test]
    fn wrong_sender_rejected() {
        let sp = ProcessId(100);
        let cp = ProcessId(200);
        let mut link = AlpcDuplexLink::new(sp, cp);
        assert!(link.post_from_client(sp, b"x").is_err());
        assert!(link.post_from_server(cp, b"x").is_err());
    }

    #[test]
    fn cross_address_space_bounce_zero_cr3() {
        assert!(post_cross_address_space(0, b"alpc-bounce").is_ok());
        assert_eq!(cross_as_bounce_len_for_test(), 11);
        let mut t = [0u8; 16];
        let n = cross_as_bounce_bytes_for_test(&mut t);
        assert_eq!(n, 11);
        assert_eq!(&t[..11], b"alpc-bounce");
    }

    #[test]
    fn cross_address_space_rejects_unknown_cr3() {
        assert!(post_cross_address_space(u64::MAX, b"x").is_err());
    }

    #[test]
    fn cross_into_remote_rejects_non_canonical_user_va_stub() {
        assert!(post_cross_address_space_into_remote_va(1, 0xFFFF_8000_0000_0000, b"x").is_err());
    }
}
