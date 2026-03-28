//! Trap frame handling.
//!
//! Exception gates will share bookkeeping with [`crate::ke::irql`]; timer IRQs use
//! [`crate::arch::x86_64::isr`] until per-vector Rust dispatch lands.
