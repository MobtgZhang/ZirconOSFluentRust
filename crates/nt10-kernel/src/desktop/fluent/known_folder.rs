//! KNOWNFOLDERID-style buckets mapped to ZirconOS VFS bring-up paths.

/// Subset of known-folder semantics (no shell PIDL yet).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum KnownFolderId {
    Desktop,
    Documents,
    Downloads,
    RecycleBin,
}

/// Virtual path hints for the in-kernel explorer (`explorer_view`); adjust when VFS roots grow.
#[must_use]
pub fn zircon_path_hint(id: KnownFolderId) -> &'static [u8] {
    match id {
        KnownFolderId::Desktop => b"A:\\Users\\Public\\Desktop",
        KnownFolderId::Documents => b"A:\\Users\\Public\\Documents",
        KnownFolderId::Downloads => b"A:\\Users\\Public\\Downloads",
        KnownFolderId::RecycleBin => b"A:\\$Recycle.Bin",
    }
}
