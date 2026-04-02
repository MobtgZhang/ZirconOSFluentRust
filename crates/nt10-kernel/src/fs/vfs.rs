//! Virtual file system mount table (stub).

use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::fs::fat32::{self, Fat32Bpb};
use crate::io::device::BlockVolumeBringup;
use crate::io::iomgr::io_complete_request;
use crate::io::irp::Irp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MountId(pub u32);

#[derive(Clone, Copy)]
pub struct VfsMountPoint {
    pub id: MountId,
    pub volume_label: [u8; 16],
    /// Block device backing this mount (optional until driver attaches storage).
    pub block_volume: Option<BlockVolumeBringup>,
}

impl VfsMountPoint {
    pub const fn new(id: u32) -> Self {
        Self {
            id: MountId(id),
            volume_label: [0; 16],
            block_volume: None,
        }
    }

    #[must_use]
    pub fn with_ramdisk(id: u32, data: &'static [u8]) -> Self {
        Self {
            id: MountId(id),
            volume_label: [0; 16],
            block_volume: Some(BlockVolumeBringup::from_static_slice(data)),
        }
    }

    /// VirtIO-blk (or any linear disk image) staged in RAM — same as [`Self::with_ramdisk`], path name for VFS/PE bring-up.
    #[must_use]
    pub fn with_virtio_blk_linear_image(id: u32, whole_disk_image: &'static [u8]) -> Self {
        Self::with_ramdisk(id, whole_disk_image)
    }
}

pub struct VfsTable {
    pub mounts: [Option<VfsMountPoint>; 8],
}

impl VfsTable {
    pub const fn new() -> Self {
        const NONE: Option<VfsMountPoint> = None;
        Self { mounts: [NONE; 8] }
    }

    pub fn mount(&mut self, slot: usize, point: VfsMountPoint) -> Result<(), ()> {
        if slot >= self.mounts.len() {
            return Err(());
        }
        if self.mounts[slot].is_some() {
            return Err(());
        }
        self.mounts[slot] = Some(point);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, slot: usize) -> Option<&VfsMountPoint> {
        self.mounts.get(slot)?.as_ref()
    }
}

/// Open file cursor for a mounted volume with [`VfsMountPoint::block_volume`] set.
pub struct OpenFileHandle {
    pub mount_slot: usize,
    pub position: u64,
}

/// Optional global VFS for UEFI Fluent **Files** window (`vfs_register_bringup`).
/// Future: session-local handle from SMSS / IO manager instead of static pointer.
static BRINGUP_VFS: AtomicPtr<VfsTable> = AtomicPtr::new(core::ptr::null_mut());

/// Register `vfs` for [`fill_desktop_file_list`]; call once from kernel init if a table exists.
///
/// # Safety
/// `ptr` must point to a `VfsTable` that outlives the desktop session (e.g. `static mut` or leaked).
pub unsafe fn vfs_register_bringup(ptr: NonNull<VfsTable>) {
    BRINGUP_VFS.store(ptr.as_ptr(), Ordering::Release);
}

#[must_use]
pub fn vfs_bringup_ptr() -> Option<NonNull<VfsTable>> {
    let p = BRINGUP_VFS.load(Ordering::Acquire);
    if p.is_null() {
        None
    } else {
        Some(unsafe { NonNull::new_unchecked(p) })
    }
}

/// One-line summary for mount `slot`: `VolN: label` or `(empty)`.
pub fn format_mount_slot_line(slot: usize, mp: Option<&VfsMountPoint>, out: &mut [u8; 80]) -> usize {
    let mut w = 0usize;
    let push = |out: &mut [u8; 80], w: &mut usize, b: u8| {
        if *w < out.len() {
            out[*w] = b;
            *w += 1;
        }
    };
    for &b in b"Vol" {
        push(out, &mut w, b);
    }
    let slot_u = slot as u32;
    if slot_u < 10 {
        push(out, &mut w, b'0' + slot_u as u8);
    } else {
        push(out, &mut w, b'0' + (slot_u / 10) as u8);
        push(out, &mut w, b'0' + (slot_u % 10) as u8);
    }
    match mp {
        Some(m) => {
            push(out, &mut w, b':');
            push(out, &mut w, b' ');
            for i in 0..m.volume_label.len() {
                let b = m.volume_label[i];
                if b == 0 {
                    break;
                }
                if (32..127).contains(&b) {
                    push(out, &mut w, b);
                }
            }
            if m.block_volume.is_some() {
                for &b in b" [mounted]" {
                    push(out, &mut w, b);
                }
            }
        }
        None => {
            for &b in b" (empty)" {
                push(out, &mut w, b);
            }
        }
    }
    w
}

