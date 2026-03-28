//! HVCI / VTL / CFG policy hooks (no hypervisor implementation).

use core::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SecurityStrictness {
    #[default]
    Relaxed,
    Strict,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HvciPolicy {
    pub enforce_signing: bool,
    pub block_dynamic_code: bool,
    pub strictness: SecurityStrictness,
    /// Forward CFG checks at load time (stub).
    pub cfg_enforced: bool,
}

impl HvciPolicy {
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            enforce_signing: false,
            block_dynamic_code: false,
            strictness: SecurityStrictness::Relaxed,
            cfg_enforced: false,
        }
    }

    #[must_use]
    pub const fn strict_stub() -> Self {
        Self {
            enforce_signing: true,
            block_dynamic_code: true,
            strictness: SecurityStrictness::Strict,
            cfg_enforced: true,
        }
    }
}

static STRICT_TAG: AtomicU8 = AtomicU8::new(0);
static CFG_ENFORCED: AtomicU8 = AtomicU8::new(0);

fn tag_strictness(t: u8) -> SecurityStrictness {
    if t == 1 {
        SecurityStrictness::Strict
    } else {
        SecurityStrictness::Relaxed
    }
}

/// Default is relaxed; loaders may query before mapping images.
#[must_use]
pub fn hvci_runtime_policy() -> HvciPolicy {
    let strictness = tag_strictness(STRICT_TAG.load(Ordering::Relaxed));
    let cfg_enforced = CFG_ENFORCED.load(Ordering::Relaxed) != 0;
    let strict = strictness == SecurityStrictness::Strict;
    HvciPolicy {
        enforce_signing: strict,
        block_dynamic_code: strict,
        strictness,
        cfg_enforced: cfg_enforced || strict,
    }
}

pub fn set_hvci_runtime_policy(p: HvciPolicy) {
    let t = match p.strictness {
        SecurityStrictness::Relaxed => 0u8,
        SecurityStrictness::Strict => 1u8,
    };
    STRICT_TAG.store(t, Ordering::Release);
    CFG_ENFORCED.store(u8::from(p.cfg_enforced), Ordering::Release);
}
