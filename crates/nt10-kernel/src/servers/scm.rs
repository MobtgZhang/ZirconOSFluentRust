//! Service Control Manager — `CreateService` / `StartService` state skeleton (no `services.exe` yet).
//!
//! Real SCM would load the service image, push a `SERVICE_RUNNING` state machine, and talk ALPC to
//! the SVC host. This module records named service slots for bring-up tracing only.

use crate::ke::spinlock::SpinLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    StartPending,
    Running,
    StopPending,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceRecord {
    pub name_hash: u32,
    pub state: ServiceState,
}

const MAX_SERVICES: usize = 8;

struct ScmTable {
    entries: [Option<ServiceRecord>; MAX_SERVICES],
}

impl ScmTable {
    const fn new() -> Self {
        Self {
            entries: [None; MAX_SERVICES],
        }
    }

    fn find_slot(&self, name_hash: u32) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .find_map(|(i, e)| e.filter(|r| r.name_hash == name_hash).map(|_| i))
    }

    fn alloc_empty(&mut self) -> Option<usize> {
        self.entries.iter().position(|e| e.is_none())
    }
}

static SCM: SpinLock<ScmTable> = SpinLock::new(ScmTable::new());

/// `CreateService`-shaped registration (hash stands in for UTF-16 name).
pub fn create_service_bringup(name_hash: u32) -> Result<(), ()> {
    let mut g = SCM.lock();
    if g.find_slot(name_hash).is_some() {
        return Err(());
    }
    let i = g.alloc_empty().ok_or(())?;
    g.entries[i] = Some(ServiceRecord {
        name_hash,
        state: ServiceState::Stopped,
    });
    Ok(())
}

/// `StartService` — moves `Stopped` → `Running` through `StartPending` in one step for bring-up.
pub fn start_service_bringup(name_hash: u32) -> Result<(), ()> {
    let mut g = SCM.lock();
    let i = g.find_slot(name_hash).ok_or(())?;
    let e = g.entries[i].as_mut().ok_or(())?;
    if e.state != ServiceState::Stopped {
        return Err(());
    }
    e.state = ServiceState::Running;
    Ok(())
}

#[must_use]
pub fn query_service_state(name_hash: u32) -> Option<ServiceState> {
    let g = SCM.lock();
    let i = g.find_slot(name_hash)?;
    g.entries[i].map(|e| e.state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_start_query() {
        assert!(create_service_bringup(0x42).is_ok());
        assert_eq!(query_service_state(0x42), Some(ServiceState::Stopped));
        assert!(start_service_bringup(0x42).is_ok());
        assert_eq!(query_service_state(0x42), Some(ServiceState::Running));
    }
}
