# Loader, Win32k/WDDM, user-mode libraries, sessions — repo mapping

**中文**：[../cn/Loader-Win32k-Desktop.md](../cn/Loader-Win32k-Desktop.md)

Draft §15–§19 plus where **this repo** keeps Loader / Win32 / Fluent **Rust stubs**.

## 1. Loader

### 1.1 PE32+ load flow (planned)

```
NtCreateSection(SEC_IMAGE)
  → parse MZ / COFF / optional header
  → validate Machine / Subsystem / DllCharacteristics
  → map .text / .data / .rdata / .rsrc …
  → relocate (ASLR)
  → resolve imports, load DLLs recursively
  → page protections (NX / RO / RW)
  → CFG bitmap if enabled
  → TLS
  → fill PEB.Ldr
```

### 1.2 ASLR

Image, heap, stack randomization — [loader/aslr.rs](../../crates/nt10-kernel/src/loader/aslr.rs).

### 1.3 Paths in this repo

[loader/pe.rs](../../crates/nt10-kernel/src/loader/pe.rs), `pe32.rs`, `elf.rs` (WSL stub), `import_.rs`, `reloc.rs`, `aslr.rs`.

## 2. Win32k and WDDM 2.x (target)

```
User mode                    Kernel mode
D3D12 runtime    →  dxgkrnl
user32 / gdi32   →  win32k.sys
                   → wddm2 / VidPN / display DDI
                   → framebuffer (QEMU GOP phase)
```

**DWM** (target): per-window offscreen buffers, alpha compose, Acrylic, Mica, animations.

**This repo**: Win32k/WDDM are **stubs** under [drivers/video/wddm2/](../../crates/nt10-kernel/src/drivers/video/wddm2/), etc.

## 3. User-mode API libraries (target)

- **ntdll**: `Nt*` / `Zw*` / `Rtl*` / `Ldr*`, NT10 (19041) syscall numbers.
- **kernel32 / kernelbase**: NT6+ split; most APIs in kernelbase.
- **combase / winrt_rt**: COM and WinRT (`RoActivateInstance`, …) — draft §17, §22.

**Stubs**: [libs/](../../crates/nt10-kernel/src/libs/).

## 4. Session manager and Win32 subsystem (target)

- **SMSS**: page file, `\Sessions\0`, ApiPort, csrss / wininit.
- **csrss**: window stations, desktops, message queues, Csr* registration.
- **ConPTY**: conhost, VT sequences — draft §19.

Boot chain (summary): kernel phases → `smss` → `csrss` → `wininit` → services / lsass / winlogon → explorer.

**Stubs**: [servers/](../../crates/nt10-kernel/src/servers/), [subsystems/win32/](../../crates/nt10-kernel/src/subsystems/win32/).

---

## 5. Fluent desktop (this repo)

There is **no** runnable Fluent shell or host binary yet. Phase-14 visuals are represented only by **[desktop/fluent/](../../crates/nt10-kernel/src/desktop/fluent/)** stubs (`shell.rs`, `acrylic.rs`, `mica.rs`, `dwm.rs`, …) for future Win32k/DWM work.

Static assets live under the root **[`resources/`](../../resources/)** with a machine-readable **[`resources/manifest.json`](../../resources/manifest.json)** (wallpapers, multi-size icons, `misc/`, etc.; see [`resources/README.md`](../../resources/README.md) for provenance notes).

## 6. Related docs

- [Architecture.md](Architecture.md) §5
- [Roadmap-and-TODO.md](Roadmap-and-TODO.md) (Phase 14)
- [Build-Test-Coding.md](Build-Test-Coding.md)
