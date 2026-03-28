# 构建系统、测试策略与编码规范

**English**: [../en/Build-Test-Coding.md](../en/Build-Test-Coding.md)

本文对应母版 §24、§26、§27，并说明 **本仓库当前 Cargo 工作区** 与母版「完整内核工程」目标的差异。

## 1. 构建系统

### 1.1 目标内核工程（母版 §24 — **规划**）

以下命令描述 **未来** 扩展后的预期体验（特性开关、ISO、QEMU 一键运行等），**不等于** 当前仓库已具备全部能力：

```bash
cargo build -p nt10-kernel --release --target x86_64-unknown-none
cargo xtask iso          # 规划：需 xtask 或 shell 脚本
cargo xtask run-qemu     # 规划
cargo test               # 规划：host 可测 crate 或 cfg(test) 策略
```

母版中的特性开关概念（`enable_vbs`、`enable_hyperv`、`enable_la57` 等）在 Rust 侧可通过 **Cargo features** 或 **`cfg` 标志** 落地；具体命名待与 [Cargo.toml](../../Cargo.toml) 同步。

### 1.2 本仓库 **当前** Cargo 工作区（事实）

| 项 | 说明 |
|----|------|
| 根清单 | [Cargo.toml](../../Cargo.toml)：`nt10-kernel`、`nt10-boot-uefi` |
| 工具链 | [rust-toolchain.toml](../../rust-toolchain.toml)：stable + `x86_64-unknown-none` |
| 内核库 | [crates/nt10-kernel/](../../crates/nt10-kernel/)：`#![no_std]` rlib，**不**注入链接脚本（避免与可执行 crate 的 `-T` 冲突） |
| 可引导内核 ELF | [crates/nt10-kernel-bin/](../../crates/nt10-kernel-bin/)：[build.rs](../../crates/nt10-kernel-bin/build.rs) 在 `x86_64-unknown-none` 下传入 [link/x86_64-uefi-load.ld](../../link/x86_64-uefi-load.ld)（物理 `0x100000`）；[link/x86_64.ld](../../link/x86_64.ld) 仅作高半核实验参考 |
| 快捷别名 | [.cargo/config.toml](../../.cargo/config.toml)：`cargo kcheck` → `check -p nt10-kernel --target x86_64-unknown-none` |
| 引导占位 | [crates/nt10-boot-uefi/](../../crates/nt10-boot-uefi/)：无 `uefi-rs` 依赖 |

**常用命令**：

```bash
rustup target add x86_64-unknown-none
cargo check --workspace
cargo check -p nt10-kernel --target x86_64-unknown-none
# 或
cargo kcheck
```

### 1.3 脚本与资源

| 脚本 | 说明 |
|------|------|
| [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh) | QEMU + OVMF；默认调用 [`scripts/pack-esp.sh`](../../scripts/pack-esp.sh) 生成含 `BOOTX64.EFI` 与 `EFI/ZirconOS/NT10KRNL.BIN` 的 ESP；环境变量 `PROFILE=release` 时临时 ESP 使用 release 产物 |
| [`scripts/pack-esp.sh`](../../scripts/pack-esp.sh) | 构建 `zbm10.efi` 与扁平内核二进制并写入指定目录；`PROFILE=release` 等价于 `cargo --release` |
| [`xtask/`](../../xtask/) | `cargo run -p xtask -- build|pack-esp|qemu|qemu-kernel` 封装上述脚本与构建（`--release` 会设置 `PROFILE=release`） |
| [`scripts/generate-resource-icons.py`](../../scripts/generate-resource-icons.py) | 自 `resources/icons/_sources/` 导出多尺寸 PNG（需 `pip install pillow`） |

ISO 镜像生成仍为后续规划；**xtask** 已提供 `build` / `pack-esp` / `qemu` 入口。

**UEFI 临时 ESP 布局**（[`scripts/pack-esp.sh`](../../scripts/pack-esp.sh)）：`EFI/BOOT/BOOTX64.EFI`（ZBM10）、`EFI/ZirconOS/NT10KRNL.BIN`（扁平内核）、根目录 `startup.nsh`（缓解部分 OVMF 在 QEMU `fat:` 盘上默认启动项 `Unsupported`、进入 Shell 后可自动拉起 `BOOTX64.EFI`）。可选 `EFI/ZirconOS/zbm10.cfg`（如 `kernel=MYKRNL.BIN`）。OVMF：合并镜像用 QEMU `-bios`；分体 `*CODE*.fd` 与同目录 `OVMF_VARS.fd` 组成双 pflash（见 [`run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh)）。

**LoongArch UEFI**：上游 `r-efi` 支持 `loongarch64-unknown-uefi` 时，可在本工作区对 `nt10-boot-uefi` 增加对应 target 与链接脚本（与 x86_64 流程对称）。

## 2. 测试策略（母版 §26 — 目标内核）

- **单元测试**：规划置于 `tests/` 或各 crate 的 `#[cfg(test)]`；内核 `no_std` 部分需在 host 上用 mock/`std` 或独立 host crate 测算法。
- **集成测试**：QEMU 下冒烟（规划）。
- **一致性测试**：对照 NT10 文档 / ReactOS 等（规划）。
- **CI**：[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 运行 `cargo check --workspace`、`cargo kcheck` 与 UEFI 引导构建。

**本仓库现状**：以 **`cargo check` 通过** 为基线；纯逻辑可在 host 上跑单元测试（如 `cargo test -p nt10-kernel` 测 `mm::vad`）。**QEMU / 串口**里程碑（COM1 上的 `nt10-kernel: …` 日志）见 [scripts/run-qemu-x86_64.sh](../../scripts/run-qemu-x86_64.sh) 与根目录 [Makefile](../../Makefile) 的 `run-debug`。

## 3. 编码规范（母版 §27 摘要，Rust）

### 3.1 命名

| 类别 | 风格 | 示例 |
|------|------|------|
| 类型/结构体 | PascalCase | `EProcess`, `ObjectHeader` |
| 函数/模块 | snake_case | `alloc_phys_frame` |
| 常量 | SCREAMING_SNAKE | `PAGE_SIZE` |
| NT 导出 API | PascalCase + Nt 前缀 | `NtCreateFile` |
| 内部实现细节 | snake_case | `flush_tlb_internal` |

### 3.2 Rust 使用要点

- **布局敏感** 且与 NT 互操作的类型使用 `#[repr(C)]`（或明确对齐的 `repr`）。
- **导出 NT API** 的返回值与内核约定统一映射到 `NTSTATUS`（或 `i32` / newtype，团队选定后保持一致）。
- **IRQL ≥ DISPATCH_LEVEL**：不分配可分页内存、不访问可能缺页的地址；在文档与 `unsafe` 块注释中标明 IRQL 前提。
- 与用户指针交互前必须经过探测/校验路径；禁止未验证的 `unsafe` 解引用。
- HAL 等多实现点可用 **trait 对象**、**泛型参数** 或 **条件编译 `cfg(target_arch)`** 替代母版中的 comptime 多态描述。

### 3.3 安全编码

- `unsafe` 需注释 **不变量** 与 **调用方责任**。
- ISR 保存完整寄存器上下文；DPC 回调避免可分页路径（仅用 NonPagedPool 等）。

## 4. 相关文档

- [PROCESS_NT10.md](PROCESS_NT10.md)
- [Architecture.md](Architecture.md)
