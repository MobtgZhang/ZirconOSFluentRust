//! WinRT runtime — AppContainer / activation stubs (no Windows OS code;
//! [`crate::milestones::PHASE_WINRT`]).

/// ZirconOS-local runtime class id (opaque index, not a Windows CLSID).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ZrRuntimeClassId(pub u32);

/// Placeholder AppContainer SID index (ZirconOS-local numbering).
#[derive(Clone, Copy, Debug)]
pub struct AppContainerId(pub u32);

/// ConPTY pair (user/kernel boundary) — hook for future console host.
#[derive(Clone, Copy, Debug)]
pub struct ConptyPair {
    pub host_handle: u64,
    pub client_handle: u64,
}

impl ConptyPair {
    pub const fn disabled() -> Self {
        Self {
            host_handle: 0,
            client_handle: 0,
        }
    }
}

/// Packaged app activation factory token (opaque).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActivationFactoryRef(pub u64);

static ACTIVATION_TABLE: &[(ZrRuntimeClassId, u64)] = &[
    (ZrRuntimeClassId(0x5A01), 0x8000_0000_0000_0001),
    (ZrRuntimeClassId(0x5A02), 0x8000_0000_0000_0002),
];

/// `RoActivateInstance`-style lookup into a static ZirconOS class table.
#[must_use]
pub fn ro_activate_instance_zircon(class_id: ZrRuntimeClassId) -> ActivationFactoryRef {
    for (id, handle) in ACTIVATION_TABLE {
        if id.0 == class_id.0 {
            return ActivationFactoryRef(*handle);
        }
    }
    ActivationFactoryRef(0)
}

/// Legacy stub name used by older call sites.
#[must_use]
pub fn ro_activate_instance_stub(_class_name_utf16: *const u16) -> ActivationFactoryRef {
    ActivationFactoryRef(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_class_returns_handle() {
        assert_eq!(
            ro_activate_instance_zircon(ZrRuntimeClassId(0x5A01)).0,
            0x8000_0000_0000_0001
        );
    }

    #[test]
    fn unknown_class_null() {
        assert_eq!(ro_activate_instance_zircon(ZrRuntimeClassId(0)).0, 0);
    }
}
