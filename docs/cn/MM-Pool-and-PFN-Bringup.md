# MM：PFN、buddy 与池化 bring-up（ZirconOSFluent）

**English**: [../en/MM-Pool-and-PFN-Bringup.md](../en/MM-Pool-and-PFN-Bringup.md)

本文记录早期内核内存管理的**设计意图与不变量**，描述**本仓库**行为，不复制任何厂商内部实现细节（clean-room）。

## 物理帧（PFN）

- **依据**：UEFI 内存图经 [`ZirconBootInfo`](../../crates/nt10-boot-protocol/src/lib.rs) 传入，由 [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) 校验、[`early_map`](../../crates/nt10-kernel/src/mm/early_map.rs) 归纳。
- **Bring-up 分配器**：[`phys`](../../crates/nt10-kernel/src/mm/phys.rs) / [`pfn`](../../crates/nt10-kernel/src/mm/pfn.rs) 的 bump 式分配。
- **不变量**：交给池或用户映射的帧不得与已加载内核映像或固件保留区重叠。

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

## 后续（P0 方向）

- 按 tag 的统计与可选调试构建泄漏审计。
- 完整 **NUMA** 与本 bring-up 文档范围无关。
- **内核整体重定位**单独里程碑，见 [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3。

另见：[Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 1。
