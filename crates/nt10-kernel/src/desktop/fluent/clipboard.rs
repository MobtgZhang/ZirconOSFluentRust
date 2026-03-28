//! Session / window-station scoped clipboard (minimal `CF_UNICODETEXT` + `CF_HDROP` placeholder).
//!
//! Full Win32 clipboard chains per window station; single-slot bring-up ignores `hwnd` owner.

use crate::ke::spinlock::SpinLock;

/// Standard clipboard format ids (Win32 public values).
pub const CF_UNICODETEXT: u32 = 13;
pub const CF_HDROP: u32 = 15;

const TEXT_CAP: usize = 512;
const HDROP_CAP: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveFormat {
    None,
    UnicodeText,
    Hdrop,
}

struct ClipState {
    active: ActiveFormat,
    text_utf8: [u8; TEXT_CAP],
    text_len: usize,
    hdrop: [u8; HDROP_CAP],
    hdrop_len: usize,
}

impl ClipState {
    const fn empty() -> Self {
        Self {
            active: ActiveFormat::None,
            text_utf8: [0u8; TEXT_CAP],
            text_len: 0,
            hdrop: [0u8; HDROP_CAP],
            hdrop_len: 0,
        }
    }
}

static CLIP: SpinLock<ClipState> = SpinLock::new(ClipState::empty());

/// Store UTF-8 text as `CF_UNICODETEXT` stand-in until UTF-16 heap buffers exist.
pub fn set_clipboard_unicodetext_utf8(text: &[u8]) -> Result<(), ()> {
    if text.len() > TEXT_CAP {
        return Err(());
    }
    let mut g = CLIP.lock();
    g.text_utf8[..text.len()].copy_from_slice(text);
    g.text_len = text.len();
    g.active = ActiveFormat::UnicodeText;
    Ok(())
}

/// Copy active text into `dst` (UTF-8 bring-up).
#[must_use]
pub fn get_clipboard_text_utf8(dst: &mut [u8]) -> Option<usize> {
    let g = CLIP.lock();
    if g.active != ActiveFormat::UnicodeText {
        return None;
    }
    let n = g.text_len.min(dst.len());
    dst[..n].copy_from_slice(&g.text_utf8[..n]);
    Some(n)
}

/// Placeholder: store opaque `CF_HDROP` payload (paths blob).
pub fn set_clipboard_hdrop(data: &[u8]) -> Result<(), ()> {
    if data.len() > HDROP_CAP {
        return Err(());
    }
    let mut g = CLIP.lock();
    g.hdrop[..data.len()].copy_from_slice(data);
    g.hdrop_len = data.len();
    g.active = ActiveFormat::Hdrop;
    Ok(())
}

#[must_use]
pub fn clipboard_format_active() -> u32 {
    match CLIP.lock().active {
        ActiveFormat::None => 0,
        ActiveFormat::UnicodeText => CF_UNICODETEXT,
        ActiveFormat::Hdrop => CF_HDROP,
    }
}
