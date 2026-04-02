# Phase 08：IPC、RPC、服务与进程线程

## 本阶段目标

将文档中的 **进程线程、服务控制管理器、RPC、经典 IPC** 与 ZirconOSFluent 的 ALPC、SMSS/服务桩及对象管理对照（Roadmap Phase 5–6、8）。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/system-services.md](../references/win32/desktop-src/system-services.md)
- [references/win32/desktop-src/ipc/interprocess-communications.md](../references/win32/desktop-src/ipc/interprocess-communications.md)
- [references/win32/desktop-src/Rpc/rpc-start-page.md](../references/win32/desktop-src/Rpc/rpc-start-page.md)
- [references/win32/desktop-src/Services/services.md](../references/win32/desktop-src/Services/services.md)
- [references/win32/desktop-src/ProcThread/processes-and-threads.md](../references/win32/desktop-src/ProcThread/processes-and-threads.md)
- [references/win32/desktop-src/Memory/memory-management.md](../references/win32/desktop-src/Memory/memory-management.md)
- [references/win32/desktop-src/Sync/synchronization.md](../references/win32/desktop-src/Sync/synchronization.md)
- [references/win32/desktop-src/Dlls/dynamic-link-libraries.md](../references/win32/desktop-src/Dlls/dynamic-link-libraries.md)
- [references/win32/desktop-src/com/component-object-model--com--portal.md](../references/win32/desktop-src/com/component-object-model--com--portal.md)
- [references/win32/desktop-src/midl/midl-start-page.md](../references/win32/desktop-src/midl/midl-start-page.md)
- [references/win32/desktop-src/Stg/structured-storage-start-page.md](../references/win32/desktop-src/Stg/structured-storage-start-page.md)
- [references/win32/desktop-src/Ktm/kernel-transaction-manager-portal.md](../references/win32/desktop-src/Ktm/kernel-transaction-manager-portal.md)

## 实现 TODO

- [ ] 建立 **ALPC 端口** 与 MSDN「管道/邮槽/套接字」概念的对照表（何种场景用 ALPC 替代）。
- [ ] 将 **RPC 端点映射** 标为用户态或内核服务远期；注明与 `Rpc` 文档的兼容性目标（可「不支持」）。
- [ ] 列出 **服务控制管理器（SCM）** 状态机：`CreateService`、`StartService` 与 `services.exe` 启动链。
- [ ] 对照 `ProcThread`，核对 **进程创建参数**（环境块、句柄继承）与当前 `EProcess` bring-up。
- [ ] 将 **作业对象（Job）**、**进程组** 与安全沙箱（AppContainer）关联标注。
- [ ] 评估 **命名管道（\\.\pipe）** 在 ZirconOSFluent 命名空间中的表示（`\Device\NamedPipe` 类比）。
- [ ] 将 **本地 RPC / ncalrpc** 与 csrss / LSASS 通信模式做文献笔记（不要求实现）。
- [ ] 记录 **线程池、线程局部存储** 与 PE TLS 目录的完整实现缺口（链 loader）。
- [ ] 将 **COM 编组 / STG** 与 `com` 桩、`Stg` 文档标为 WinRT/COM 扩展线。
- [ ] 核对 **KTM** 与事务性 NTFS 是否在文件系统路线图中（默认低优先级）。
- [ ] 在 `alpc/` 增加 **跨会话端口 ACL** 的设计要点（Session 0 隔离）。
- [ ] 将 **调试对象 / 进程快照**（`toc` 中 proc_snap 外链）列为调试器远期依赖。
- [ ] 统一 **同步原语**（mutex/event/semaphore）与内核 `dispatcher` 对象命名。
- [ ] 为 **WOW64 进程创建** 增加与 `wow64.rs` 的交叉引用（专用 syscall 路径）。
