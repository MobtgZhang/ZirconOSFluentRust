//! Canonical `\Device\...` style paths (object namespace bring-up).

/// Named pipe device root (Win32 `\\.\pipe\Foo` → `\Device\NamedPipe\Foo` on Windows).
pub const DEVICE_NAMED_PIPE_PREFIX: &[u8] = br"\Device\NamedPipe\";

/// Mailslot root (classic Win32 IPC — far future).
pub const DEVICE_MAILSLOT_PREFIX: &[u8] = br"\Device\Mailslot\";
