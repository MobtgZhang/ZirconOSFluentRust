# 引导路径与用户态冒烟（ZirconOSFluent / NT10）

**English**: [../en/Boot-paths.md](../en/Boot-paths.md)

## 两条主路径

| 路径 | 页表 / CR3 | 硬件定时器与 `sti` | Ring-3 冒烟 |
|------|------------|-------------------|-------------|
| **QEMU `-kernel`**（扁平 `nt10-kernel-bin`） | 内核内置 PML4（[`paging::init_low_identity`](../../crates/nt10-kernel/src/arch/x86_64/paging.rs)） | 启用 PIC/LAPIC bring-up，可调 `sti` | **会执行** [`user_enter`](../../crates/nt10-kernel/src/arch/x86_64/user_enter.rs) 路径 |
| **UEFI → ZBM10 → NT10KRNL.BIN** | 沿用固件已启用分页；**不**安装内置 CR3 | **故意跳过** 重编程 PIT/LAPIC + `sti`，避免与 OVMF 虚拟线 IRQ 冲突 | **跳过** ring-3 冒烟（见 [`kmain`](../../crates/nt10-kernel/src/kmain.rs) 分支） |

## 设计原因（摘要）

UEFI 路径下，固件已配置 IOAPIC/LAPIC 与虚拟线模式；内核再开 PIC 或 `sti` 易导致异常向量风暴或三重故障，表现为 QEMU 下「重启循环」。因此在 handoff 有效且**非**内置页表时，仅排队示例 KAPC，不启动周期时钟 ISR。

## 统一策略（演进方向）

- 要么在 UEFI 路径实现 **与固件协调的 LAPIC one-shot / TSC Deadline** 且明确 EOI 契约，再恢复用户态冒烟；
- 要么文档化「仅 `-kernel` 验证 syscall 门」，本页即为当前权威说明。

内核 ESP 内核文件路径：`EFI/ZirconOSFluent/NT10KRNL.BIN`（见 [`pack-esp.sh`](../../scripts/pack-esp.sh)）。
