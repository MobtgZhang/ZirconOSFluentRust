# Build system, testing, and coding standards

**中文**：[../cn/Build-Test-Coding.md](../cn/Build-Test-Coding.md)

This covers draft §24, §26, §27 and contrasts the **current Cargo workspace** with the full kernel project described in the draft.

## 1. Build system

### 1.1 Target kernel project (draft §24 — **planned**)

The commands below describe a **future** expanded workflow (feature flags, ISO, one-shot QEMU). The repo does **not** implement all of this yet.

```bash
cargo build -p nt10-kernel --release --target x86_64-unknown-none
cargo xtask iso          # planned: xtask or shell scripts
cargo xtask run-qemu     # planned
cargo test               # planned: host-testable crates or cfg(test) strategy
```

Draft feature concepts (`enable_vbs`, `enable_hyperv`, `enable_la57`, …) map naturally to **Cargo features** or **`cfg` flags**; exact names should stay in sync with [Cargo.toml](../../Cargo.toml).

### 1.2 **Current** Cargo workspace

| Item | Notes |
|------|------|
| Root manifest | [Cargo.toml](../../Cargo.toml): members `nt10-kernel`, `nt10-boot-uefi` |
| Toolchain | [rust-toolchain.toml](../../rust-toolchain.toml): stable + `x86_64-unknown-none` |
| Kernel library | [crates/nt10-kernel/](../../crates/nt10-kernel/): `#![no_std]` rlib; **no** linker script injected (avoids conflicting `-T` with the executable crate) |
| Bootable kernel ELF | [crates/nt10-kernel-bin/](../../crates/nt10-kernel-bin/): [build.rs](../../crates/nt10-kernel-bin/build.rs) passes [link/x86_64-uefi-load.ld](../../link/x86_64-uefi-load.ld) for `x86_64-unknown-none` (physical `0x100000`); [link/x86_64.ld](../../link/x86_64.ld) is reference-only for higher-half experiments |
| Alias | [.cargo/config.toml](../../.cargo/config.toml): `cargo kcheck` → `check -p nt10-kernel --target x86_64-unknown-none` |
| Boot stub | [crates/nt10-boot-uefi/](../../crates/nt10-boot-uefi/): no `uefi-rs` yet |

**Typical commands**:

```bash
rustup target add x86_64-unknown-none
cargo check --workspace
cargo check -p nt10-kernel --target x86_64-unknown-none
# or
cargo kcheck
```

### 1.3 Scripts and assets

| Script | Role |
|--------|------|
| [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh) | QEMU + OVMF; by default runs [`scripts/pack-esp.sh`](../../scripts/pack-esp.sh) to build an ESP with `BOOTX64.EFI` and `EFI/ZirconOS/NT10KRNL.BIN`; `PROFILE=release` uses release artifacts for the temp ESP |
| [`scripts/pack-esp.sh`](../../scripts/pack-esp.sh) | Builds `zbm10.efi` and a flat kernel `.BIN` into a given ESP directory; `PROFILE=release` maps to `cargo --release` |
| [`xtask/`](../../xtask/) | `cargo run -p xtask -- build|pack-esp|qemu|qemu-kernel` wraps the scripts (`--release` sets `PROFILE=release`) |
| [`scripts/generate-resource-icons.py`](../../scripts/generate-resource-icons.py) | Exports multi-size PNGs from `resources/icons/_sources/` (`pip install pillow`) |

ISO generation remains future work; **xtask** provides `build` / `pack-esp` / `qemu` entry points.

**UEFI temp ESP layout** ([`scripts/pack-esp.sh`](../../scripts/pack-esp.sh)): `EFI/BOOT/BOOTX64.EFI` (ZBM10), `EFI/ZirconOS/NT10KRNL.BIN` (flat kernel), root `startup.nsh` (helps when OVMF returns `Unsupported` for QEMU `fat:` default boot and the Shell countdown runs `BOOTX64.EFI`). Optional `EFI/ZirconOS/zbm10.cfg` (e.g. `kernel=MYKRNL.BIN`). OVMF: monolithic firmware uses QEMU `-bios`; split `*CODE*.fd` pairs with `OVMF_VARS.fd` as dual pflash (see [`run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh)).

**LoongArch UEFI**: when `r-efi` and Rust expose `loongarch64-unknown-uefi`, add a matching target and linker script for `nt10-boot-uefi` alongside x86_64.

## 2. Testing (draft §26 — target kernel)

- **Unit**: `tests/` or per-crate `#[cfg(test)]`; `no_std` kernel code may need host mocks or a small host-only crate for algorithms.
- **Integration**: QEMU smoke — **planned**.
- **Conformance**: NT10 / ReactOS-style suites — **planned**.
- **CI**: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) runs `cargo check --workspace`, `cargo kcheck`, and a UEFI boot build.

**Today**: **`cargo check` green** is the baseline; host-side unit tests may exist for pure logic (e.g. `cargo test -p nt10-kernel` for `mm::vad`). **QEMU / serial** milestones (`nt10-kernel: …` on COM1) are described in [scripts/run-qemu-x86_64.sh](../../scripts/run-qemu-x86_64.sh) and the [Makefile](../../Makefile) `run-debug` target.

## 3. Coding standards (draft §27, Rust)

### 3.1 Naming

| Kind | Style | Example |
|------|------|---------|
| Types | PascalCase | `EProcess`, `ObjectHeader` |
| Functions / modules | snake_case | `alloc_phys_frame` |
| Constants | SCREAMING_SNAKE | `PAGE_SIZE` |
| NT exports | PascalCase + `Nt` | `NtCreateFile` |

### 3.2 Rust usage

- Layout-sensitive / NT-interop types: `#[repr(C)]` (or explicit `repr`).
- **Exported NT APIs**: map results to `NTSTATUS` (or a single newtype — pick one convention).
- **IRQL ≥ DISPATCH_LEVEL**: no paged allocations, no faulting accesses; document IRQL in `unsafe` blocks.
- Never dereference user pointers without a probe/validation path.
- HAL-style polymorphism: traits, generics, or `cfg(target_arch)` instead of the draft’s comptime wording.

### 3.3 Safety

- Every `unsafe` block documents **invariants** and **caller obligations**.
- ISRs save full register state; DPC paths stay in non-paged memory only.

## 4. Related docs

- [PROCESS_NT10.md](PROCESS_NT10.md)
- [Architecture.md](Architecture.md)