fn format_fat83_name(raw: &[u8; 11], line: &mut [u8; 80]) -> usize {
    let mut w = 0usize;
    let mut end = 8usize;
    while end > 0 && raw[end - 1] == b' ' {
        end -= 1;
    }
    for b in raw.iter().take(end) {
        if w < line.len() {
            line[w] = *b;
            w += 1;
        }
    }
    let mut ext_end = 11usize;
    while ext_end > 8 && raw[ext_end - 1] == b' ' {
        ext_end -= 1;
    }
    if ext_end > 8 && w + 1 + (ext_end - 8) <= line.len() {
        line[w] = b'.';
        w += 1;
        for b in raw.iter().take(ext_end).skip(8) {
            line[w] = *b;
            w += 1;
        }
    }
    w
}

/// If `mp` backs a plausible FAT32 image, append root short-file names to the desktop list.
fn append_fat32_root_files(mp: &VfsMountPoint, rows: &mut [[u8; 80]; 32], lens: &mut [usize; 32], count: &mut usize) {
    let Some(bv) = mp.block_volume.as_ref() else {
        return;
    };
    let vol = unsafe { bv.disk.as_slice() };
    if vol.len() < core::mem::size_of::<Fat32Bpb>() {
        return;
    }
    let bpb: Fat32Bpb = unsafe { vol.as_ptr().cast::<Fat32Bpb>().read_unaligned() };
    if !bpb.looks_plausible() {
        return;
    }
    let mut raw = [[0u8; 11]; 16];
    let Ok(n) = fat32::fat32_list_root_short_names_chained(&bpb, vol, 16, &mut raw) else {
        return;
    };
    for i in 0..n {
        if *count >= rows.len() {
            break;
        }
        let nw = format_fat83_name(&raw[i], &mut rows[*count]);
        if nw == 0 {
            continue;
        }
        lens[*count] = nw;
        *count += 1;
    }
}

/// Fill `rows` / `lens` / `count` for the Files list: **This PC** plus mount slots (or a stub line).
pub fn fill_desktop_file_list(rows: &mut [[u8; 80]; 32], lens: &mut [usize; 32], count: &mut usize) {
    *count = 0;
    lens.fill(0);
    const T: &[u8] = b"This PC";
    rows[0][..T.len()].copy_from_slice(T);
    lens[0] = T.len();
    *count = 1;
    let vfs_ptr = vfs_bringup_ptr();
    if let Some(p) = vfs_ptr {
        let vfs = unsafe { p.as_ref() };
        for slot in 0..8 {
            if *count >= 32 {
                break;
            }
            let mp = vfs.get(slot);
            let n = format_mount_slot_line(slot, mp, &mut rows[*count]);
            if n > 0 || mp.is_some() {
                lens[*count] = n.max(1);
                *count += 1;
            }
            if let Some(m) = mp {
                append_fat32_root_files(m, rows, lens, count);
            }
        }
    } else if *count < 32 {
        const M: &[u8] = b"No VFS registered (see vfs_register_bringup)";
        let n = M.len().min(80);
        rows[*count][..n].copy_from_slice(&M[..n]);
        lens[*count] = n;
        *count += 1;
    }
}

/// Read a root-directory file by exact FAT 8.3 name (11 bytes, e.g. `b"HELLO   TXT"`).
#[must_use]
pub fn vfs_read_fat32_root_file_short(
    vfs: &VfsTable,
    slot: usize,
    name11: &[u8; 11],
    dest: &mut [u8],
) -> Result<usize, ()> {
    let mp = vfs.get(slot).ok_or(())?;
    let bv = mp.block_volume.as_ref().ok_or(())?;
    let vol = unsafe { bv.disk.as_slice() };
    if vol.len() < core::mem::size_of::<Fat32Bpb>() {
        return Err(());
    }
    let bpb: Fat32Bpb = unsafe { vol.as_ptr().cast::<Fat32Bpb>().read_unaligned() };
    if !bpb.looks_plausible() {
        return Err(());
    }
    let (fc, sz) = fat32::fat32_find_root_file_short_name(&bpb, vol, name11, 64).ok_or(())?;
    fat32::fat32_read_file_chain(&bpb, vol, fc, sz, dest)
}

