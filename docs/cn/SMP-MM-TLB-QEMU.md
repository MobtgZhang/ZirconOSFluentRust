# SMP、TLB shootdown 与 QEMU（与 MM 联动）

**English**: [SMP-MM-TLB-QEMU.md](../en/SMP-MM-TLB-QEMU.md)

Clean-room 说明：**内存管理** 与 **TLB 失效** 在多逻辑 CPU 在线时如何衔接。

## BSP 与 AP 同 IDT

在调用 [`smp_set_online_cpu_count`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) 且 `n > 1` 之前，每个应用处理器须加载与 BSP **相同的 IDT**，其中包括 [`TLB_FLUSH_IPI_VECTOR`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs)（`0xFD`）的门。否则 AP 上远程 `invlpg` IPI 未定义，PTE 修改后可能残留陈旧 TLB 项。

## 与 MM 的约定

任何成功改变用户可见映射的 [`map_4k`](../../crates/nt10-kernel/src/mm/pt.rs) / [`unmap_4k`](../../crates/nt10-kernel/src/mm/pt.rs) 之后，应调用 [`flush_after_pte_change`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs)（或等价的范围 shootdown）。在 SMP 下可能升级为 [`shootdown_range_all_cpus`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs)。

## QEMU `-smp`

- 示例：`qemu-system-x86_64 -smp 2 ...`（再加 [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh) 常用的 OVMF/ESP 参数）。
- **Bring-up 说明：** 默认引导路径不假定 AP 已完整拉起；在 AP 启动与 IDT 交接完成前，多核仅作**可选实验**。
- SMP 下串口顺序可能变化；[`ZBM10_CAPTURE_SERIAL`](../../scripts/run-qemu-x86_64.sh) 与 [`scripts/verify-mm-serial-keywords.sh`](../../scripts/verify-mm-serial-keywords.sh) 仅作可选 MM 标记检查，除非已启用 AP 代码，否则不能单凭此证明 IPI 已送达。

## 宿主机 `cargo test` 与 `invlpg`

[`shootdown_bringup_tests`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) 标为 `#[ignore]`，因 `invlpg` 与 LAPIC IPI 为 ring 0 路径。将来在内核 harness 或 QEMU 支持的测试运行器中再执行；勿为默认宿主机 `cargo test` 去掉 `ignore`。

另见：[MM-Goals-and-Invariants.md](MM-Goals-and-Invariants.md)、[Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3 / SMP 相关描述。
