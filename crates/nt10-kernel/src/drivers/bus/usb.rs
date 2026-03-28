//! USB device model and descriptor constants (bring-up; no hub/TTH).

/// Standard device request: GET_DESCRIPTOR.
pub const REQ_GET_DESCRIPTOR: u8 = 0x06;
/// Standard device request: SET_CONFIGURATION.
pub const REQ_SET_CONFIGURATION: u8 = 0x09;
/// Standard interface request: SET_INTERFACE (optional).
pub const REQ_SET_INTERFACE: u8 = 0x0B;

pub const DESC_DEVICE: u8 = 1;
pub const DESC_CONFIGURATION: u8 = 2;
pub const DESC_STRING: u8 = 3;
pub const DESC_HID: u8 = 0x21;
pub const DESC_HID_REPORT: u8 = 0x22;

pub const HID_REQ_SET_IDLE: u8 = 0x0A;
pub const HID_REQ_SET_PROTOCOL: u8 = 0x0B;

/// Boot protocol.
pub const HID_PROTOCOL_BOOT: u16 = 0;
/// Report protocol.
pub const HID_PROTOCOL_REPORT: u16 = 1;

#[derive(Clone, Copy, Debug, Default)]
pub struct UsbDeviceDescriptor {
    pub b_max_packet0: u8,
    pub id_vendor: u16,
    pub id_product: u16,
    pub b_num_configurations: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UsbInterfaceSummary {
    pub interface_number: u8,
    pub alternate_setting: u8,
    pub class: u8,
    pub subclass: u8,
    pub protocol: u8,
    pub endpoint_in: u8,
    pub max_packet_size: u16,
    pub interval: u8,
}

impl UsbInterfaceSummary {
    #[must_use]
    pub const fn is_hid_boot_keyboard(self) -> bool {
        self.class == 0x03 && self.subclass == 0x01 && self.protocol == 0x01
    }

    #[must_use]
    pub const fn is_hid_boot_mouse(self) -> bool {
        self.class == 0x03 && self.subclass == 0x01 && self.protocol == 0x02
    }

    /// HID pointer source: boot mouse (`protocol == 2`), QEMU `usb-tablet` (`protocol == 0`), etc.
    /// Excludes boot keyboard (`subclass/protocol` keyboard).
    #[must_use]
    pub const fn is_hid_pointer_interface(self) -> bool {
        self.class == 0x03
            && (self.endpoint_in & 0x80) != 0
            && !self.is_hid_boot_keyboard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qemu_tablet_iface_is_pointer_not_boot_mouse() {
        let t = UsbInterfaceSummary {
            interface_number: 0,
            alternate_setting: 0,
            class: 0x03,
            subclass: 0x00,
            protocol: 0x00,
            endpoint_in: 0x81,
            max_packet_size: 8,
            interval: 10,
        };
        assert!(!t.is_hid_boot_mouse());
        assert!(t.is_hid_pointer_interface());
    }
}
