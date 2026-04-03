# MM：PFN、buddy 与池化 bring-up（ZirconOSFluent）

**English**: [../en/MM-Pool-and-PFN-Bringup.md](../en/MM-Pool-and-PFN-Bringup.md)

本文记录早期内核内存管理的**设计意图与不变量**，描述**本仓库**行为，不复制任何厂商内部实现细节（clean-room）。

## 物理帧（PFN）

- **依据**：UEFI 内存图经 [`ZirconBootInfo`](../../crates/nt10-boot-protocol/src/lib.rs) 传入，由 [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) 校验、[`early_map`](../../crates/nt10-kernel/src/mm/early_map.rs) 归纳。
- **Bring-up 分配器**：[`phys`](../../crates/nt10-kernel/src/mm/phys.rs) / [`pfn`](../../crates/nt10-kernel/src/mm/pfn.rs) 的 bump 式分配。
- **不变量**：交给池或用户映射的帧不得与已加载内核映像或固件保留区重叠。

## 责任边界（PFN ↔ buddy ↔ pool）

| 层 | 职责 | 调用方预期 |
|----|------|------------|
| **PFN / phys** | [`pfn_bringup_init`](../../crates/nt10-kernel/src/mm/phys.rs) 之后持有可管理帧有序表；[`pfn_bringup_alloc`](../../crates/nt10-kernel/src/mm/phys.rs) / [`pfn_bringup_free`](../../crates/nt10-kernel/src/mm/phys.rs) 为池补充与 demand-zero #PF 等使用的 4 KiB 页入口。 | 上层不得释放非本栈取得的物理页；不得映射 `boot_mem` 已排除的帧。 |
| **Buddy** | 自同一 PFN 有序切片初始化后，对多块做可选合并；[`alloc_order`](../../crates/nt10-kernel/src/mm/buddy.rs) / [`free_order`](../../crates/nt10-kernel/src/mm/buddy.rs) 维护 PFN 元数据。 | 小对象池 class **不**走 buddy；池用单页 PFN 做 [`refill_class`](../../crates/nt10-kernel/src/mm/pool.rs)。 |
| **Pool** | 按 class 的 slab 空闲链；[`refill_class`](../../crates/nt10-kernel/src/mm/pool.rs) 取 **一页** 4 KiB PFN 再切多块。 | refill 失败（PFN 耗尽）时分配返回 `null`，**调用方**自行处理（bring-up 无自动 buddy 回退）。 |

本文为 **ZirconOSFluent** 分层说明，非任何厂商内部结构描述。

## Buddy

- [`buddy`](../../crates/nt10-kernel/src/mm/buddy.rs) 作为 PFN 之上的大块/可回收路径补充。
- **不变量**：归还 buddy 的页须与分配时 order 一致（禁止重复释放）。

## Slab 池（`pool.rs`）

- **形态**：2 的幂**总**块大小（含 8 字节头），头内存 tag 与 class 索引。
- **补充**：[`refill_class`](../../crates/nt10-kernel/src/mm/pool.rs) 尽量单页 4 KiB 切多块。
- **不变量**：
  1. `ex_free_pool_with_tag` 须使用分配返回的用户指针且 tag 一致，否则忽略释放；tag 不匹配时 x86_64 串口输出 `[ZFOS][MM]`。
  2. `POOL_BYTES` 仅粗粒度统计，非泄漏检测器。
  3. 大块连续分配优先 PFN slab / section，而非小 class。
- **遥测**：[`pool_alloc_fail_count`](../../crates/nt10-kernel/src/mm/pool.rs) 统计分配失败次数；失败时串口打印 `pool_alloc_req_bytes=` 等。
- **特性 `mm-pool-stats`**：[`pool_stats_snapshot`](../../crates/nt10-kernel/src/mm/pool.rs) 提供成功分配/释放次数与粗粒度 slab 字节。
- **特性 `mm-pool-tag-hist`**：按 `tag % 8` 分 8 槽，[`pool_tag_buckets_snapshot`](../../crates/nt10-kernel/src/mm/pool.rs)（轻量形状提示，非完整 tag 表）。
- **调试构建**（`debug_assertions`）：[`pool_debug_live_count`](../../crates/nt10-kernel/src/mm/pool.rs) 跟踪未配对释放的成功分配数；若释放会使计数跌破零则 `debug_assert!`（疑似 double-free 或账不平）。宿主机 `cargo test` 不得写 COM1 — 池在 `cfg(test)` 下已避免串口日志。

## 可选 QEMU / 串口检查

捕获客户机串口（如 [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh) 的 `ZBM10_CAPTURE_SERIAL`）后，可运行 [`scripts/verify-mm-serial-keywords.sh`](../../scripts/verify-mm-serial-keywords.sh) 检索 `[ZFOS][MM]` 行。

## 后续（P0 方向）

- 在控制 release 体积前提下再加强按 tag 的统计（不限于 `tag % 8`）。
- 完整 **NUMA** 与本 bring-up 文档范围无关。
- **内核整体重定位**见 [Kernel-Relocate-Phases.md](Kernel-Relocate-Phases.md) 与 [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3。

另见：[Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 1。
