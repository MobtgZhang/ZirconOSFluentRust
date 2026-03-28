//! ALPC port — connection server endpoint with a fixed-depth message queue (bring-up).

use super::message::{AlpcInlineMessage, ALPC_INLINE_BYTES};
use core::sync::atomic::{AtomicU64, Ordering};

/// Server-side port identifier (opaque).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AlpcPortId(pub u64);

static NEXT_PORT: AtomicU64 = AtomicU64::new(1);

const QUEUE_DEPTH: usize = 4;

/// Minimal port + bounded queue (single address space bring-up).
#[derive(Debug)]
pub struct AlpcPort {
    pub id: AlpcPortId,
    queue: [AlpcInlineMessage; QUEUE_DEPTH],
    head: usize,
    tail: usize,
    len: usize,
}

impl AlpcPort {
    #[must_use]
    pub fn new() -> Self {
        let id = AlpcPortId(NEXT_PORT.fetch_add(1, Ordering::Relaxed));
        Self {
            id,
            queue: [AlpcInlineMessage::empty(); QUEUE_DEPTH],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub fn try_send(&mut self, payload: &[u8]) -> Result<(), ()> {
        if self.len >= QUEUE_DEPTH || payload.len() > ALPC_INLINE_BYTES {
            return Err(());
        }
        let mut msg = AlpcInlineMessage::empty();
        msg.len = payload.len() as u32;
        msg.data[..payload.len()].copy_from_slice(payload);
        self.queue[self.tail] = msg;
        self.tail = (self.tail + 1) % QUEUE_DEPTH;
        self.len += 1;
        Ok(())
    }

    pub fn try_recv(&mut self) -> Result<AlpcInlineMessage, ()> {
        if self.len == 0 {
            return Err(());
        }
        let msg = self.queue[self.head];
        self.head = (self.head + 1) % QUEUE_DEPTH;
        self.len -= 1;
        Ok(msg)
    }
}
