# Memory manager (MM) and object manager (OB)

**中文**：[../cn/Memory-and-Objects.md](../cn/Memory-and-Objects.md)

Draft §8–§9. **Status**: design target; stubs under [crates/nt10-kernel/src/mm/](../../crates/nt10-kernel/src/mm/) and [ob/](../../crates/nt10-kernel/src/ob/).

## 1. Memory manager (MM)

### 1.1 Physical memory

- Early: **bitmap allocator**; later: **buddy system**.
- 4 KiB, 2 MiB, 1 GiB pages.
- **NUMA**: per-node free lists and policies.

### 1.2 Virtual layout (x86_64, LA57 off)

| Range | Use |
|-------|-----|
| `0x0000…` – `0x00007FFF_FFFF_FFFF` | User (128 TiB) |
| `0xFFFF8000…` – `0xFFFF8FFF_FFFF_FFFF` | Direct map (16 TiB) |
| `0xFFFF9000…` – `0xFFFF9FFF_FFFF_FFFF` | vmalloc |
| `0xFFFFA000…` – `0xFFFFAFFF_FFFF_FFFF` | NonPagedPool |
| `0xFFFFB000…` – `0xFFFFBFFF_FFFF_FFFF` | PagedPool |
| `0xFFFFF800…` – `0xFFFFFFFFFFFFFFFF` | Kernel image, HAL, PFN DB, … |

**LA57**: optional 5-level paging; see draft §23 and [arch/x86_64/paging.rs](../../crates/nt10-kernel/src/arch/x86_64/paging.rs).

### 1.3 VAD tree

User VAs managed with a **VAD AVL tree**: region type (heap/stack/map/reserve), protection (`PAGE_EXECUTE_READ`, …), commit state.

### 1.4 Section objects

`NtCreateSection` plans:

- Image sections (`SEC_IMAGE`)
- Anonymous committed sections
- **COW**

### 1.5 Modules in this repo

[mm/phys.rs](../../crates/nt10-kernel/src/mm/phys.rs), `buddy.rs`, `pfn.rs`, `pt.rs`, `pool.rs`, `vm.rs`, `paging.rs`, `heap.rs`, `large_page.rs`, `numa.rs`, `pagefile.rs`, `section.rs`, `vad.rs`, `working_set.rs`, …

### 1.6 Managed runtime and MM (.NET)

Many Windows applications depend on **.NET**. Those processes use `VirtualAlloc` / `NtAllocateVirtualMemory` with heavy **reserve/commit** patterns; JIT code needs **executable mappings** consistent with **NX / DEP**. The kernel should provide documented **VAD, demand paging, and page protections** (MM-P3/MM-P4) without special-casing “managed” vs native. Policy: [DotNet-UserMode.md](DotNet-UserMode.md).

## 2. Object manager (OB)

Each object has a header; draft-style Rust sketch:

```rust
#[repr(C)]
pub struct ObjectHeader {
    pub pointer_count: i64,
    pub handle_count: i64,
    pub type_index: u8,
    pub flags: ObjectFlags,
    pub security_descriptor: *mut SecurityDescriptor,
}
```

Optional headers (name, creator, quota) may precede this in memory (match real NT layouts carefully).

### 2.1 Namespace (logical)

```
\
├── Device\
├── ObjectTypes\
├── KnownDlls\
├── Windows\WindowStations\WinSta0\Default\
├── Sessions\
└── GLOBAL?? (DOS devices, e.g. C: → volume)
```

### 2.2 Modules in this repo

[ob/object.rs](../../crates/nt10-kernel/src/ob/object.rs), `handle.rs`, `namespace.rs`, `symlink.rs`, `directory.rs`, `wait.rs`, …

## 3. Related docs

- [DotNet-UserMode.md](DotNet-UserMode.md)
- [Processes-Security-IO.md](Processes-Security-IO.md)
- [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md)
