//! FAT32 — boot sector layout (public BPB field meanings).

/// BIOS Parameter Block for FAT32 (first sector fields used by bring-up tools).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Fat32Bpb {
    pub jmp_boot: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    pub fat_size_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info: u16,
    pub bk_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_number: u8,
    pub reserved1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fil_sys_type: [u8; 8],
}

impl Fat32Bpb {
    #[must_use]
    pub fn sector_size(&self) -> u32 {
        u32::from(self.bytes_per_sector)
    }

    /// Returns `true` if fields are in plausible ranges (not a full FS verify).
    #[must_use]
    pub fn looks_plausible(&self) -> bool {
        let bps = self.sector_size();
        (512..=4096).contains(&bps) && self.num_fats >= 1 && self.root_cluster >= 2
    }
}

/// First sector index of the data region (after reserved + FATs).
#[must_use]
pub fn fat32_first_data_sector(bpb: &Fat32Bpb) -> u32 {
    let reserved = bpb.reserved_sector_count as u32;
    let fats = bpb.num_fats as u32;
    reserved + fats * bpb.fat_size_32
}

#[must_use]
pub fn fat32_sector_slice<'a>(bpb: &Fat32Bpb, vol: &'a [u8], sector: u32) -> Option<&'a [u8]> {
    let bps = bpb.sector_size() as usize;
    let base = (sector as usize).checked_mul(bps)?;
    vol.get(base..base + bps)
}

/// First sector of cluster `clust` (FAT32; valid clusters start at 2).
#[must_use]
pub fn fat32_cluster_first_sector(bpb: &Fat32Bpb, clust: u32) -> Option<u32> {
    if clust < 2 {
        return None;
    }
    let data = fat32_first_data_sector(bpb);
    let spc = bpb.sectors_per_cluster as u32;
    Some(data + (clust - 2) * spc)
}

const ATTR_LONG_NAME: u8 = 0x0F;
const ATTR_VOLUME_ID: u8 = 0x08;

const FAT32_EOC_MIN: u32 = 0x0FFF_FFF8;

/// Reads the FAT32 entry (lower 28 bits) for `cluster` (cluster index >= 2 typical).
#[must_use]
pub fn fat32_fat_entry(bpb: &Fat32Bpb, vol: &[u8], cluster: u32) -> Option<u32> {
    let fat_start_sec = u32::from(bpb.reserved_sector_count);
    let bps = bpb.sector_size() as usize;
    let idx = (cluster as usize).checked_mul(4)?;
    let fat_byte = fat_start_sec.checked_mul(bps as u32)? as usize;
    let base = fat_byte.checked_add(idx)?;
    let le = vol.get(base..base + 4)?;
    let raw = u32::from_le_bytes([le[0], le[1], le[2], le[3]]);
    Some(raw & 0x0FFF_FFFF)
}

/// Next cluster in a chain, or `None` if end-of-chain / bad FAT.
#[must_use]
pub fn fat32_next_cluster(bpb: &Fat32Bpb, vol: &[u8], cluster: u32) -> Option<u32> {
    let e = fat32_fat_entry(bpb, vol, cluster)?;
    if e >= FAT32_EOC_MIN {
        return None;
    }
    Some(e)
}

/// Counts short names in one directory cluster (stops at first end-of-directory slot in any sector).
#[must_use]
pub fn fat32_count_short_names_in_cluster(bpb: &Fat32Bpb, vol: &[u8], clust: u32) -> Result<usize, ()> {
    if !bpb.looks_plausible() {
        return Err(());
    }
    let s0 = fat32_cluster_first_sector(bpb, clust).ok_or(())?;
    let bps = bpb.sector_size() as usize;
    let spc = bpb.sectors_per_cluster as usize;
    let mut count = 0usize;
    let mut ended = false;
    for sec_off in 0..spc {
        let sec = s0 + sec_off as u32;
        let sect = fat32_sector_slice(bpb, vol, sec).ok_or(())?;
        for i in 0..(bps / 32) {
            let entry = &sect[i * 32..i * 32 + 32];
            let first = entry[0];
            if first == 0 {
                ended = true;
                break;
            }
            if first == 0xe5 {
                continue;
            }
            let attr = entry[11];
            if attr == ATTR_LONG_NAME || (attr & ATTR_VOLUME_ID) != 0 {
                continue;
            }
            count += 1;
        }
        if ended {
            break;
        }
    }
    Ok(count)
}

/// Root directory: first cluster only (legacy helper).
#[must_use]
pub fn fat32_count_root_short_names_first_cluster(bpb: &Fat32Bpb, vol: &[u8]) -> Result<usize, ()> {
    fat32_count_short_names_in_cluster(bpb, vol, bpb.root_cluster)
}

