//! xHCI (USB3) MMIO bring-up: reset, command/event rings, root-port reset, slot/EP0, HID interrupt IN (poll).
//!
//! **Assumptions**: BSP only; UEFI loads the kernel at low identity (`0x8000000` VA == PA) so `static` DMA
//! addresses are valid physical addresses. Interrupts are not used — event ring is polled.

#![allow(dead_code)]

use core::ptr::{addr_of_mut, read_volatile, write_volatile};

use super::pci::{self, PciMmioBar, PciXhciLocation};
use super::usb::{
    UsbDeviceDescriptor, UsbInterfaceSummary, DESC_CONFIGURATION, DESC_DEVICE, REQ_GET_DESCRIPTOR,
    REQ_SET_CONFIGURATION,
};

// --- TRB ------------------------------------------------------------------

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Trb {
    param: u64,
    status: u32,
    ctrl: u32,
}

impl Trb {
    const fn zero() -> Self {
        Self {
            param: 0,
            status: 0,
            ctrl: 0,
        }
    }
}

const TRB_TYPE_NOOP: u32 = 23 << 10;
const TRB_TYPE_ENABLE_SLOT: u32 = 9 << 10;
const TRB_TYPE_ADDRESS_DEVICE: u32 = 11 << 10;
const TRB_TYPE_CONFIGURE_EP: u32 = 12 << 10;
const TRB_TYPE_EVAL_CONTEXT: u32 = 13 << 10;
const TRB_TYPE_SETUP: u32 = 2 << 10;
const TRB_TYPE_DATA: u32 = 3 << 10;
const TRB_TYPE_STATUS: u32 = 4 << 10;

const TRB_CYCLE: u32 = 1;
const TRB_IOC: u32 = 1 << 5;
const TRB_CHAIN: u32 = 1 << 4;
const TRB_DIR_IN: u32 = 1 << 16;

/// Transfer type for SETUP stage: IN data from device.
const TRB_TRT_IN: u32 = 3 << 16;

const CC_SUCCESS: u8 = 1;

const USB_REQ_TYPE_DEVICE_IN: u8 = 0x80;
const USB_REQ_TYPE_DEVICE_OUT: u8 = 0x00;

// --- Static DMA pool -------------------------------------------------------

const CMD_N: usize = 128;
const EVT_N: usize = 128;
const EP0_N: usize = 64;
const INTR_N: usize = 64;

#[repr(C, align(4096))]
struct XhciPool {
    dcbaa: [u64; 256],
    scratch_ptrs: [u64; 32],
    scratch_pages: [[u8; 4096]; 8],
    /// Output device contexts (pool is 4 KiB aligned; offset kept 64-byte aligned).
    dev_ctx: [u8; 2048],
    cmd_ring: [Trb; CMD_N],
    evt_ring: [Trb; EVT_N],
    /// Event Ring Segment Table (one segment): two u32s base lo/hi + size + rsvd.
    erst: [u8; 32],
    input_ctx: [u8; 4096],
    ep0_ring: [Trb; EP0_N],
    intr_ring: [Trb; INTR_N],
    /// Control / interrupt payload buffers.
    io: [u8; 2048],
}

