//! `IShellFolder` / `IShellItem`-shaped traits for VFS-backed enumeration (no COM vtable yet).

/// Minimal folder enumeration for [`super::explorer_view`] migration.
pub trait ShellFolder {
    /// Writes up to `out.len()` child display names; returns count placed.
    fn enumerate_children(&self, out: &mut [&[u8]]) -> usize;
}

/// Lightweight item id (path slice into static / pooled storage).
#[derive(Clone, Copy, Debug)]
pub struct ShellItemId<'a> {
    pub path: &'a [u8],
}

pub trait ShellItem {
    fn id(&self) -> ShellItemId<'_>;
}

/// Root of the Zircon bring-up namespace (single synthetic listing).
pub struct RootShellFolder;

impl ShellFolder for RootShellFolder {
    fn enumerate_children(&self, out: &mut [&[u8]]) -> usize {
        let entries: [&[u8]; 2] = [b"Zircon Files", b"This PC"];
        let n = entries.len().min(out.len());
        out[..n].copy_from_slice(&entries[..n]);
        n
    }
}