/// Root directory following the FAT cluster chain (bounded steps).
#[must_use]
pub fn fat32_count_root_short_names_chained(
    bpb: &Fat32Bpb,
    vol: &[u8],
    max_clusters: u32,
) -> Result<usize, ()> {
    if !bpb.looks_plausible() {
        return Err(());
    }
    let mut total = 0usize;
    let mut clust = bpb.root_cluster;
    for _ in 0..max_clusters {
        total += fat32_count_short_names_in_cluster(bpb, vol, clust)?;
        match fat32_next_cluster(bpb, vol, clust) {
            Some(n) => clust = n,
            None => return Ok(total),
        }
    }
    Err(())
}

/// Split a path on `\\` (optional leading `\\`); yields one 8.3-style segment and the rest.
#[must_use]
pub fn next_path_component_backslash(path: &[u8]) -> Option<(&[u8], &[u8])> {
    let p = path.strip_prefix(b"\\").unwrap_or(path);
    if p.is_empty() {
        return None;
    }
    if let Some(i) = p.iter().position(|&c| c == b'\\') {
        let (a, r) = p.split_at(i);
        let tail = if r.len() > 1 { &r[1..] } else { &[] };
        Some((a, tail))
    } else {
        Some((p, &[]))
    }
}

/// Lists up to `out.len()` short (8.3) **file** names from the root directory cluster chain.
/// Each name is the raw 11 directory bytes (space-padded); caller may trim spaces.
#[must_use]
pub fn fat32_list_root_short_names_chained(
    bpb: &Fat32Bpb,
    vol: &[u8],
    max_clusters: u32,
    out: &mut [[u8; 11]],
) -> Result<usize, ()> {
    if !bpb.looks_plausible() {
        return Err(());
    }
    let mut written = 0usize;
    let mut clust = bpb.root_cluster;
    'outer: for _ in 0..max_clusters {
        if written >= out.len() {
            break;
        }
        let s0 = fat32_cluster_first_sector(bpb, clust).ok_or(())?;
        let bps = bpb.sector_size() as usize;
        let spc = bpb.sectors_per_cluster as usize;
        let mut ended = false;
        for sec_off in 0..spc {
            let sec = s0 + sec_off as u32;
            let sect = fat32_sector_slice(bpb, vol, sec).ok_or(())?;
            for i in 0..(bps / 32) {
                if written >= out.len() {
                    break 'outer;
                }
                let entry = &sect[i * 32..i * 32 + 32];
                let first = entry[0];
                if first == 0 {
                    ended = true;
                    break;
                }
                if first == 0xe5 {
                    continue;
                }
                let attr = entry[11];
                if attr == ATTR_LONG_NAME || (attr & ATTR_VOLUME_ID) != 0 {
                    continue;
                }
                if (attr & 0x10) != 0 {
                    // directory entry — skip for simple file listing
                    continue;
                }
                let mut name = [0u8; 11];
                name.copy_from_slice(&entry[0..11]);
                out[written] = name;
                written += 1;
            }
            if ended {
                break;
            }
        }
        match fat32_next_cluster(bpb, vol, clust) {
            Some(n) => clust = n,
            None => break,
        }
    }
    Ok(written)
}

/// Locate a root **file** (not subdirectory) with exact 8.3 name `name11` (11 bytes, space-padded).
#[must_use]
pub fn fat32_find_root_file_short_name(
    bpb: &Fat32Bpb,
    vol: &[u8],
    name11: &[u8; 11],
    max_clusters: u32,
) -> Option<(u32, u32)> {
    if !bpb.looks_plausible() {
        return None;
    }
    let mut clust = bpb.root_cluster;
    for _ in 0..max_clusters {
        let s0 = fat32_cluster_first_sector(bpb, clust)?;
        let bps = bpb.sector_size() as usize;
        let spc = bpb.sectors_per_cluster as usize;
        let mut ended = false;
        for sec_off in 0..spc {
            let sec = s0 + sec_off as u32;
            let sect = fat32_sector_slice(bpb, vol, sec)?;
            for i in 0..(bps / 32) {
                let entry = &sect[i * 32..i * 32 + 32];
                let first = entry[0];
                if first == 0 {
                    ended = true;
                    break;
                }
                if first == 0xe5 {
                    continue;
                }
                let attr = entry[11];
                if attr == ATTR_LONG_NAME || (attr & ATTR_VOLUME_ID) != 0 {
                    continue;
                }
                if (attr & 0x10) != 0 {
                    continue;
                }
                if entry[0..11] != *name11 {
                    continue;
                }
                let cl_hi = u16::from_le_bytes([entry[20], entry[21]]) as u32;
                let cl_lo = u16::from_le_bytes([entry[26], entry[27]]) as u32;
                let first_c = (cl_hi << 16) | cl_lo;
                let size = u32::from_le_bytes([entry[28], entry[29], entry[30], entry[31]]);
                return Some((first_c, size));
            }
            if ended {
                return None;
            }
        }
        clust = fat32_next_cluster(bpb, vol, clust)?;
    }
    None
}

