//! Logical namespace paths (`\Device\...`, `\Sessions\...`) — parser only.
//!
//! Full insertion lives in the object manager; this module classifies path prefixes for routing.

use super::directory::DirectoryObject;
use core::ptr::NonNull;

/// Max reparse steps for future symbolic-link resolution (no symlink objects yet).
pub const MAX_SYMLINK_RESOLVE_DEPTH: usize = 8;

/// `\Sessions\` prefix for session-local namespace routing.
pub const SESSIONS_PREFIX: &[u8] = br"\Sessions\";

/// Single-session bring-up: default interactive window station under session 0.
pub const SESSION0_WINSTA0: &[u8] = br"\Sessions\0\WinSta0";
/// Default desktop on `WinSta0` (login / apps share this path in the simplified model).
pub const SESSION0_DESKTOP_DEFAULT: &[u8] = br"\Sessions\0\WinSta0\Default";

/// Public-docs alias: `\Windows\WindowStations\` (session 0 bring-up only maps `WinSta0` below).
///
/// Lookup helpers rewrite this prefix to [`SESSION0_WINSTA0`] so paths match
/// [`super::winsta::lookup_session_winsta_desktop_path`] without duplicating mounts.
pub const WINDOWS_WINSTATIONS_PREFIX: &[u8] = br"\Windows\WindowStations\";

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

/// Rewrites `\Windows\WindowStations\WinSta0[...]` into `\Sessions\0\WinSta0[...]` for object lookup.
///
/// Returns byte length written to `out` on success. Other station names are rejected in this bring-up alias.
#[must_use]
pub fn normalize_winsta_path_to_sessions(path: &[u8], out: &mut [u8]) -> Option<usize> {
    let rest = path.strip_prefix(WINDOWS_WINSTATIONS_PREFIX)?;
    if !rest.starts_with(b"WinSta0") {
        return None;
    }
    let tail = rest.strip_prefix(b"WinSta0").unwrap_or(&[]);
    if !tail.is_empty() && tail[0] != b'\\' {
        return None;
    }
    let base = SESSION0_WINSTA0;
    let n = base.len().saturating_add(tail.len());
    if n > out.len() {
        return None;
    }
    out[..base.len()].copy_from_slice(base);
    out[base.len()..n].copy_from_slice(tail);
    Some(n)
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

    pub fn remove_session_child(&mut self, session_id: u8, name: &[u8]) -> Result<(), ()> {
        let sid = session_id as usize;
        if sid >= MAX_SESSION_DIRS {
            return Err(());
        }
        self.by_session[sid].remove(name)
    }

    /// Like [`Self::insert_session_path`], but denies cross-session namespace writes unless the path session matches
    /// [`crate::se::token::SecurityToken::session_id`] (bring-up DAC hook).
    pub fn insert_session_path_for_token(
        &mut self,
        token: &crate::se::token::SecurityToken,
        path: &[u8],
        object: NonNull<()>,
    ) -> Result<(), ()> {
        let rest = strip_sessions_subpath(path).ok_or(())?;
        let (sid, _) = parse_session_id_and_name(rest).ok_or(())?;
        if sid as u32 != token.session_id {
            return Err(());
        }
        self.insert_session_path(path, object)
    }

    /// Lookup after DAC ([`crate::se::acl::access_check_sid_equal`]) and session match on `path`.
    #[must_use]
    pub fn lookup_session_path_for_token(
        &self,
        token: &crate::se::token::SecurityToken,
        path: &[u8],
        resource_owner: &crate::se::sid::Sid,
    ) -> Option<NonNull<()>> {
        if token.access_check_vs_owner(resource_owner) != crate::se::acl::AccessCheckResult::Granted {
            return None;
        }
        let rest = strip_sessions_subpath(path)?;
        let (sid, _) = parse_session_id_and_name(rest)?;
        if sid as u32 != token.session_id {
            return None;
        }
        self.lookup_session_path(path)
    }
}

/// Split `WinSta0\Default` into `WinSta0` + `Some(Default)`; `WinSta0` → `None` desktop.
#[must_use]
pub fn split_first_path_segment(s: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    if s.is_empty() {
        return None;
    }
    if let Some(i) = s.iter().position(|&c| c == b'\\') {
        let left = &s[..i];
        let right = &s[i + 1..];
        if left.is_empty() {
            return None;
        }
        Some((left, Some(right)))
    } else {
        Some((s, None))
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
    use crate::se::token::SecurityToken;

    #[test]
    fn insert_session_path_for_token_denies_cross_session() {
        let mut ns = NamespaceBuckets::new();
        let tok0 = SecurityToken::system_bootstrap();
        let p = NonNull::new(0x1000usize as *mut ()).unwrap();
        assert!(ns
            .insert_session_path_for_token(&tok0, br"\Sessions\0\Obj", p)
            .is_ok());
        assert!(ns
            .insert_session_path_for_token(&tok0, br"\Sessions\1\Other", p)
            .is_err());
    }

    #[test]
    fn lookup_session_path_for_token_requires_dac_and_session() {
        use crate::se::sid::Sid;
        let mut ns = NamespaceBuckets::new();
        let p = NonNull::new(0x2000usize as *mut ()).unwrap();
        ns.insert_session_path(br"\Sessions\0\Res", p).unwrap();
        let tok = SecurityToken::system_bootstrap();
        let world = Sid::well_known_world();
        assert_eq!(
            ns.lookup_session_path_for_token(&tok, br"\Sessions\0\Res", &world),
            Some(p)
        );
        let other = Sid {
            revision: 1,
            sub_auth_count: 1,
            identifier_authority: [0, 0, 0, 0, 0, 5],
            sub_authority: [99, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(ns
            .lookup_session_path_for_token(&tok, br"\Sessions\0\Res", &other)
            .is_none());
    }

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

    #[test]
    fn legacy_windows_winstations_alias() {
        let mut buf = [0u8; 96];
        let n = normalize_winsta_path_to_sessions(br"\Windows\WindowStations\WinSta0\Default", &mut buf)
            .expect("alias");
        assert_eq!(&buf[..n], SESSION0_DESKTOP_DEFAULT);
        assert!(normalize_winsta_path_to_sessions(br"\Windows\WindowStations\Other\X", &mut buf).is_none());
    }
}
