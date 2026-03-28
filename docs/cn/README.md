# ZirconOS NT10 中文文档

**英文对照**：[../en/README.md](../en/README.md)

**代码实现**：Rust + Cargo；内核骨架见 [crates/nt10-kernel/src/](../../crates/nt10-kernel/src/)，与架构母版 §4 目录一一对应。

## 阅读顺序建议

1. [Architecture.md](Architecture.md) — 总览与「当前实现 vs 目标架构」
2. [Roadmap-and-TODO.md](Roadmap-and-TODO.md) — Phase 0–15 与仓库状态
3. [Kernel-Executive-and-HAL.md](Kernel-Executive-and-HAL.md) — 引导、HAL、执行体核心
4. [Memory-and-Objects.md](Memory-and-Objects.md) — 内存管理器、对象管理器
5. [Processes-Security-IO.md](Processes-Security-IO.md) — 进程/安全/I/O/文件系统/ALPC
6. [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md) — 加载器、Win32k/WDDM、`desktop/fluent` 模块
7. [Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md) — Hyper-V 感知、CFG/CET、WinRT、多架构
8. [Build-Test-Coding.md](Build-Test-Coding.md) — 构建、测试、编码规范（Cargo / `rustc`）
9. [PROCESS_NT10.md](PROCESS_NT10.md) — 贡献流程与文档同步约定
10. [References-Policy.md](References-Policy.md) — 如何合规使用 `references/win32` 与 `references/r-efi`
11. [Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md) — x86_64 系统调用 ABI（本项目自有）

**互链约定**：每篇正文顶部可放一行 `English: ../en/<同名文件>`。

**母版**：[../../ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md)
