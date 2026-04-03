# 内核映像重定位 — 分期里程碑（clean-room）

**English**: [Kernel-Relocate-Phases.md](../en/Kernel-Relocate-Phases.md)

**完整内核虚拟重定位**（将运行中映像迁到新 VA 区间）在 bring-up 中**未**实现。本文将后续工作拆成可**独立验收**的阶段；依据限于公开文档与本仓库自测，不以 Windows 内部布局为权威。

## 阶段 R1 — 解析并界定已加载映像

- 从**本仓库**加载器数据结构解析 PE/ELF（或自定义）**已加载区间**与段权限。
- **验收：** 解析代码可置于 feature 后；`cargo check -p nt10-kernel --target x86_64-unknown-none`；**不改变**运行中 CR3/`rip`。

## 阶段 R2 — 重定位表校验（只读）

- 读取重定位记录；对照 R1 做**一致性校验**（区间、对齐、与已校验内存图中的固件保留不冲突）。
- **验收：** 宿主机单元测试使用**合成**表；仍不改变运行 PC。

## 阶段 R3 — 引导页表调整

- 构建或修改早期映射，使**目标** VA 与当前映射等价（临时双映射或分步切换），**尚未**跳转。
- **验收：** QEMU 引导到约定检查点与日志；可选用串口关键字脚本。

## 阶段 R4 — 与 MM / PFN 集成

- 过渡期间 PFN/buddy/pool **排除**旧、新映像占用区；与 [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) 不变量一致。
- **验收：** 除非有意的短暂窗口且文档说明，否则避免可写双重映射。

## 阶段 R5 — 实际 `rip` / 栈迁移（最后）

- 仅在 R1–R4 通过后执行受控跳转到新 VA。
- **验收：** 单路径 QEMU；回滚策略写入文档。

另见：[Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3、[MM-Pool-and-PFN-Bringup.md](MM-Pool-and-PFN-Bringup.md)。