static mut POOL: XhciPool = XhciPool {
    dcbaa: [0; 256],
    scratch_ptrs: [0; 32],
    scratch_pages: [[0; 4096]; 8],
    dev_ctx: [0; 2048],
    cmd_ring: [Trb::zero(); CMD_N],
    evt_ring: [Trb::zero(); EVT_N],
    erst: [0; 32],
    input_ctx: [0; 4096],
    ep0_ring: [Trb::zero(); EP0_N],
    intr_ring: [Trb::zero(); INTR_N],
    io: [0; 2048],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XhciError {
    Timeout,
    NotHalted,
    BadVersion,
    BadCompletion(u8),
    NoPort,
    TransferShort,
    Parse,
    NoHid,
    TooManyScratchBuffers,
}

#[derive(Clone, Copy, Debug)]
pub struct XhciHidState {
    pub pci: PciXhciLocation,
    pub mmio_base: u64,
    pub port_index: u8,
    pub slot_id: u8,
    pub intr_dci: u8,
    pub max_packet_intr: u16,
    pub keyboard_iface: Option<u8>,
    pub mouse_iface: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
struct RingPersist {
    mmio: u64,
    cmd_enq: usize,
    cmd_cycle: u32,
    evt_deq: usize,
    evt_cycle: u32,
    ep0_enq: usize,
    ep0_cycle: u32,
    intr_enq: usize,
    intr_cycle: u32,
}

static mut XHCI_RING_PERSIST: Option<RingPersist> = None;
static mut XHCI_SCRATCH_ACTIVE: bool = false;

#[inline]
fn dcbaa_slot_index(slot_id: u8) -> usize {
    unsafe {
        if XHCI_SCRATCH_ACTIVE {
            slot_id as usize
        } else {
            (slot_id as usize).saturating_sub(1)
        }
    }
}

unsafe fn ring_persist_save(h: &Host) {
    XHCI_RING_PERSIST = Some(RingPersist {
        mmio: h.cap as u64,
        cmd_enq: h.cmd_enq,
        cmd_cycle: h.cmd_cycle,
        evt_deq: h.evt_deq,
        evt_cycle: h.evt_cycle,
        ep0_enq: h.ep0_enq,
        ep0_cycle: h.ep0_cycle,
        intr_enq: h.intr_enq,
        intr_cycle: h.intr_cycle,
    });
}

unsafe fn host_with_persist(mmio: u64) -> Result<Host, XhciError> {
    let mut h = Host::from_mmio(mmio)?;
    if let Some(p) = XHCI_RING_PERSIST {
        if p.mmio == h.cap as u64 {
            h.cmd_enq = p.cmd_enq;
            h.cmd_cycle = p.cmd_cycle;
            h.evt_deq = p.evt_deq;
            h.evt_cycle = p.evt_cycle;
            h.ep0_enq = p.ep0_enq;
            h.ep0_cycle = p.ep0_cycle;
            h.intr_enq = p.intr_enq;
            h.intr_cycle = p.intr_cycle;
        }
    }
    Ok(h)
}

// --- MMIO -------------------------------------------------------------------

#[inline]
unsafe fn r32(base: usize, off: usize) -> u32 {
    unsafe { read_volatile((base + off) as *const u32) }
}

#[inline]
unsafe fn w32(base: usize, off: usize, v: u32) {
    unsafe { write_volatile((base + off) as *mut u32, v) }
}

#[inline]
unsafe fn r64(base: usize, off: usize) -> u64 {
    let lo = r32(base, off) as u64;
    let hi = r32(base, off + 4) as u64;
    lo | (hi << 32)
}

#[inline]
unsafe fn w64(base: usize, off: usize, v: u64) {
    w32(base, off, v as u32);
    w32(base, off + 4, (v >> 32) as u32);
}

#[inline]
fn phys<T>(p: *const T) -> u64 {
    p as u64
}

unsafe fn wait_clear(base: usize, off: usize, mask: u32, spins: usize) -> Result<(), XhciError> {
    for _ in 0..spins {
        if r32(base, off) & mask == 0 {
            return Ok(());
        }
        spin_wait();
    }
    Err(XhciError::Timeout)
}

#[inline]
fn spin_wait() {
    for _ in 0..256 {
        core::hint::spin_loop();
    }
}

// --- Host -------------------------------------------------------------------

struct Host {
    cap: usize,
    op: usize,
    rt: usize,
    db: usize,
    n_ports: u8,
    max_slots: u8,
    ctx_bytes: usize,
    cmd_enq: usize,
    cmd_cycle: u32,
    evt_deq: usize,
    evt_cycle: u32,
    ep0_enq: usize,
    ep0_cycle: u32,
    intr_enq: usize,
    intr_cycle: u32,
}

impl Host {
    unsafe fn from_mmio(mmio: u64) -> Result<Self, XhciError> {
        let cap = mmio as usize;
        let cap0 = r32(cap, 0);
        let cap_len = (cap0 & 0xFF) as usize;
        let hci_ver = (cap0 >> 16) & 0xFFFF;
        if hci_ver < 0x0100 {
            return Err(XhciError::BadVersion);
        }
        let hcs1 = r32(cap, 4);
        let max_slots = (hcs1 & 0xFF) as u8;
        let n_ports = ((hcs1 >> 24) & 0xFF) as u8;
        let hcc = r32(cap, 0x10);
        let ctx_bytes = if (hcc & (1 << 2)) != 0 { 64 } else { 32 };
        let db_off = r32(cap, 0x14);
        let rt_off = r32(cap, 0x18);
        let db = cap + ((db_off & 0xFFFF_FFE0) as usize);
        let rt = cap + ((rt_off & 0xFFFF_FFE0) as usize);
        let op = cap + cap_len;
        Ok(Self {
            cap,
            op,
            rt,
            db,
            n_ports,
            max_slots,
            ctx_bytes,
            cmd_enq: 0,
            cmd_cycle: 1,
            evt_deq: 0,
            evt_cycle: 1,
            ep0_enq: 0,
            ep0_cycle: 1,
            intr_enq: 0,
            intr_cycle: 1,
        })
    }

    unsafe fn reset(&mut self) -> Result<(), XhciError> {
        // HCRST
        let usbcmd = r32(self.op, 0);
        w32(self.op, 0, usbcmd | (1 << 1));
        wait_clear(self.op, 4, 1 << 11, 1_000_000)?; // USBSTS.CNR
        w32(self.op, 0, 0);
        wait_clear(self.op, 4, 1 << 11, 1_000_000)?;
        Ok(())
    }

    unsafe fn init_rings(&mut self) -> Result<(), XhciError> {
        let hcs2 = r32(self.cap, 8);
        let n_scratch = ((hcs2 >> 21) & 0x1F) as usize;
        let pool = addr_of_mut!(POOL);
        for e in (*pool).dcbaa.iter_mut() {
            *e = 0;
        }
        XHCI_SCRATCH_ACTIVE = n_scratch > 0;
        if n_scratch > 0 && n_scratch <= (*pool).scratch_pages.len() {
            for i in 0..n_scratch {
                (*pool).scratch_ptrs[i] = phys(addr_of_mut!((*pool).scratch_pages[i]));
            }
            (*pool).dcbaa[0] = phys(addr_of_mut!((*pool).scratch_ptrs));
        } else if n_scratch > (*pool).scratch_pages.len() {
            return Err(XhciError::TooManyScratchBuffers);
        }

        let slots = self.max_slots.min(32).max(1);
        w32(self.op, 0x38, slots as u32);

        let cmd_p = phys(addr_of_mut!((*pool).cmd_ring));
        let evt_p = phys(addr_of_mut!((*pool).evt_ring));
        // CRCR: command ring pointer, RCS in bit 0
        w64(self.op, 0x18, cmd_p | 1);

        // Interrupter 0: ERST size, ERSTBA, ERDP
        let iman_off = self.rt + 0x20;
        let erst = phys(addr_of_mut!((*pool).erst));
        // One segment: base 64-bit + size (TRB count)
        let erst_ent = addr_of_mut!((*pool).erst) as *mut u32;
        write_volatile(erst_ent, evt_p as u32);
        write_volatile(erst_ent.add(1), (evt_p >> 32) as u32);
        write_volatile(erst_ent.add(2), EVT_N as u32);
        write_volatile(erst_ent.add(3), 0);

        w32(iman_off, 8, EVT_N as u32);
        w64(iman_off, 0x10, erst);
        w64(iman_off, 0x18, evt_p | (1 << 3)); // EHB clear

        let dcbaa_p = phys(addr_of_mut!((*pool).dcbaa));
        w64(self.op, 0x30, dcbaa_p);

        // Run
        w32(self.op, 0, 1);
        wait_clear(self.op, 4, 1 << 11, 1_000_000)?;
        Ok(())
    }

    unsafe fn push_cmd(&mut self, param: u64, status: u32, ctrl: u32) {
        let pool = addr_of_mut!(POOL);
        let i = self.cmd_enq;
        let c = TRB_CYCLE & self.cmd_cycle;
        (*pool).cmd_ring[i] = Trb {
            param,
            status,
            ctrl: ctrl | c,
        };
        self.cmd_enq += 1;
        if self.cmd_enq >= CMD_N {
            self.cmd_enq = 0;
            self.cmd_cycle ^= TRB_CYCLE;
        }
    }

    unsafe fn ring_cmd_doorbell(&self) {
        let pool = addr_of_mut!(POOL);
        let addr = phys(addr_of_mut!((*pool).cmd_ring)) + (self.cmd_enq as u64) * 16;
        w64(self.op, 0x18, addr | (self.cmd_cycle & 1) as u64);
    }

    unsafe fn wait_cmd_completion(&mut self, spins: usize) -> Result<(u8, u8), XhciError> {
        let pool = addr_of_mut!(POOL);
        for _ in 0..spins {
            let trb = &(*pool).evt_ring[self.evt_deq];
            let c = trb.ctrl & 1;
            if c != self.evt_cycle & 1 {
                spin_wait();
                continue;
            }
            let ty = (trb.ctrl >> 10) & 0x3F;
            if ty == 33 {
                // Command completion
                let cc = ((trb.status >> 24) & 0xFF) as u8;
                let slot = ((trb.ctrl >> 24) & 0xFF) as u8;
                self.evt_deq += 1;
                if self.evt_deq >= EVT_N {
                    self.evt_deq = 0;
                    self.evt_cycle ^= TRB_CYCLE;
                }
                // ACK EHB on ERDP
                let iman_off = self.rt + 0x20;
                let erdp = phys(addr_of_mut!((*pool).evt_ring)) + (self.evt_deq as u64) * 16;
                w64(iman_off, 0x18, erdp | (1 << 3));
                return Ok((cc, slot));
            }
            if ty == 34 {
                // Port status — consume
                self.evt_deq += 1;
                if self.evt_deq >= EVT_N {
                    self.evt_deq = 0;
                    self.evt_cycle ^= TRB_CYCLE;
                }
                let iman_off = self.rt + 0x20;
                let erdp = phys(addr_of_mut!((*pool).evt_ring)) + (self.evt_deq as u64) * 16;
                w64(iman_off, 0x18, erdp | (1 << 3));
                continue;
            }
            spin_wait();
        }
        Err(XhciError::Timeout)
    }

    unsafe fn portsc_base(&self) -> usize {
        self.op + 0x400
    }

    unsafe fn wait_transfer_event(&mut self, spins: usize) -> Result<(u8, u32), XhciError> {
        let pool = addr_of_mut!(POOL);
        for _ in 0..spins {
            let trb = &(*pool).evt_ring[self.evt_deq];
            let c = trb.ctrl & 1;
            if c != self.evt_cycle & 1 {
                spin_wait();
                continue;
            }
            let ty = (trb.ctrl >> 10) & 0x3F;
            if ty == 32 {
                let cc = ((trb.status >> 24) & 0xFF) as u8;
                let resid = (trb.status & 0xFFFFFF) as u32;
                self.evt_deq += 1;
                if self.evt_deq >= EVT_N {
                    self.evt_deq = 0;
                    self.evt_cycle ^= TRB_CYCLE;
                }
                let iman_off = self.rt + 0x20;
                let erdp = phys(addr_of_mut!((*pool).evt_ring)) + (self.evt_deq as u64) * 16;
                w64(iman_off, 0x18, erdp | (1 << 3));
                return Ok((cc, resid));
            }
            if ty == 33 || ty == 34 {
                let _ = self.wait_cmd_completion(1);
                continue;
            }
            spin_wait();
        }
        Err(XhciError::Timeout)
    }

    unsafe fn doorbell(&self, slot: u8, target: u8) {
        let off = (slot as usize) * 4;
        w32(self.db, off, target as u32);
    }

    unsafe fn push_ep0(&mut self, param: u64, status: u32, ctrl: u32) {
        let pool = addr_of_mut!(POOL);
        let i = self.ep0_enq;
        let c = TRB_CYCLE & self.ep0_cycle;
        (*pool).ep0_ring[i] = Trb {
            param,
            status,
            ctrl: ctrl | c,
        };
        self.ep0_enq += 1;
        if self.ep0_enq >= EP0_N {
            self.ep0_enq = 0;
            self.ep0_cycle ^= TRB_CYCLE;
        }
    }

    unsafe fn push_intr(&mut self, param: u64, status: u32, ctrl: u32) {
        let pool = addr_of_mut!(POOL);
        let i = self.intr_enq;
        let c = TRB_CYCLE & self.intr_cycle;
        (*pool).intr_ring[i] = Trb {
            param,
            status,
            ctrl: ctrl | c,
        };
        self.intr_enq += 1;
        if self.intr_enq >= INTR_N {
            self.intr_enq = 0;
            self.intr_cycle ^= TRB_CYCLE;
        }
    }
}

// --- Context helpers --------------------------------------------------------

fn dci_for_ep(addr: u8) -> u8 {
    let num = addr & 0x0F;
    let inn = (addr & 0x80) != 0;
    2 * num + if inn { 1 } else { 0 }
}

unsafe fn input_ctx_clear(ic: *mut u8, bytes: usize) {
    core::ptr::write_bytes(ic, 0, bytes);
}

unsafe fn input_set_add_flag(ic: *mut u8, idx: u8) {
    let p = ic.add(4) as *mut u32;
    let v = read_volatile(p);
    write_volatile(p, v | (1u32 << (idx as u32)));
}

unsafe fn write_u32_le(p: *mut u8, off: usize, v: u32) {
    let q = p.add(off) as *mut u32;
    write_volatile(q, v.to_le());
}

/// Input Context for **Address Device**: ICC (32 B) + Slot + EP0 (DCI 1).
unsafe fn build_slot_ep0_input(
    ic: *mut u8,
    ctx: usize,
    speed: u8,
    root_port: u8,
    ep0_deq: u64,
    ep0_cycle: u32,
    max_packet0: u16,
) {
    let clear_bytes = 0x20 + 2 * ctx;
    input_ctx_clear(ic, clear_bytes + 64);
    input_set_add_flag(ic, 0);
    input_set_add_flag(ic, 1);

    let slot = ic.add(0x20);
    let dw0 = (1u32 << 27) | ((speed as u32) << 20);
    write_u32_le(slot, 0, dw0);
    write_u32_le(slot, 4, 0);
    write_u32_le(slot, 8, root_port as u32);

    let ep = ic.add(0x20 + ctx);
    let typ_ctrl = 4u32 << 3;
    let cerr = 3u32 << 1;
    write_u32_le(ep, 0, cerr | typ_ctrl);
    write_u32_le(ep, 4, max_packet0 as u32);
    let deq = (ep0_deq & !0xFu64) | (ep0_cycle & 1) as u64;
    write_volatile(ep.add(8) as *mut u64, deq.to_le());
    write_u32_le(ep, 16, 8);
}

/// Input Context for **Configure Endpoint**: only the interrupt EP (DCI) is added.
unsafe fn build_configure_intr_input(
    ic: *mut u8,
    ctx: usize,
    dci: u8,
    deq: u64,
    cycle: u32,
    mps: u16,
    iv: u8,
) {
    let clear_bytes = 0x20 + (dci as usize + 1) * ctx;
    input_ctx_clear(ic, clear_bytes + 64);
    input_set_add_flag(ic, dci);

    let ep = ic.add(0x20 + (dci as usize) * ctx);
    let typ_int = 7u32 << 3;
    let cerr = 3u32 << 1;
    write_u32_le(ep, 0, cerr | typ_int | ((iv as u32) << 16));
    write_u32_le(ep, 4, mps as u32);
    let deqp = (deq & !0xFu64) | (cycle & 1) as u64;
    write_volatile(ep.add(8) as *mut u64, deqp.to_le());
    write_u32_le(ep, 16, 8);
}

// --- Public API -------------------------------------------------------------

/// Locate PCI xHCI and return MMIO base (physical == virtual for flat kernel).
pub fn find_xhci() -> Option<(PciXhciLocation, PciMmioBar)> {
    pci::find_first_xhci_mmio_bar(0)
}

/// Reset controller, rings, NO-OP smoke, first connected root port, HID boot keyboard or mouse.
///
/// # Safety
/// BSP only; no concurrent access to [`POOL`].
pub unsafe fn xhci_init_hid() -> Result<XhciHidState, XhciError> {
    let (loc, bar) = find_xhci().ok_or(XhciError::NoPort)?;
    let mmio = bar.phys_base;
    let mut h = unsafe { Host::from_mmio(mmio)? };
    unsafe { h.reset()? };
    unsafe { h.init_rings()? };

    // NOOP
    unsafe {
        h.push_cmd(0, 0, TRB_TYPE_NOOP);
        h.ring_cmd_doorbell();
    }
    let (cc, _) = unsafe { h.wait_cmd_completion(500_000)? };
    if cc != CC_SUCCESS {
        return Err(XhciError::BadCompletion(cc));
    }

    // Find first port with CCS
    let mut port_idx: u8 = 0;
    let mut psc;
    for p in 1..=h.n_ports {
        let off = (p as usize - 1) * 0x10;
        psc = r32(h.portsc_base(), off);
        if (psc & 1) != 0 {
            port_idx = p;
            break;
        }
    }
    if port_idx == 0 {
        return Err(XhciError::NoPort);
    }
    let poff = (port_idx as usize - 1) * 0x10;
    // Port reset
    psc = r32(h.portsc_base(), poff);
    w32(h.portsc_base(), poff, psc | (1 << 4));
    for _ in 0..500_000 {
        psc = r32(h.portsc_base(), poff);
        if (psc & (1 << 4)) == 0 {
            break;
        }
        spin_wait();
    }
    let speed = ((psc >> 10) & 0xF) as u8;
    let max0 = match speed {
        2 => 8u16,
        1 | 3 => 64,
        4 | 5 => 512,
        _ => 64,
    };

    // Enable slot
    unsafe {
        h.push_cmd(0, 0, TRB_TYPE_ENABLE_SLOT);
        h.ring_cmd_doorbell();
    }
    let (cc, slot_id) = unsafe { h.wait_cmd_completion(500_000)? };
    if cc != CC_SUCCESS {
        return Err(XhciError::BadCompletion(cc));
    }
    if slot_id == 0 {
        return Err(XhciError::BadCompletion(0));
    }

    let pool = addr_of_mut!(POOL);
    let ep0_deq = phys(addr_of_mut!((*pool).ep0_ring));
    let ic = addr_of_mut!((*pool).input_ctx) as *mut u8;
    let ctx = h.ctx_bytes;
    unsafe {
        build_slot_ep0_input(ic, ctx, speed, port_idx, ep0_deq, h.ep0_cycle, max0);
        let icp = phys(ic);
        let di = dcbaa_slot_index(slot_id);
        (*pool).dcbaa[di] = phys(addr_of_mut!((*pool).dev_ctx));
        h.push_cmd(icp, 0, TRB_TYPE_ADDRESS_DEVICE | ((slot_id as u32) << 24));
        h.ring_cmd_doorbell();
    }
    let (cc, _) = unsafe { h.wait_cmd_completion(500_000)? };
    if cc != CC_SUCCESS {
        return Err(XhciError::BadCompletion(cc));
    }

    // Device descriptor
    let mut dev = UsbDeviceDescriptor::default();
    unsafe {
        xhci_control_get_descriptor(
            &mut h,
            slot_id,
            USB_REQ_TYPE_DEVICE_IN,
            REQ_GET_DESCRIPTOR,
            (DESC_DEVICE as u16) << 8,
            0,
            18,
            (*pool).io.as_mut_ptr(),
        )?;
        let p = &(*pool).io;
        dev.b_max_packet0 = p[7];
        dev.id_vendor = u16::from_le_bytes([p[8], p[9]]);
        dev.id_product = u16::from_le_bytes([p[10], p[11]]);
        dev.b_num_configurations = p[17];
    }

    if dev.b_max_packet0 as u16 != max0 {
        // Evaluate context for actual MPS0
        unsafe {
            build_slot_ep0_input(
                ic,
                ctx,
                speed,
                port_idx,
                ep0_deq,
                h.ep0_cycle,
                dev.b_max_packet0 as u16,
            );
            let icp = phys(ic);
            h.push_cmd(icp, 0, TRB_TYPE_EVAL_CONTEXT | ((slot_id as u32) << 24));
            h.ring_cmd_doorbell();
        }
        let (cc, _) = unsafe { h.wait_cmd_completion(500_000)? };
        if cc != CC_SUCCESS {
            return Err(XhciError::BadCompletion(cc));
        }
    }

    // Configuration descriptor (first 9 bytes for total length)
    let mut cfg_total;
    unsafe {
        xhci_control_get_descriptor(
            &mut h,
            slot_id,
            USB_REQ_TYPE_DEVICE_IN,
            REQ_GET_DESCRIPTOR,
            (DESC_CONFIGURATION as u16) << 8,
            0,
            9,
            (*pool).io.as_mut_ptr(),
        )?;
        cfg_total = u16::from_le_bytes([(*pool).io[2], (*pool).io[3]]) as usize;
        if cfg_total > (*pool).io.len() {
            cfg_total = (*pool).io.len();
        }
        xhci_control_get_descriptor(
            &mut h,
            slot_id,
            USB_REQ_TYPE_DEVICE_IN,
            REQ_GET_DESCRIPTOR,
            (DESC_CONFIGURATION as u16) << 8,
            0,
            cfg_total as u16,
            (*pool).io.as_mut_ptr(),
        )?;
    }

    let iface = parse_config_for_hid(unsafe {
        core::slice::from_raw_parts((*pool).io.as_ptr(), cfg_total.min((*pool).io.len()))
    })
    .ok_or(XhciError::NoHid)?;
    let dci = dci_for_ep(iface.endpoint_in);

    // SET_CONFIGURATION
    unsafe {
        xhci_control_out_zero(
            &mut h,
            slot_id,
            USB_REQ_TYPE_DEVICE_OUT,
            REQ_SET_CONFIGURATION,
            1,
            0,
        )?;
    }

    // Configure interrupt endpoint
    let intr_deq = phys(addr_of_mut!((*pool).intr_ring));
    unsafe {
        build_configure_intr_input(
            ic,
            ctx,
            dci,
            intr_deq,
            h.intr_cycle,
            iface.max_packet_size,
            iface.interval,
        );
        let icp = phys(ic);
        h.push_cmd(
            icp,
            0,
            TRB_TYPE_CONFIGURE_EP | ((slot_id as u32) << 24),
        );
        h.ring_cmd_doorbell();
    }
    let (cc, _) = unsafe { h.wait_cmd_completion(500_000)? };
    if cc != CC_SUCCESS {
        return Err(XhciError::BadCompletion(cc));
    }

    // Prime interrupt IN
    let buf = addr_of_mut!((*pool).io[1024]) as *mut u8;
    unsafe {
        h.push_intr(
            phys(buf),
            iface.max_packet_size as u32,
            TRB_TYPE_DATA | TRB_IOC | TRB_DIR_IN,
        );
        h.doorbell(slot_id, dci);
    }

    unsafe {
        ring_persist_save(&h);
    }

    Ok(XhciHidState {
        pci: loc,
        mmio_base: mmio,
        port_index: port_idx,
        slot_id,
        intr_dci: dci,
        max_packet_intr: iface.max_packet_size,
        keyboard_iface: if iface.is_hid_boot_keyboard() {
            Some(iface.interface_number)
        } else {
            None
        },
        mouse_iface: if iface.is_hid_pointer_interface() {
            Some(iface.interface_number)
        } else {
            None
        },
    })
}

/// Poll interrupt endpoint; refills transfer ring. Copies report into `out` (up to `out.len()`).
///
/// # Safety
/// [`xhci_init_hid`] must have succeeded; BSP only.
pub unsafe fn xhci_poll_hid(state: &XhciHidState, out: &mut [u8]) -> Result<usize, XhciError> {
    let mut h = unsafe { host_with_persist(state.mmio_base)? };
    let pool = addr_of_mut!(POOL);
    let buf = addr_of_mut!((*pool).io[1024]) as *mut u8;
    let (cc, resid) = match h.wait_transfer_event(4096) {
        Ok(v) => v,
        Err(XhciError::Timeout) => {
            unsafe {
                ring_persist_save(&h);
            }
            return Ok(0);
        }
        Err(e) => return Err(e),
    };
    if cc != CC_SUCCESS && cc != 13 {
        return Err(XhciError::BadCompletion(cc));
    }
    let xfer_len = (state.max_packet_intr as u32).saturating_sub(resid);
    let n = (xfer_len as usize).min(out.len()).min(64);
    unsafe {
        core::ptr::copy_nonoverlapping(buf, out.as_mut_ptr(), n);
        h.push_intr(
            phys(buf),
            state.max_packet_intr as u32,
            TRB_TYPE_DATA | TRB_IOC | TRB_DIR_IN,
        );
        h.doorbell(state.slot_id, state.intr_dci);
        ring_persist_save(&h);
    }
    Ok(n)
}

// --- Control transfers ------------------------------------------------------

unsafe fn xhci_control_get_descriptor(
    h: &mut Host,
    slot: u8,
    rt: u8,
    req: u8,
    value: u16,
    index: u16,
    len: u16,
    data: *mut u8,
) -> Result<(), XhciError> {
    let pool = addr_of_mut!(POOL);
    let setup = addr_of_mut!((*pool).io[256]) as *mut u8;
    write_volatile(setup, rt);
    write_volatile(setup.add(1), req);
    write_volatile(setup.add(2), value as u8);
    write_volatile(setup.add(3), (value >> 8) as u8);
    write_volatile(setup.add(4), index as u8);
    write_volatile(setup.add(5), (index >> 8) as u8);
    write_volatile(setup.add(6), len as u8);
    write_volatile(setup.add(7), (len >> 8) as u8);

    h.ep0_enq = 0;
    h.ep0_cycle = 1;
    h.push_ep0(
        phys(setup),
        TRB_TRT_IN,
        TRB_TYPE_SETUP | TRB_CHAIN,
    );
    if len > 0 {
        h.push_ep0(
            phys(data),
            len as u32,
            TRB_TYPE_DATA | TRB_DIR_IN | TRB_CHAIN,
        );
    }
    h.push_ep0(0, 0, TRB_TYPE_STATUS | TRB_IOC);
    h.doorbell(slot, 1);
    let (cc, _) = h.wait_transfer_event(2_000_000)?;
    if cc != CC_SUCCESS && cc != 13 {
        return Err(XhciError::BadCompletion(cc));
    }
    Ok(())
}

unsafe fn xhci_control_out_zero(
    h: &mut Host,
    slot: u8,
    rt: u8,
    req: u8,
    value: u16,
    index: u16,
) -> Result<(), XhciError> {
    let pool = addr_of_mut!(POOL);
    let setup = addr_of_mut!((*pool).io[256]) as *mut u8;
    write_volatile(setup, rt);
    write_volatile(setup.add(1), req);
    write_volatile(setup.add(2), value as u8);
    write_volatile(setup.add(3), (value >> 8) as u8);
    write_volatile(setup.add(4), index as u8);
    write_volatile(setup.add(5), (index >> 8) as u8);
    write_volatile(setup.add(6), 0);
    write_volatile(setup.add(7), 0);

    h.ep0_enq = 0;
    h.ep0_cycle = 1;
    h.push_ep0(phys(setup), 0, TRB_TYPE_SETUP | TRB_CHAIN);
    h.push_ep0(0, 0, TRB_TYPE_STATUS | TRB_IOC | TRB_DIR_IN);
    h.doorbell(slot, 1);
    let (cc, _) = h.wait_transfer_event(2_000_000)?;
    if cc != CC_SUCCESS {
        return Err(XhciError::BadCompletion(cc));
    }
    Ok(())
}

fn parse_config_for_hid(cfg: &[u8]) -> Option<UsbInterfaceSummary> {
    if cfg.len() < 9 {
        return None;
    }
    let total = u16::from_le_bytes([cfg[2], cfg[3]]) as usize;
    let lim = total.min(cfg.len());
    let mut i = 0usize;
    let mut if_num = 0u8;
    let mut if_class = 0u8;
    let mut if_sub = 0u8;
    let mut if_proto = 0u8;
    let mut first_hid_in: Option<UsbInterfaceSummary> = None;
    let mut pointer_hid_in: Option<UsbInterfaceSummary> = None;
    while i + 2 < lim {
        let blen = cfg[i] as usize;
        if blen < 2 || i + blen > lim {
            break;
        }
        let typ = cfg[i + 1];
        match typ {
            4 if blen >= 9 => {
                if_num = cfg[i + 2];
                if_class = cfg[i + 5];
                if_sub = cfg[i + 6];
                if_proto = cfg[i + 7];
            }
            5 if blen >= 6 => {
                let addr = cfg[i + 2];
                let attr = cfg[i + 3];
                let mps = u16::from_le_bytes([cfg[i + 4], cfg[i + 5]]);
                let iv = if blen >= 7 { cfg[i + 6] } else { 0 };
                if if_class == 0x03 && (attr & 3) == 3 && (addr & 0x80) != 0 {
                    let sum = UsbInterfaceSummary {
                        interface_number: if_num,
                        alternate_setting: 0,
                        class: if_class,
                        subclass: if_sub,
                        protocol: if_proto,
                        endpoint_in: addr,
                        max_packet_size: mps & 0x7FF,
                        interval: iv,
                    };
                    if first_hid_in.is_none() {
                        first_hid_in = Some(sum);
                    }
                    if sum.is_hid_pointer_interface() {
                        pointer_hid_in = Some(sum);
                    }
                }
            }
            _ => {}
        }
        i += blen;
    }
    pointer_hid_in.or(first_hid_in)
}
