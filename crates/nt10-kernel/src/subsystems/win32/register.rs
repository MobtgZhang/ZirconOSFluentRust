//! Win32 subsystem registration state (kernel-side bring-up).

use core::sync::atomic::{AtomicU8, Ordering};

const DISCONNECTED: u8 = 0;
const CONNECTING: u8 = 1;
const READY: u8 = 2;

static WIN32_SUBSYS: AtomicU8 = AtomicU8::new(DISCONNECTED);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Win32SubsystemState {
    Disconnected,
    Connecting,
    Ready,
}

fn tag_to_state(t: u8) -> Win32SubsystemState {
    match t {
        CONNECTING => Win32SubsystemState::Connecting,
        READY => Win32SubsystemState::Ready,
        _ => Win32SubsystemState::Disconnected,
    }
}

#[must_use]
pub fn win32_subsystem_state() -> Win32SubsystemState {
    tag_to_state(WIN32_SUBSYS.load(Ordering::Relaxed))
}

pub fn win32_subsystem_begin_connect() {
    WIN32_SUBSYS.store(CONNECTING, Ordering::Release);
}

pub fn win32_subsystem_mark_ready() {
    WIN32_SUBSYS.store(READY, Ordering::Release);
}

pub fn win32_subsystem_disconnect() {
    WIN32_SUBSYS.store(DISCONNECTED, Ordering::Release);
}
