//! Minimal dual-process ALPC path (single kernel): two [`crate::ps::process::ProcessId`] values and
//! paired bounded queues (`client -> server`, `server -> client`).

use crate::ps::process::ProcessId;

use super::message::AlpcInlineMessage;
use super::port::AlpcPort;

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

/// Placeholder for hardware-isolated ALPC: copy `src` into a target address space (`target_cr3`).
pub fn post_cross_address_space(_target_cr3: u64, _src: &[u8]) -> Result<(), ()> {
    Err(())
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
}
