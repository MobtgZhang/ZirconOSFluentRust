#![no_std]

//! ZirconOSFluent / NT10 kernel library (skeleton).
//! Layout: [ideas/ZirconOS_NT10_Architecture.md](../../../ideas/ZirconOS_NT10_Architecture.md) §4.

pub mod alpc;
pub mod arch;
pub mod desktop;
pub mod drivers;
pub mod fs;
pub mod hal;
pub mod handoff;
pub mod hyperv;
pub mod io;
pub mod ke;
pub mod kmain;
pub mod libs;
pub mod loader;
pub mod milestones;
pub mod mm;
pub mod ob;
pub mod ps;
pub mod rtl;
pub mod se;
pub mod servers;
pub mod subsystems;
pub mod sync;
pub mod vbs;
