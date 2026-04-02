# Ring-3 用户态 bring-up 路线（ZirconOSFluent）

**English**: [../en/Ring3-bringup.md](../en/Ring3-bringup.md)

**说明**：与 [Boot-paths.md](Boot-paths.md) 配合阅读；当前仅 `-kernel` + 内置 CR3 路径运行 syscall 冒烟。

## 目标顺序

1. **链接与加载**：产出固定布局的静态 PIE 或裸 ELF，由 [`loader`](../../crates/nt10-kernel/src/loader/) 从 VFS 映射到用户 VA（扩展 [`USER_LARGE_ARENA_HINT`](../../crates/nt10-kernel/src/mm/user_va.rs)）。
2. **ntdll 用户态**：与 [`libs/ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs) 中 `numbers` 表一致的薄封装，经 `syscall` 进入内核分发表。
3. **极简 Shell**：替换纯内核 [`shell_bringup`](../../crates/nt10-kernel/src/subsystems/win32/shell_bringup.rs)，提供读行 + 内建命令。

## 合规

实现仅依据公开 ABI/PE 规范与本仓库 [Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md)，不复制 Windows 用户态源码。
