# .NET 与用户态策略（ZirconOSFluent / NT10 对标）

**English**: [../en/DotNet-UserMode.md](../en/DotNet-UserMode.md)

## 1. 生态事实

在 Windows 10（含 10.0.19045）生态中，大量**桌面应用、工具与管理组件**依赖 **.NET Framework** 或 **.NET（Core）**：业务线程序、部分系统管理体验、以及依赖托管运行时的第三方软件。将 ZirconOSFluent 作为「兼容 NT10 行为意图」的平台长期推进时，需要正视这一用户态依赖，而不是假设全部为纯原生 PE。

## 2. 与本项目的关系

- **内核与用户态分离**：本仓库内核（`nt10-kernel`）为 `no_std` Rust；**不在内核中实现 CLR、JIT 或 BCL**。
- **合规**：任何未来托管运行时与类库须遵循 [CONTRIBUTING.md](../../CONTRIBUTING.md) 的 clean-room 要求——独立实现或兼容公开规范与测试，**不复制** Windows / .NET 专有源码。
- **完整兼容路径（远期）**：在 ring-3 提供或移植兼容的 **CLR 宿主** 与 **BCL 子集**，通过既有 **NT 风格系统调用**（虚拟内存、节对象、线程等）与内核交互；具体范围由后续路线图定义。

## 3. 为何不提供 PowerShell

**Windows PowerShell** 的引擎与 cmdlet 体系建立在 **.NET** 之上。在本仓库**不实现 .NET 运行时**的前提下，在内核或 bring-up 层实现「真 PowerShell」不现实；因此 **本仓库不实现 PowerShell**，已移除内核侧 PowerShell 占位模块。

命令行 bring-up 以 **CMD 风格桩**为主；需要**类 PowerShell 的脚本体验**时，应规划为**未来由 .NET 用户态进程承载**的宿主（见上文），而非在内核复制 PowerShell 语义。

## 4. 与内存管理器的衔接

托管进程通过 `VirtualAlloc` / `NtAllocateVirtualMemory` 等路径产生大量 **保留（reserve）与提交（commit）**；JIT 生成代码需要**可执行且受 NX/DEP 约束**的映射。这与 [Memory-and-Objects.md](Memory-and-Objects.md) 中的 VAD、按需分页与页表保护目标一致；内核侧只需提供**符合公开文档语义的 MM 能力**，无需知晓「这是 .NET」。

## 5. 相关文档

- [Memory-and-Objects.md](Memory-and-Objects.md) — MM 布局与 VAD/节对象
- [Ring3-bringup.md](Ring3-bringup.md) — 用户态演进
- [CONTRIBUTING.md](../../CONTRIBUTING.md) — 参考策略与版权
- [Build-Test-Coding.md §1.4](Build-Test-Coding.md) — **字体与 UI 资源许可**（OFL 栈、`third_party/fonts/licenses` 索引）；托管 UI 若带图标/字体，须同样避免捆绑受限微软素材