/// Read up to `file_size` bytes from a FAT32 cluster chain into `out`; returns bytes copied.
#[must_use]
pub fn fat32_read_file_chain(
    bpb: &Fat32Bpb,
    vol: &[u8],
    first_clust: u32,
    file_size: u32,
    out: &mut [u8],
) -> Result<usize, ()> {
    if !bpb.looks_plausible() || first_clust < 2 {
        return Err(());
    }
    let bps = bpb.sector_size() as usize;
    let spc = bpb.sectors_per_cluster as usize;
    let cluster_bytes = bps
        .checked_mul(spc)
        .ok_or(())?;
    let want = (file_size as usize).min(out.len());
    let mut written = 0usize;
    let mut clust = first_clust;
    let max_clusters = want
        .saturating_div(cluster_bytes.max(1))
        .saturating_add(3);
    for _ in 0..max_clusters {
        if written >= want {
            break;
        }
        let s0 = fat32_cluster_first_sector(bpb, clust).ok_or(())?;
        for sec_off in 0..spc {
            if written >= want {
                break;
            }
            let sec = s0 + sec_off as u32;
            let sect = fat32_sector_slice(bpb, vol, sec).ok_or(())?;
            let take = (want - written).min(sect.len());
            out[written..written + take].copy_from_slice(&sect[..take]);
            written += take;
        }
        if written >= want {
            break;
        }
        match fat32_next_cluster(bpb, vol, clust) {
            Some(n) => clust = n,
            None => break,
        }
    }
    Ok(written)
}

