# NT10 development process and documentation

**中文**: [../cn/PROCESS_NT10.md](../cn/PROCESS_NT10.md)

## 1. Docs vs source draft

- **Published set**: `docs/cn/*.md`, `docs/en/*.md` (paired filenames).
- **Single-file master**: [ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md).

When design changes:

1. Update either **ideas** or **docs** first (pick one source of truth), then sync the other.
2. When updating **Phase status**, edit both `docs/cn/Roadmap-and-TODO.md` and `docs/en/Roadmap-and-TODO.md`.

## 2. Where new code should live (this repo, Rust)

Draft §4 `src/...` maps to the **Cargo workspace** as follows:

- **UEFI boot** → [crates/nt10-boot-uefi/](../../crates/nt10-boot-uefi/) (ZBM10; optional future `boot/zbm10/` assets)
- **Arch** → [crates/nt10-kernel/src/arch/<arch>/](../../crates/nt10-kernel/src/arch/)
- **HAL** → [crates/nt10-kernel/src/hal/<arch>/](../../crates/nt10-kernel/src/hal/)
- **Executive / MM / OB / PS / SE / IO / ALPC / FS** → [ke/](../../crates/nt10-kernel/src/ke/), [mm/](../../crates/nt10-kernel/src/mm/), [ob/](../../crates/nt10-kernel/src/ob/), [ps/](../../crates/nt10-kernel/src/ps/), [se/](../../crates/nt10-kernel/src/se/), [io/](../../crates/nt10-kernel/src/io/), [alpc/](../../crates/nt10-kernel/src/alpc/), [fs/](../../crates/nt10-kernel/src/fs/)
- **Drivers** → [drivers/](../../crates/nt10-kernel/src/drivers/)
- **User-mode libs (stubs / shared types)** → [libs/](../../crates/nt10-kernel/src/libs/)
- **System services** → [servers/](../../crates/nt10-kernel/src/servers/)
- **Win32 subsystem** → [subsystems/win32/](../../crates/nt10-kernel/src/subsystems/win32/)
- **Fluent desktop (stubs)** → [desktop/fluent/](../../crates/nt10-kernel/src/desktop/fluent/)

## 3. Resource paths

Static assets live under the root **[`resources/`](../../resources/)** with **[`resources/manifest.json`](../../resources/manifest.json)**.

## 4. Review expectations

- Kernel behavior changes: update **both** CN and EN sections.
- Cross-cutting changes: refresh “Related docs” in [Architecture.md](Architecture.md) or the relevant volume.
- Trademarks / third-party content: no unlicensed assets; align with any future `LICENSE` and manifest.
- Offline trees under `references/` (e.g. Learn-style prose): read [References-Policy.md](References-Policy.md) before copying text into the repo.

## 5. Index

- [docs/en/README.md](README.md)
- [docs/README.md](../README.md)
