//! ConHost — ConPTY-style master/slave ring buffers (no escape sequences).
//! Future: pair with kernel `\\Device\\NamedPipe`-style objects for session-isolated TTY I/O.
//! TSF / IME integration would sit above this UTF-8 pipe (`extensions/phase-05-input-stack.md`).

/// Fixed-size byte ring for host ↔ client copy (bring-up).
#[derive(Debug)]
pub struct ConPtyRingBuffer<const N: usize> {
    data: [u8; N],
    head: usize,
    len: usize,
}

impl<const N: usize> ConPtyRingBuffer<N> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            data: [0; N],
            head: 0,
            len: 0,
        }
    }

    /// Push as many bytes as fit; returns count written.
    pub fn write(&mut self, src: &[u8]) -> usize {
        let mut n = 0usize;
        for &b in src {
            if self.len >= N {
                break;
            }
            let idx = (self.head + self.len) % N;
            self.data[idx] = b;
            self.len += 1;
            n += 1;
        }
        n
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        let b = self.data[self.head % N];
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(b)
    }

    #[must_use]
    pub fn available(&self) -> usize {
        self.len
    }
}

pub type ConPtyBringupRing = ConPtyRingBuffer<256>;

/// Master/slave pair (names follow common PTY terminology; both live in kernel bring-up).
#[derive(Debug)]
pub struct ConPtyPipeStub {
    pub master_to_slave: ConPtyBringupRing,
    pub slave_to_master: ConPtyBringupRing,
}

impl ConPtyPipeStub {
    #[must_use]
    pub fn new() -> Self {
        Self {
            master_to_slave: ConPtyBringupRing::new(),
            slave_to_master: ConPtyBringupRing::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_round_trip() {
        let mut r = ConPtyRingBuffer::<8>::new();
        assert_eq!(r.write(b"abc"), 3);
        assert_eq!(r.read_byte(), Some(b'a'));
        assert_eq!(r.read_byte(), Some(b'b'));
    }
}
