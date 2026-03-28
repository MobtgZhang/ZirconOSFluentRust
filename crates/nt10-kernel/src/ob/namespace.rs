//! Logical namespace paths (`\Device\...`, `\Sessions\...`) — parser only.
//!
//! Full insertion lives in the object manager; this module classifies path prefixes for routing.

use super::directory::DirectoryObject;
use core::ptr::NonNull;

/// `\Sessions\` prefix for session-local namespace routing.
pub const SESSIONS_PREFIX: &[u8] = br"\Sessions\";

/// Single-session bring-up: default interactive window station under session 0.
pub const SESSION0_WINSTA0: &[u8] = br"\Sessions\0\WinSta0";
/// Default desktop on `WinSta0` (login / apps share this path in the simplified model).
pub const SESSION0_DESKTOP_DEFAULT: &[u8] = br"\Sessions\0\WinSta0\Default";

/// Supported session directory slots (`\Sessions\0\` … `\Sessions\7\`).
pub const MAX_SESSION_DIRS: usize = 8;

/// Returns the path after `\Sessions\` when present.
#[must_use]
pub fn strip_sessions_subpath(path: &[u8]) -> Option<&[u8]> {
    path.strip_prefix(SESSIONS_PREFIX)
}

/// Parses `N\rest` where `N` is ASCII `'0'`..=`'7'` and `rest` is the child name bytes.
#[must_use]
pub fn parse_session_id_and_name(rest: &[u8]) -> Option<(usize, &[u8])> {
    if rest.len() < 2 {
        return None;
    }
    let c = rest[0];
    if !(b'0'..=b'7').contains(&c) || rest[1] != b'\\' {
        return None;
    }
    let sid = (c - b'0') as usize;
    if sid >= MAX_SESSION_DIRS {
        return None;
    }
    Some((sid, &rest[2..]))
}

/// Per-session directory buckets (Session 0 at index 0, etc.).
pub struct NamespaceBuckets {
    pub by_session: [DirectoryObject; MAX_SESSION_DIRS],
}

impl NamespaceBuckets {
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_session: core::array::from_fn(|_| DirectoryObject::new()),
        }
    }

    /// Insert using `\Sessions\<0-7>\<name>` (name = final segment, no further backslashes in bring-up).
    pub fn insert_session_path(&mut self, path: &[u8], object: NonNull<()>) -> Result<(), ()> {
        let rest = strip_sessions_subpath(path).ok_or(())?;
        let (sid, name) = parse_session_id_and_name(rest).ok_or(())?;
        if name.is_empty() || name.contains(&b'\\') {
            return Err(());
        }
        self.by_session[sid].insert(name, object)
    }

    #[must_use]
    pub fn lookup_session_path(&self, path: &[u8]) -> Option<NonNull<()>> {
        let rest = strip_sessions_subpath(path)?;
        let (sid, name) = parse_session_id_and_name(rest)?;
        self.by_session[sid].lookup(name)
    }

    /// Insert under Session 0 using a `\Sessions\0\<name>` path.
    pub fn insert_session0_child(&mut self, path: &[u8], object: NonNull<()>) -> Result<(), ()> {
        self.insert_session_path(path, object)
    }

    #[must_use]
    pub fn lookup_session0_child(&self, path: &[u8]) -> Option<NonNull<()>> {
        self.lookup_session_path(path)
    }

    /// Insert under `\Sessions\<id>\<name>` with explicit session index 0..8.
    pub fn insert_session_child(
        &mut self,
        session_id: u8,
        name: &[u8],
        object: NonNull<()>,
    ) -> Result<(), ()> {
        let sid = session_id as usize;
        if sid >= MAX_SESSION_DIRS {
            return Err(());
        }
        self.by_session[sid].insert(name, object)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathRoot {
    DosDevices,
    Device,
    ObjectTypes,
    Sessions,
    Unknown,
}

#[must_use]
pub fn classify_path(prefix: &[u8]) -> PathRoot {
    if prefix.starts_with(br"\??\") || prefix.starts_with(br"\DosDevices\") {
        return PathRoot::DosDevices;
    }
    if prefix.starts_with(br"\Device\") {
        return PathRoot::Device;
    }
    if prefix.starts_with(br"\ObjectTypes\") {
        return PathRoot::ObjectTypes;
    }
    if prefix.starts_with(br"\Sessions\") {
        return PathRoot::Sessions;
    }
    PathRoot::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_session_paths() {
        let mut ns = NamespaceBuckets::new();
        let a = NonNull::new(2usize as *mut ()).unwrap();
        let b = NonNull::new(3usize as *mut ()).unwrap();
        assert!(ns
            .insert_session_path(br"\Sessions\1\Foo", a)
            .is_ok());
        assert!(ns
            .insert_session_path(br"\Sessions\0\Bar", b)
            .is_ok());
        assert_eq!(ns.lookup_session_path(br"\Sessions\1\Foo"), Some(a));
        assert_eq!(ns.lookup_session_path(br"\Sessions\0\Bar"), Some(b));
    }
}
