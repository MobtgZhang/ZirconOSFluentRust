//! CSRSS / ALPC message opcodes for the Win32 subsystem (ZirconOS-local numbering).

/// Max bytes in [`CsrMessageEnvelope::payload`] (fixed layout for bring-up).
pub const CSR_MESSAGE_PAYLOAD_CAP: usize = 64;

/// Connect to the Win32 API server port.
pub const CSR_CONNECT: u32 = 0x1000;
/// Allocate a window station (stub).
pub const CSR_CREATE_WINSTA: u32 = 0x1001;
/// Allocate a desktop (stub).
pub const CSR_CREATE_DESKTOP: u32 = 0x1002;
/// Open an existing window station by name (ALPC payload = UTF-16 or ANSI name; bring-up ignores).
pub const CSR_OPEN_WINSTA: u32 = 0x1005;
/// `SetProcessWindowStation` — attach calling client to named station (stub).
pub const CSR_SET_PROCESS_WINSTA: u32 = 0x1006;
/// Pump-style poll (stub).
pub const CSR_GET_MESSAGE: u32 = 0x1003;
/// Synthetic tick the host may enqueue for pump tests (ZirconOS-local).
pub const CSR_SERVER_TICK: u32 = 0x1004;

/// ACK: request completed without error (payload may echo correlation).
pub const CSR_ACK_OK: u32 = 0x1FF0;
/// ACK: inbound queue full or unknown opcode.
pub const CSR_ACK_ERROR: u32 = 0x1FF1;

/// Fixed envelope for ALPC-style fixed messages (kernel ↔ csrss bring-up).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CsrMessageEnvelope {
    pub opcode: u32,
    pub payload_len: u32,
    pub payload: [u8; CSR_MESSAGE_PAYLOAD_CAP],
}

impl CsrMessageEnvelope {
    pub const fn empty(op: u32) -> Self {
        Self {
            opcode: op,
            payload_len: 0,
            payload: [0u8; CSR_MESSAGE_PAYLOAD_CAP],
        }
    }

    /// ACK with `payload_len == 4` carrying `request_opcode` (LE) for bring-up tracing.
    #[must_use]
    pub const fn ack_ok(request_opcode: u32) -> Self {
        let mut payload = [0u8; CSR_MESSAGE_PAYLOAD_CAP];
        payload[0] = request_opcode as u8;
        payload[1] = (request_opcode >> 8) as u8;
        payload[2] = (request_opcode >> 16) as u8;
        payload[3] = (request_opcode >> 24) as u8;
        Self {
            opcode: CSR_ACK_OK,
            payload_len: 4,
            payload,
        }
    }

    #[must_use]
    pub const fn ack_error(request_opcode: u32) -> Self {
        let mut payload = [0u8; CSR_MESSAGE_PAYLOAD_CAP];
        payload[0] = request_opcode as u8;
        payload[1] = (request_opcode >> 8) as u8;
        payload[2] = (request_opcode >> 16) as u8;
        payload[3] = (request_opcode >> 24) as u8;
        Self {
            opcode: CSR_ACK_ERROR,
            payload_len: 4,
            payload,
        }
    }

    pub fn with_payload(op: u32, data: &[u8]) -> Result<Self, ()> {
        if data.len() > CSR_MESSAGE_PAYLOAD_CAP {
            return Err(());
        }
        let mut payload = [0u8; CSR_MESSAGE_PAYLOAD_CAP];
        payload[..data.len()].copy_from_slice(data);
        Ok(Self {
            opcode: op,
            payload_len: data.len() as u32,
            payload,
        })
    }
}

#[repr(C)]
pub struct CsrConnectMsg {
    pub opcode: u32,
    pub client_pid: u64,
}

impl CsrConnectMsg {
    #[must_use]
    pub const fn new(client_pid: u64) -> Self {
        Self {
            opcode: CSR_CONNECT,
            client_pid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_len() {
        let e = CsrMessageEnvelope::with_payload(CSR_CREATE_DESKTOP, b"Z").unwrap();
        assert_eq!(e.payload_len, 1);
        assert_eq!(e.payload[0], b'Z');
    }
}
