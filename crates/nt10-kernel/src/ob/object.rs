//! Object header (reference-counted kernel object base).

/// Per-type index into a static type table (bring-up).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectTypeIndex(pub u8);

#[repr(C)]
pub struct ObjectHeader {
    pub pointer_count: i64,
    pub handle_count: i32,
    pub type_index: ObjectTypeIndex,
    pub flags: u32,
}

impl ObjectHeader {
    pub const fn new(ty: ObjectTypeIndex) -> Self {
        Self {
            pointer_count: 1,
            handle_count: 0,
            type_index: ty,
            flags: 0,
        }
    }

    pub fn reference(&mut self) {
        self.pointer_count += 1;
    }

    pub fn dereference(&mut self) -> bool {
        self.pointer_count -= 1;
        self.pointer_count == 0
    }

    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.pointer_count > 0
    }
}
