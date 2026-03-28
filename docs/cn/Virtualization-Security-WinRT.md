# Hyper-V 感知、现代安全、WinRT/UWP 与多架构

**English**: [../en/Virtualization-Security-WinRT.md](../en/Virtualization-Security-WinRT.md)

本文对应母版 §20–§23。**实现状态**：目标设计；模块桩见 [hyperv/](../../crates/nt10-kernel/src/hyperv/)、[vbs/](../../crates/nt10-kernel/src/vbs/)、[arch/x86_64/cet.rs](../../crates/nt10-kernel/src/arch/x86_64/cet.rs) 等。

## 1. Hyper-V 感知层（§20）

### 1.1 检测

通过 **CPUID** 叶 `0x40000000`–`0x40000006` 等识别 **Microsoft Hypervisor**（如厂商字符串 `"Microsoft Hv"`）。

### 1.2 感知优化（Enlightenments）

示例（母版）：`HVCALL_FLUSH_VIRTUAL_ADDRESS_LIST` 批量 TLB 刷新、`HVCALL_NOTIFY_LONG_SPIN_WAIT`、参考时间计数页、**SynIC**、**VMBus** 等。非 Hyper-V 环境下模块应退化为空操作。

### 1.3 目标路径（本仓库）

[hyperv/detect.rs](../../crates/nt10-kernel/src/hyperv/detect.rs)、`hypercall.rs`、`synic.rs`、`enlighten.rs`、`vmbus.rs`。

## 2. 现代安全特性（§21）

### 2.1 CFG（控制流防护）

每进程 **CFG 位图**（如 16 字节粒度）；加载器填充；间接调用前校验目标。

### 2.2 CET（控制流强制技术）

影子栈：[arch/x86_64/cet.rs](../../crates/nt10-kernel/src/arch/x86_64/cet.rs) 规划配置 CR4.CET 与相关 MSR；`RET` 与影子栈不一致触发 `#CP`。

### 2.3 DEP

页表 **NX** 位；`VirtualProtect` 等路径在 [mm/vm.rs](../../crates/nt10-kernel/src/mm/vm.rs) 中更新。

### 2.4 其他

MIC、HVCI 接口见 [Processes-Security-IO.md](Processes-Security-IO.md) 与 [se/hvci.rs](../../crates/nt10-kernel/src/se/hvci.rs) 规划。

## 3. VBS（母版 §21 周边与 vbs/ 目录）

[vbs/vsm.rs](../../crates/nt10-kernel/src/vbs/vsm.rs)、`skci.rs`、`credguard.rs` 等：虚拟安全模式、SKCI、Credential Guard 桩。

## 4. WinRT / UWP 运行时支持（§22）

- **AppModel**：包身份、AppContainer、**RoActivateInstance** → ALPC → 进程外 COM 服务器。
- **IAsyncOperation** 等异步模型。
- 开发阶段以 **桩实现** 为主，优先 Win32 子系统完整性。

## 5. 多架构支持（§23）

| 架构 | 优先级 | 系统调用 | 页表 | 虚拟化感知 |
|------|--------|----------|------|------------|
| x86_64 | 主力 | SYSCALL/SYSRET | 4（LA57 可选 5） | VMX |
| aarch64 | 次要 | SVC #0 | 4 级 | HVC |
| riscv64 | 实验 | ECALL | Sv48 | 无 |
| loongarch64 | 实验 | SYSCALL | 4 | 无 |
| mips64el | 实验 | SYSCALL | 3/4 | 无 |

**WOW64**：x64 上跑 x86 PE32；ARM64 上跑 ARM32 与 PE32（母版）。

**本仓库**：[arch/](../../crates/nt10-kernel/src/arch/) 下含 `x86_64`、`aarch64` 等子目录桩。

## 6. 相关文档

- [Kernel-Executive-and-HAL.md](Kernel-Executive-and-HAL.md)
- [Build-Test-Coding.md](Build-Test-Coding.md)
