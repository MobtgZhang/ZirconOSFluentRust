# 内存管理器 — 目标与不变量（ZirconOSFluent）

**English**：[../en/MM-Goals-and-Invariants.md](../en/MM-Goals-and-Invariants.md)

本文档为**项目自有**说明，不描述 Windows 内部实现。实现依据为公开 CPU 手册（Intel SDM / AMD APM）、相关 UEFI 内存描述、PE/COFF 公开布局与本仓库测试。

## 目标

- **每地址空间**：x86_64 四级页表，根为 `CR3`（bring-up 阶段可多个进程共用同一用户 `CR3`，拆分后每进程独立）。
- **VAD 与 PTE 一致**：已提交用户映射须有对应 [`VadEntry`](../../crates/nt10-kernel/src/mm/vad.rs)；按需零页与文件后备路径仅在 VAD 校验通过后装 PTE。
- **PFN 生命周期**：帧来自 buddy 池（[`buddy.rs`](../../crates/nt10-kernel/src/mm/buddy.rs)、[`phys.rs`](../../crates/nt10-kernel/src/mm/phys.rs)）；[`pfn.rs`](../../crates/nt10-kernel/src/mm/pfn.rs) 引用计数支撑共享/COW。
- **TLB 一致**：修改 PTE 后调用 [`flush_after_pte_change`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) 或 `shootdown_range_all_cpus`（多核或非当前 `CR3` 可能缓存该 VA 时），见 [`pt.rs`](../../crates/nt10-kernel/src/mm/pt.rs) 注释。

## 不变量（bring-up）

1. **Buddy**：`alloc_order` / `free_order` 成对；仅释放由 `pfn_bringup_alloc` 等返回的帧。
2. **引导 handoff**：[`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) 可用区排除已加载内核映像，这些 PFN 不进入空闲池。
3. **用户规范 VA**：用户 `#PF` 与 syscall 指针检查使用 `< 0x8000_0000_0000`，封装见 [`user_va`](../../crates/nt10-kernel/src/mm/user_va.rs)。
4. **Section 析构**：先拆除引用 [`SectionObject`](../../crates/nt10-kernel/src/mm/section.rs) 的 VAD，再 `release` 驱动 PFN 释放（见 `section.rs` 模块说明）。
5. **匿名上限**：[`SECTION_ANONYMOUS_PAGE_CAP`](../../crates/nt10-kernel/src/mm/section.rs) 为 bring-up 上限；超出返回 [`SectionCommitError::AnonymousCapExceeded`](../../crates/nt10-kernel/src/mm/section.rs)。

## 明确延后 / 当前非目标

- **NUMA**：多节点未实现，见 [`numa.rs`](../../crates/nt10-kernel/src/mm/numa.rs)。
- **页文件**：换出到盘为 [`Unsupported`](../../crates/nt10-kernel/src/mm/pagefile.rs)，除非后续增加后端。
- **大页用户映射**：见 [`large_page.rs`](../../crates/nt10-kernel/src/mm/large_page.rs)；默认仍为 4 KiB。

## 参见

- [Clean-Room-Implementation.md](../en/Clean-Room-Implementation.md)
- [Public-References.md](Public-References.md)
- [Roadmap-and-TODO.md](Roadmap-and-TODO.md)
