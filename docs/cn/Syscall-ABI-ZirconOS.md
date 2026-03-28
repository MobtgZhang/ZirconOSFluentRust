# ZirconOS 系统调用 ABI（x86_64）

**英文**：[../en/Syscall-ABI-ZirconOS.md](../en/Syscall-ABI-ZirconOS.md)

本文描述 **ZirconOS NT10** 在 x86_64 上如何向用户态暴露系统服务，**仅适用于本仓库**，不是微软官方文档。

## 目标

- 在 [`nt10-kernel`](../../crates/nt10-kernel/src/lib.rs) 与未来用户态桩（如 [`libs/ntdll`](../../crates/nt10-kernel/src/libs/ntdll.rs)）之间约定**手写、可演进的 ABI**。
- 在可行范围内与 Windows NT 10 一代行为对齐，依据**公开事实**（寄存器约定、`syscall` 指令语义等），**不**粘贴版权受限的参考正文。

## 机制概要

- 用户态执行 `syscall`，由 `IA32_LSTAR` 等 MSR 与 GDT 用户段共同决定入口与特权级；指令级细节请以 **Intel SDM / AMD APM** 等公开手册为准。
- 内核维护分发表（[`arch/x86_64/syscall.rs`](../../crates/nt10-kernel/src/arch/x86_64/syscall.rs)），下标为 **ZirconOS 自编号**；用户态桩的**编号单一来源**为 [`libs/ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs) 中的 `numbers` 模块与 `SYSCALL_NUMBERING_REVISION`（变更编号时须递增修订值）。
- 当前引导阶段仅在安装含用户环描述符的 GDT 之后置位 **`EFER.SCE`**；`STAR` / `LSTAR` / `FMASK` 的完整编程在用户态路径启用前完成。

## 策略

- **禁止**从 `references/win32` 或 MSDN **大段摘录**进本文；需要时请链到公开网页。
- 若变更调用约定或表布局，须在变更说明与（若存在）持久化契约中**显式记录版本**。
