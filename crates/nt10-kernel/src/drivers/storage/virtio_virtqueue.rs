//! Virtqueue layout (VirtIO 1.x) — descriptor / avail / used rings in a contiguous slab.

/// Descriptor flags.
pub const VRING_DESC_F_NEXT: u16 = 1;
pub const VRING_DESC_F_WRITE: u16 = 2;

#[repr(C, packed)]
pub struct VringDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[repr(C, packed)]
pub struct VirtioBlkReqLe {
    pub type_le: u32,
    pub reserved_le: u32,
    pub sector_le: u64,
}

pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_S_OK: u8 = 0;

/// Returns `(desc_off, avail_off, used_off, data_off, total_used)` for `queue_size` power of 2 <= 256.
#[must_use]
pub fn ring_layout(queue_size: usize) -> Option<(usize, usize, usize, usize, usize)> {
    if queue_size == 0 || queue_size > 256 || !queue_size.is_power_of_two() {
        return None;
    }
    let desc_bytes = 16usize.checked_mul(queue_size)?;
    let desc_off = 0usize;
    let avail_off = align_up(desc_off + desc_bytes, 2)?;
    // avail: flags 2 + idx 2 + ring[qs]*2 + used_event 2 (EVENT_IDX; reserve space)
    let avail_bytes = 4usize.checked_add(2usize.checked_mul(queue_size)?)?.checked_add(2)?;
    let used_off = align_up(avail_off + avail_bytes, 4)?;
    // used: flags 2 + idx 2 + elems*8 + avail_event 2
    let used_bytes = 4usize.checked_add(8usize.checked_mul(queue_size)?)?.checked_add(2)?;
    let data_off = align_up(used_off + used_bytes, 8)?;
    // one sector buffer + status + req struct
    let total = data_off
        .checked_add(core::mem::size_of::<VirtioBlkReqLe>())?
        .checked_add(512)?
        .checked_add(8)?;
    Some((desc_off, avail_off, used_off, data_off, total))
}

#[inline]
fn align_up(v: usize, a: usize) -> Option<usize> {
    if a == 0 {
        return None;
    }
    let m = v.checked_add(a)?.checked_sub(1)?;
    Some(m & !(a - 1))
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn q8_layout_monotonic() {
        let (d, a, u, data, tot) = ring_layout(8).expect("layout");
        assert!(d < a);
        assert!(a < u);
        assert!(u < data);
        assert!(tot < 4096);
    }
}
