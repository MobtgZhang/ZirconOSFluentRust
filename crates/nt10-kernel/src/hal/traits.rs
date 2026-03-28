//! HAL interface — execution body uses this instead of calling UART/MSR sites directly.

/// Platform operations visible to KE/MM bring-up.
pub trait Hal {
    fn debug_write(&self, data: &[u8]);
    /// Full TLB flush (e.g. reload `CR3`); may be expensive.
    fn flush_tlb_all(&self);
}

/// x86_64 BSP implementation (COM1 + `CR3` reload).
#[derive(Clone, Copy, Debug, Default)]
pub struct X86Hal64;

impl Hal for X86Hal64 {
    fn debug_write(&self, data: &[u8]) {
        crate::hal::x86_64::serial::write_bytes(data);
    }

    fn flush_tlb_all(&self) {
        crate::arch::x86_64::paging::flush_tlb_all();
    }
}
