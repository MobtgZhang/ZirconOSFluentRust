//! Phase 6 bring-up: Zircon-owned ALPC port hints and spawn stubs (no Windows protocol cloning).
//!
//! Fixed [`ZrCsrssAlpcEnvelope`] is the intended first wire format for [`crate::servers::smss::try_smss_alpc_start_csrss_stub`]
//! once [`crate::alpc::cross_proc::post_cross_address_space`] / `post_cross_address_space_into_remote_va` carry user payloads.

use crate::ps::process::ProcessId;

/// UTF-8 port path for a future csrss API object (documented name only; not wired to a live port yet).
pub const ZR_ALPC_CSRSS_API_PORT_UTF8: &[u8] = b"\\RPC Control\\ZrNt10CsrssApi";

/// Magic: `ZrCR` little-endian — distinguishes ZirconOS csrss ALPC envelopes from random bytes.
pub const ZR_CSRSS_ALPC_ENVELOPE_MAGIC: u32 = 0x5243725A;

pub const ZR_CSRSS_ALPC_ENVELOPE_VERSION: u16 = 1;

pub mod csrss_alpc_opcodes {
    pub const HELLO: u16 = 1;
    pub const SMSS_HANDOFF: u16 = 2;
    pub const CSRSS_READY: u16 = 3;
}

/// Fixed-size header; `payload_len` bytes follow in the same ALPC message buffer (bring-up contract).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ZrCsrssAlpcEnvelope {
    pub magic: u32,
    pub version: u16,
    pub opcode: u16,
    pub payload_len: u32,
}

impl ZrCsrssAlpcEnvelope {
    #[must_use]
    pub const fn new(opcode: u16, payload_len: u32) -> Self {
        Self {
            magic: ZR_CSRSS_ALPC_ENVELOPE_MAGIC,
            version: ZR_CSRSS_ALPC_ENVELOPE_VERSION,
            opcode,
            payload_len,
        }
    }
}

/// Placeholder: smss would post here after [`crate::alpc::cross_proc::post_cross_address_space`] binds user buffers.
pub fn try_alpc_handoff_csrss_spawn_stub(_from_smss: ProcessId, _image_hint: u32) -> Result<(), ()> {
    Err(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip_size() {
        let e = ZrCsrssAlpcEnvelope::new(csrss_alpc_opcodes::HELLO, 0);
        assert_eq!(e.magic, ZR_CSRSS_ALPC_ENVELOPE_MAGIC);
        assert_eq!(core::mem::size_of::<ZrCsrssAlpcEnvelope>(), 12);
    }
}
