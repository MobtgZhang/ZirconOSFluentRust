//! Phase 6 bring-up: Zircon-owned ALPC port hints and spawn stubs (no Windows protocol cloning).

use crate::ps::process::ProcessId;

/// UTF-8 port path for a future csrss API object (documented name only; not wired to a live port yet).
pub const ZR_ALPC_CSRSS_API_PORT_UTF8: &[u8] = b"\\RPC Control\\ZrNt10CsrssApi";

/// Placeholder: smss would post here after [`crate::alpc::cross_proc::post_cross_address_space`] binds user buffers.
pub fn try_alpc_handoff_csrss_spawn_stub(_from_smss: ProcessId, _image_hint: u32) -> Result<(), ()> {
    Err(())
}
