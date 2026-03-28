# 参与贡献

## 商标与关联性

ZirconOS 与 Microsoft Corporation **无关联**。「Windows」「Windows 10」「Win32」等名称与商标归 Microsoft 所有。本仓库仅用于描述技术对标与兼容目标；对外材料请沿用 [README.md](README.md) 中的免责声明。

## 参考微软公开文档（含 `references/win32`）

本地镜像的 Win32 / 桌面 API 文档用于理解**已公开文档化的行为、参数、错误码与 ABI**。实现要求：

- **独立实现（clean-room 取向）**：根据规范与测试编写自有代码，**不要**从微软文档、SDK、WDK 或示例中**整段复制**源码。
- **禁止抄袭**：避免与 Windows 或 WDK 源码**逐行对照**抄写；可对照 **UEFI 规范、PE/COFF、ACPI** 等公开标准与自研测试验证行为。
- **许可证**：引入第三方 crate 时保留其版权与许可信息；勿提交受限制许可的二进制或源码。

## 对 `references/r-efi` 的使用

[r-efi](references/r-efi)（或 crates.io 同名 crate）提供 UEFI **协议常量与类型定义**，不实现协议逻辑。引导代码中的 **Boot Services 调用序列与错误处理须自行编写**。

为便于在本工作区对自有 crate 使用 `cargo clippy … -D warnings`，已在 vendored `references/r-efi/src/lib.rs` 增加 `#![allow(warnings)]`，避免对规范转写代码做大规模 clippy 适配；**若上游同步更新 r-efi，请视需要保留或删除该属性**。

## 代码与测试

- 布局敏感或与固件/ABI 互操作的类型使用 `#[repr(C)]`（或明确对齐），并在 `unsafe` 处写明前提与不变量。
- 为公开行为补充测试或 QEMU 冒烟步骤（见 [scripts/](scripts/) 与 [docs/cn/Build-Test-Coding.md](docs/cn/Build-Test-Coding.md)）。

## 调试输出约定（早期内核）

- **首选**：x86_64 下 **COM1**（I/O 端口 `0x3F8`，QEMU `-serial stdio`）输出 UTF-8 子集（ASCII）。
- **HAL**：调试写路径应对齐 `Hal::debug_write` 语义；在 `IRQL` 相关路径注释中标明是否可调用。
- **UEFI 阶段**：使用固件 `ConOut`（UTF-16）；进入内核后切换到串口，避免混用同一缓冲策略。