/// Read a slice from a root short-name file starting at `start` byte offset.
#[must_use]
pub fn vfs_read_fat32_root_file_partial(
    vfs: &VfsTable,
    slot: usize,
    name11: &[u8; 11],
    start: u64,
    dest: &mut [u8],
) -> Result<usize, ()> {
    let mp = vfs.get(slot).ok_or(())?;
    let bv = mp.block_volume.as_ref().ok_or(())?;
    let vol = unsafe { bv.disk.as_slice() };
    if vol.len() < core::mem::size_of::<Fat32Bpb>() {
        return Err(());
    }
    let bpb: Fat32Bpb = unsafe { vol.as_ptr().cast::<Fat32Bpb>().read_unaligned() };
    if !bpb.looks_plausible() {
        return Err(());
    }
    let (fc, sz) = fat32::fat32_find_root_file_short_name(&bpb, vol, name11, 64).ok_or(())?;
    fat32::fat32_read_file_chain_partial(&bpb, vol, fc, sz, start, dest)
}

impl OpenFileHandle {
    #[must_use]
    pub fn open_mount(vfs: &VfsTable, slot: usize) -> Result<Self, ()> {
        let m = vfs.get(slot).ok_or(())?;
        if m.block_volume.is_none() {
            return Err(());
        }
        Ok(Self {
            mount_slot: slot,
            position: 0,
        })
    }
}

/// Performs a read from the mount's block volume, advances the file position, and completes the IRP.
pub fn vfs_read_via_irp(
    vfs: &VfsTable,
    h: &mut OpenFileHandle,
    buf: &mut [u8],
    irp: &mut Irp,
) -> i32 {
    let mp = match vfs.get(h.mount_slot) {
        Some(m) => m,
        None => {
            io_complete_request(irp, -1, 0);
            return -1;
        }
    };
    let vol = match mp.block_volume.as_ref() {
        Some(v) => v,
        None => {
            io_complete_request(irp, -1, 0);
            return -1;
        }
    };
    let n = vol.disk.read_at(h.position, buf);
    h.position += n as u64;
    io_complete_request(irp, 0, n);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::irp::Irp;
    use core::ptr::NonNull;

    extern crate alloc;

    #[test]
    fn vfs_fat32_read_root_short_file() {
        let mut vol = [0u8; 8192];
        let mut bpb: fat32::Fat32Bpb = unsafe { core::mem::zeroed() };
        bpb.bytes_per_sector = 512;
        bpb.sectors_per_cluster = 1;
        bpb.reserved_sector_count = 1;
        bpb.num_fats = 1;
        bpb.fat_size_32 = 1;
        bpb.root_cluster = 2;
        let data_sec = fat32::fat32_first_data_sector(&bpb);
        unsafe {
            (vol.as_mut_ptr() as *mut fat32::Fat32Bpb).write(bpb);
        }
        let fat_off = 512usize;
        vol[fat_off + 2 * 4..fat_off + 2 * 4 + 4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        vol[fat_off + 3 * 4..fat_off + 3 * 4 + 4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        let r2 = (data_sec as usize) * 512;
        let mut ent = [0u8; 32];
        ent[0..11].copy_from_slice(b"HELLO   TXT");
        ent[11] = 0x20;
        ent[26..28].copy_from_slice(&3u16.to_le_bytes());
        ent[28..32].copy_from_slice(&5u32.to_le_bytes());
        vol[r2..r2 + 32].copy_from_slice(&ent);
        let r3 = (data_sec as usize + 1) * 512;
        vol[r3..r3 + 5].copy_from_slice(b"HELLO");
        let leaked: &'static [u8] = alloc::boxed::Box::leak(alloc::boxed::Box::new(vol)).as_slice();
        let mut vfs = VfsTable::new();
        vfs.mount(0, VfsMountPoint::with_ramdisk(0, leaked))
            .unwrap();
        let mut buf = [0u8; 8];
        let hello = *b"HELLO   TXT";
        let n = vfs_read_fat32_root_file_short(&vfs, 0, &hello, &mut buf).expect("read");
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], b"HELLO");
    }

    #[test]
    fn vfs_ramdisk_read_irp() {
        static VOL: &[u8] = b"Zr";
        let mut vfs = VfsTable::new();
        vfs.mount(0, VfsMountPoint::with_ramdisk(0, VOL)).unwrap();
        let mut h = OpenFileHandle::open_mount(&vfs, 0).unwrap();
        let mut buf = [0u8; 4];
        let mut irp = Irp::new_read(Some(NonNull::dangling()));
        let st = vfs_read_via_irp(&vfs, &mut h, &mut buf, &mut irp);
        assert_eq!(st, 0);
        assert_eq!(irp.information, 2);
        assert_eq!(&buf[..2], b"Zr");
        assert_eq!(h.position, 2);
    }
}
