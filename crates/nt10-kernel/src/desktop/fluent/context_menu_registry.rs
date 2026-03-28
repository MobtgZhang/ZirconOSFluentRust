//! Extensible context menu command ids — migrates hard-coded desktop rows to registered handlers.

use crate::ke::spinlock::SpinLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextMenuCommand(pub u32);

pub mod built_in {
    use super::ContextMenuCommand;
    pub const ROW_VIEW: ContextMenuCommand = ContextMenuCommand(1);
    pub const ROW_SORT: ContextMenuCommand = ContextMenuCommand(2);
    pub const ROW_REFRESH: ContextMenuCommand = ContextMenuCommand(3);
}

const MAX_EXT: usize = 8;

static EXT_IDS: SpinLock<[Option<u32>; MAX_EXT]> = SpinLock::new([None; MAX_EXT]);

/// Register an extension command id into the next free slot (Shell extension path).
pub fn register_extension_command(id: u32) -> Result<usize, ()> {
    let mut g = EXT_IDS.lock();
    for (i, s) in g.iter_mut().enumerate() {
        if s.is_none() {
            *s = Some(id);
            return Ok(i);
        }
    }
    Err(())
}

#[must_use]
pub fn extension_command_at(slot: usize) -> Option<u32> {
    EXT_IDS.lock().get(slot).copied().flatten()
}
