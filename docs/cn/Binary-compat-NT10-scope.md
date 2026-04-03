# NT 10.0 二进制兼容 — 范围说明（ZirconOSFluent）

**English**：[../en/Binary-compat-NT10-scope.md](../en/Binary-compat-NT10-scope.md)

本文定义本仓库中「兼容」的分阶段含义，并与 [Clean-Room-Implementation.md](../en/Clean-Room-Implementation.md) 一致。

## 阶段

| 阶段 | 目标 | 零售 Windows 二进制 |
|------|------|---------------------|
| **A** | 仅支持链接**本仓库**桩库（[`ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs)）的自建 PE/DLL | 不针对 |
| **B** | 在 A 基础上，对少量 `Nt*` 使用 **Windows 10 22H2 x64** 公开 syscall 号，并在内核侧可选 **Nt10X64** 参数展开 | 仅在明确最小导入测试时尝试 |
| **C** | 更广 syscall / 加载器 / 子系统 | 长期；仍 clean-room |

当前落地的是 **B 的基础设施**：在 Zircon 自有 `numbers::*` 与 NT 索引上**重复注册**（见 [`nt_syscall_indices.rs`](../../crates/nt10-kernel/src/arch/x86_64/nt_syscall_indices.rs)），并由 [`syscall_abi.rs`](../../crates/nt10-kernel/src/arch/x86_64/syscall_abi.rs) 选择 **LegacyZircon**（默认）或 **Nt10X64**。

## 首批 syscall 集合（B）

实现多为桩，以路线图为准：`NtTerminateProcess`、`NtReadFile`、`NtWriteFile`、`NtClose`、`NtAllocateVirtualMemory`、`NtFreeVirtualMemory`、`NtCreateFile`、`NtProtectVirtualMemory`、`NtQuerySystemTime` — 编号取自公开数据集 **j00ru** `windows-syscalls` 中 Windows 10 **22H2** 列。

## 合规

- **禁止**以 Windows 零售二进制作为主要实现依据。
- **允许**公开规范（Intel SDM、`syscall` 行为、PE/COFF）及**公开 syscall 表**（事实数据，非代码）。
- **应**用自建测试程序在 QEMU 或许可的参考环境上验证。

## 参见

- [Syscall-ABI-ZirconOS.md](../en/Syscall-ABI-ZirconOS.md)
- [Public-References.md](../en/Public-References.md)
