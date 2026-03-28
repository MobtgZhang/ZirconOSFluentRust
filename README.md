# ZirconOSFluentRust

以 **Windows NT 10.0.19045** 为设计参照的 **ZirconOS NT10** 内核方向实验仓库：**Rust** + **Cargo** 工作区。

## 快速开始

```bash
rustup target add x86_64-unknown-none   # rust-toolchain.toml 已声明该 target
cargo check --workspace
cargo check -p nt10-kernel --target x86_64-unknown-none
# 或（已配置别名时）
cargo kcheck
```

- **内核库（`#![no_std]`）**：[crates/nt10-kernel/](crates/nt10-kernel/)
- **UEFI 引导占位**：[crates/nt10-boot-uefi/](crates/nt10-boot-uefi/)
- **可引导内核 ELF / 扁平镜像**：链接脚本 [link/x86_64-uefi-load.ld](link/x86_64-uefi-load.ld) 由 [crates/nt10-kernel-bin/build.rs](crates/nt10-kernel-bin/build.rs) 在构建 `nt10-kernel-bin` 时传入（物理入口 `0x100000`，供 ZBM10 与 `objcopy -O binary`）。[link/x86_64.ld](link/x86_64.ld) 仅作高半核实验参考，不参与当前可执行链接。

## 文档

- 索引：[docs/README.md](docs/README.md)
- 中文：[docs/cn/README.md](docs/cn/README.md)
- English: [docs/en/README.md](docs/en/README.md)

## 运行与仿真

- **QEMU（x86_64 + OVMF）**：安装 QEMU 与 OVMF 后执行 [`scripts/run-qemu-x86_64.sh`](scripts/run-qemu-x86_64.sh)（或 `cargo run -p xtask -- qemu`）。临时 FAT ESP（默认）包含：
  - `EFI/BOOT/BOOTX64.EFI` ← ZBM10（`nt10-boot-uefi`）
  - `EFI/ZirconOS/NT10KRNL.BIN` ← 扁平内核（`objcopy` 自 `nt10-kernel-bin`）
  - 根目录 `startup.nsh`：在 OVMF 对 QEMU `fat:` 盘默认启动项返回 `Unsupported` 时，可从 Shell 倒计时后自动执行 `BOOTX64.EFI`  
  - **ZBM10 操作系统选择菜单**：图形控制台会显示 `ZBM10 - OS selection`（无 GOP 时仅有文本）；**约 10 秒**无按键则自动启动第一项（ZirconOS NT10）；`↑/↓` 或 `1–4` 选择，`Enter` 确认，`B` 立即启动默认项；可选「重启 / 关机」。  
  `PROFILE=release` 或 `cargo run -p xtask -- qemu --release` 使用 release 构建。未设置 `OVMF_CODE` 时会在常见目录先找 `OVMF_CODE.fd`（分体），再找 `OVMF.fd`（合并）；合并镜像用 QEMU `-bios`，分体且文件名含 `code` 时自动配对同目录 `OVMF_VARS.fd` 为双 pflash。详见脚本 `usage`。构建：`cargo build -p nt10-boot-uefi --target x86_64-unknown-uefi`（产物多为 `zbm10.efi`，与 `pack-esp.sh` 中无扩展名 `zbm10` 二选一均可）。
- **内核裸机二进制**（低地址入口，便于 `-kernel` 冒烟）：`cargo build -p nt10-kernel-bin --target x86_64-unknown-none`，产物在 `target/x86_64-unknown-none/debug/nt10-kernel-bin`；可配合 [`scripts/run-qemu-kernel.sh`](scripts/run-qemu-kernel.sh)。
- **品牌资源**：矢量标 [`assets/zirconos-mark.svg`](assets/zirconos-mark.svg)；栅格预览由 [`scripts/generate-brand-assets.py`](scripts/generate-brand-assets.py) 生成（见 [`assets/manifest.json`](assets/manifest.json)）。
- **桌面资源包**：壁纸与多尺寸图标等在 [`resources/`](resources/)，清单 [`resources/manifest.json`](resources/manifest.json)；图标尺寸由 [`scripts/generate-resource-icons.py`](scripts/generate-resource-icons.py)（需 `pip install pillow`）从 `resources/icons/_sources/` 生成。

## 贡献与合规

参与开发前请阅读 [CONTRIBUTING.md](CONTRIBUTING.md)（**clean-room** 参考策略、商标说明、调试输出约定）。

## 免责声明

ZirconOS 与 Microsoft 无关联。「Windows」「Windows 10」等商标归 Microsoft Corporation 所有；本仓库仅用于描述兼容与对标目标。
