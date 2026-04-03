//! I/O APIC (82093AA-style) — RTE programming stub for virtio IRQ routing (public Intel I/O APIC docs).
//!
//! Bring-up: ACPI/MADT parsing to locate I/O APIC MMIO and GSIs is not wired; call sites should log and
//! return `Err` until [`crate::hal::x86_64::acpi`] exposes a table consumer.

/// Program redirection table entry `gsi` to deliver `vector` on the BSP (edge-triggered, fixed).
///
/// # Safety
/// `ioapic_mmio_phys` must be identity-mapped; `gsi` must be within that controller's pin count.
pub unsafe fn ioapic_set_irq_vector_stub(
    ioapic_mmio_phys: u64,
    gsi: u32,
    vector: u8,
) -> Result<(), ()> {
    let _ = (ioapic_mmio_phys, gsi, vector);
    Err(())
}
