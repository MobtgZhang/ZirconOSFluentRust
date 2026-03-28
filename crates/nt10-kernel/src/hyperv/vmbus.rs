//! VMBus — disconnected device placeholder for future IRP wiring.

/// Unconnected channel node (no guest ring buffer mapped).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmbusStubDevice {
    pub instance_id: u32,
    pub offer_ring_size: usize,
}

impl VmbusStubDevice {
    #[must_use]
    pub const fn unconnected() -> Self {
        Self {
            instance_id: 0,
            offer_ring_size: 0,
        }
    }
}
