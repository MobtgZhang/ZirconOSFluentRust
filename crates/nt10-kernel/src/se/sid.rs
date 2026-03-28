//! Security Identifier (binary form).

/// Max sub-authorities in this bring-up SID.
pub const MAX_SUB_AUTHS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sid {
    pub revision: u8,
    pub sub_auth_count: u8,
    pub identifier_authority: [u8; 6],
    pub sub_authority: [u32; MAX_SUB_AUTHS],
}

impl Sid {
    pub const fn well_known_world() -> Self {
        Self {
            revision: 1,
            sub_auth_count: 1,
            identifier_authority: [0, 0, 0, 0, 0, 1],
            sub_authority: [1, 0, 0, 0, 0, 0, 0, 0],
        }
    }
}
