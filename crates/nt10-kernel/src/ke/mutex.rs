//! Kernel mutex (dispatcher object placeholder; uses spinlock until waitable state exists).

use crate::ke::spinlock::SpinLock;

pub struct Mutex<T> {
    inner: SpinLock<T>,
}

impl<T> Mutex<T> {
    pub const fn new(v: T) -> Self {
        Self {
            inner: SpinLock::new(v),
        }
    }

    pub fn lock(&self) -> crate::ke::spinlock::SpinLockGuard<'_, T> {
        self.inner.lock()
    }
}
