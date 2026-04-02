# 以内核管理为优先时的暂缓范围

**性质**：策略说明，非代码交付物。

在**优先做内核管理**的阶段，下列内容**默认不做**，除非有阻塞引导或 CI 的维护级修复：

- Fluent 桌面 / DWM 类能力扩展（Roadmap Phase 14 深度）。
- WinRT/UWP 桩扩展（Phase 15 深度）。
- 超出既有 bring-up 所需的新 Win32 外壳整合。
- 真实 `csrss.exe` / SMSS 用户进程与完整用户态 ALPC 约定。

工作应优先投向 MM、KE（调度、IRQL/DPC）、OB、PS、SMP/TLB、SE 接入路径。产品优先级变化时再调整。

**English**: [Kernel-Management-Focus.md](../en/Kernel-Management-Focus.md)
