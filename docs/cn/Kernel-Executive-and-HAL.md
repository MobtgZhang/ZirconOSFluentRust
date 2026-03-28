# 引导子系统、HAL 与内核执行体核心（KE）

**English**: [../en/Kernel-Executive-and-HAL.md](../en/Kernel-Executive-and-HAL.md)

本文对应母版 §5–§7：Boot、HAL、调度 / IRQL / DPC / APC。**实现状态**：目标设计；引导代码计划在 **[crates/nt10-boot-uefi](../../crates/nt10-boot-uefi/)** 展开；HAL/KE 模块桩在 **[crates/nt10-kernel/src/hal/](../../crates/nt10-kernel/src/hal/)**、**[ke/](../../crates/nt10-kernel/src/ke/)**（见 [Architecture.md](Architecture.md)）。

## 1. 引导子系统（ZBM10）

**ZBM10**（ZirconOS Boot Manager 10）规划为 **仅 UEFI**，不含传统 BIOS/MBR 路径。

### 1.1 引导流程

```
UEFI 固件
  → ZBM10（计划在 nt10-boot-uefi crate 中实现：main / BCD / 菜单 / Secure Boot）
  → Secure Boot 验证
  → BCD 解析
  → 启动菜单（Fluent 文本 UI）
  → 加载内核 EFI 映像
  → GetMemoryMap / GOP
  → ExitBootServices
  → 跳转内核入口（未来 nt10-kernel 二进制入口或静态库 + 链接脚本）
```

### 1.2 ZirconBootInfo（规划）

向内核传递引导参数的结构示例（母版），Rust 侧可用 `#[repr(C)]`：

```rust
#[repr(C)]
pub struct ZirconBootInfo {
    pub magic: u64, // 0x5A49524E4F4E3130 "ZIRNON10"
    pub mem_map: *mut MemoryDescriptor,
    pub mem_map_count: usize,
    pub framebuffer: FramebufferInfo,
    pub acpi_rsdp: u64,
    pub tpm_base: u64,
    pub cmdline: [u8; 256],
    pub kernel_base: u64,
    pub initrd_base: u64,
    pub initrd_size: usize,
}
```

占位常量见 [crates/nt10-boot-uefi/src/lib.rs](../../crates/nt10-boot-uefi/src/lib.rs) 中的 `ZIRNON10_MAGIC`。

### 1.3 Secure Boot

在加载内核前校验 PE 签名（如通过 UEFI `EFI_SECURITY2_ARCHITECTURAL_PROTOCOL`），为后续 HVCI 等链式信任打基础。

## 2. 硬件抽象层（HAL）

HAL 封装平台相关操作；执行体仅通过 HAL 访问硬件。Rust 中可用 **trait**、**泛型** 或 **`cfg(target_arch)`** 表达母版中的多态接口，例如：

```rust
pub trait Hal {
    fn mask_irq(&self, vector: u8);
    fn unmask_irq(&self, vector: u8);
    fn send_eoi(&self);
    fn current_time_ns(&self) -> u64;
    fn set_timer(&self, ns: u64, cb: TimerCallback);
    fn flush_tlb(&self, addr: u64);
    fn flush_tlb_all(&self);
    fn debug_write(&self, s: &[u8]);
}
```

**x86_64**：优先 APIC + HPET + invariant TSC；可回退 PIC + PIT。Hyper-V 宾客下可使用感知时钟源以降低 TSC 读取成本（详见 [Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md)）。

**模块路径**：[crates/nt10-kernel/src/hal/](../../crates/nt10-kernel/src/hal/)。

## 3. 内核执行体核心（KE）

### 3.1 调度器

- 多级反馈队列，**32 个优先级**（0–31），参考 NT `KPRIORITY`。
- 0–15：动态优先级（典型用户线程）；16–31：实时类。
- 支持优先级继承、CPU 亲和性（`KAFFINITY`）、NUMA 感知。

### 3.2 IRQL 模型（摘要）

| IRQL | 名称 | 用途 |
|------|------|------|
| 0 | PASSIVE_LEVEL | 普通内核/用户代码 |
| 1 | APC_LEVEL | APC |
| 2 | DISPATCH_LEVEL | DPC、调度 |
| 3–26 | DIRQL | 设备中断 |
| 27 | PROFILE_LEVEL | 性能剖析 |
| 28 | CLOCK_LEVEL | 时钟 |
| 29 | IPI_LEVEL | IPI |
| 30 | POWER_LEVEL | 电源 |
| 31 | HIGH_LEVEL | NMI 等 |

### 3.3 DPC / APC

- **DPC**：在 `DISPATCH_LEVEL` 运行，驱动下半部；**每 CPU 独立队列**。
- **KernelAPC**：`APC_LEVEL`，如线程状态、内存完成通知。
- **UserAPC**：用户态异步执行（如 `NtQueueApcThread`）。

### 3.4 模块划分（本仓库路径）

- [ke/sched.rs](../../crates/nt10-kernel/src/ke/sched.rs)、`timer.rs`、`dpc.rs`、`apc.rs`、`irq.rs`、`spinlock.rs`、`mutex.rs`、`event.rs`、`semaphore.rs`、`waitobj.rs`、`trap.rs` 等。

## 4. 相关文档

- [Memory-and-Objects.md](Memory-and-Objects.md)
- [Build-Test-Coding.md](Build-Test-Coding.md)