/// Read from `start` byte offset in the file (for demand paging / partial views).
#[must_use]
pub fn fat32_read_file_chain_partial(
    bpb: &Fat32Bpb,
    vol: &[u8],
    first_clust: u32,
    file_size: u32,
    start: u64,
    out: &mut [u8],
) -> Result<usize, ()> {
    if !bpb.looks_plausible() || first_clust < 2 {
        return Err(());
    }
    if start >= file_size as u64 || out.is_empty() {
        return Ok(0);
    }
    let max_read = ((file_size as u64) - start) as usize;
    let want = out.len().min(max_read);
    let bps = bpb.sector_size() as usize;
    let spc = bpb.sectors_per_cluster as usize;
    let mut to_skip = start as usize;
    let mut written = 0usize;
    let mut clust = first_clust;
    let max_clusters = file_size
        .saturating_div((bps * spc).max(1) as u32)
        .saturating_add(4) as usize;
    'outer: for _ in 0..max_clusters {
        let s0 = fat32_cluster_first_sector(bpb, clust).ok_or(())?;
        for sec_off in 0..spc {
            let sec = s0 + sec_off as u32;
            let sect = fat32_sector_slice(bpb, vol, sec).ok_or(())?;
            if to_skip >= sect.len() {
                to_skip -= sect.len();
                continue;
            }
            let slice = &sect[to_skip..];
            to_skip = 0;
            let take = (want - written).min(slice.len());
            out[written..written + take].copy_from_slice(&slice[..take]);
            written += take;
            if written >= want {
                break 'outer;
            }
        }
        match fat32_next_cluster(bpb, vol, clust) {
            Some(n) => clust = n,
            None => break,
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_vol_with_one_root_entry() -> ([u8; 4096], Fat32Bpb) {
        let mut vol = [0u8; 4096];
        let mut bpb: Fat32Bpb = unsafe { core::mem::zeroed() };
        bpb.bytes_per_sector = 512;
        bpb.sectors_per_cluster = 1;
        bpb.reserved_sector_count = 1;
        bpb.num_fats = 2;
        bpb.fat_size_32 = 1;
        bpb.root_cluster = 2;
        let root_sec = fat32_cluster_first_sector(&bpb, 2).unwrap();
        assert_eq!(root_sec, fat32_first_data_sector(&bpb));
        let bpb_for_return = bpb;
        unsafe {
            (vol.as_mut_ptr() as *mut Fat32Bpb).write(bpb);
        }
        let off = root_sec as usize * 512;
        vol[off..off + 11].copy_from_slice(b"HELLO   TXT");
        vol[off + 11] = 0x20;
        (vol, bpb_for_return)
    }

    #[test]
    fn root_short_name_count_smoke() {
        let (vol, bpb) = tiny_vol_with_one_root_entry();
        let n = fat32_count_root_short_names_first_cluster(&bpb, &vol).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn read_root_short_file_by_cluster_chain() {
        let mut vol = [0u8; 8192];
        let mut bpb: Fat32Bpb = unsafe { core::mem::zeroed() };
        bpb.bytes_per_sector = 512;
        bpb.sectors_per_cluster = 1;
        bpb.reserved_sector_count = 1;
        bpb.num_fats = 1;
        bpb.fat_size_32 = 1;
        bpb.root_cluster = 2;
        let data_sec = fat32_first_data_sector(&bpb);
        unsafe {
            (vol.as_mut_ptr() as *mut Fat32Bpb).write(bpb);
        }
        let fat_off = 512usize;
        // cluster 2 = root dir EOC, cluster 3 = file data EOC
        vol[fat_off + 2 * 4..fat_off + 2 * 4 + 4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        vol[fat_off + 3 * 4..fat_off + 3 * 4 + 4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        let r2 = (data_sec as usize) * 512;
        let mut ent = [0u8; 32];
        ent[0..11].copy_from_slice(b"HELLO   TXT");
        ent[11] = 0x20;
        ent[26..28].copy_from_slice(&3u16.to_le_bytes());
        let payload = b"HELLO";
        ent[28..32].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        vol[r2..r2 + 32].copy_from_slice(&ent);
        let r3 = (data_sec as usize + 1) * 512;
        vol[r3..r3 + payload.len()].copy_from_slice(payload);
        let bpb_read: Fat32Bpb = unsafe { *(vol.as_ptr() as *const Fat32Bpb) };
        let mut buf = [0u8; 16];
        let hello_name = *b"HELLO   TXT";
        let (fc, sz) =
            fat32_find_root_file_short_name(&bpb_read, &vol, &hello_name, 8).expect("find");
        assert_eq!(fc, 3);
        assert_eq!(sz, 5);
        let n = fat32_read_file_chain(&bpb_read, &vol, fc, sz, &mut buf).expect("read");
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], b"HELLO");
    }

    #[test]
    fn fat_chain_two_root_clusters() {
        let mut vol = [0u8; 8192];
        let mut bpb: Fat32Bpb = unsafe { core::mem::zeroed() };
        bpb.bytes_per_sector = 512;
        bpb.sectors_per_cluster = 1;
        bpb.reserved_sector_count = 1;
        bpb.num_fats = 1;
        bpb.fat_size_32 = 1;
        bpb.root_cluster = 2;
        let data_sec = fat32_first_data_sector(&bpb);
        unsafe {
            (vol.as_mut_ptr() as *mut Fat32Bpb).write(bpb);
        }
        let fat_off = 512usize;
        // cluster 2 -> 3, cluster 3 -> EOC
        vol[fat_off + 2 * 4..fat_off + 2 * 4 + 4].copy_from_slice(&3u32.to_le_bytes());
        vol[fat_off + 3 * 4..fat_off + 3 * 4 + 4].copy_from_slice(&0x0FFF_FFFFu32.to_le_bytes());
        let r2 = (data_sec as usize) * 512;
        vol[r2..r2 + 11].copy_from_slice(b"A       TXT");
        vol[r2 + 11] = 0x20;
        let r3 = (data_sec as usize + 1) * 512;
        vol[r3..r3 + 11].copy_from_slice(b"B       TXT");
        vol[r3 + 11] = 0x20;
        let bpb_read: Fat32Bpb = unsafe { *(vol.as_ptr() as *const Fat32Bpb) };
        let n = fat32_count_root_short_names_chained(&bpb_read, &vol, 8).unwrap();
        assert_eq!(n, 2);
        let mut names = [[0u8; 11]; 4];
        let ln = fat32_list_root_short_names_chained(&bpb_read, &vol, 8, &mut names).unwrap();
        assert_eq!(ln, 2);
        assert_eq!(&names[0], b"A       TXT");
        assert_eq!(&names[1], b"B       TXT");
    }

    #[test]
    fn next_path_component_backslash_splits() {
        assert_eq!(
            next_path_component_backslash(br"DIR\FILE.TXT"),
            Some((&b"DIR"[..], &b"FILE.TXT"[..]))
        );
        assert_eq!(
            next_path_component_backslash(br"\SUB\NEXT"),
            Some((&b"SUB"[..], &b"NEXT"[..]))
        );
        assert_eq!(next_path_component_backslash(br"ONLY"), Some((&b"ONLY"[..], &[][..])));
        assert_eq!(next_path_component_backslash(br""), None);
    }
}
