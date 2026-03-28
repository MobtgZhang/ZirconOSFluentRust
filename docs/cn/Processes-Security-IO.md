# 进程与安全、I/O、文件系统、ALPC

**English**: [../en/Processes-Security-IO.md](../en/Processes-Security-IO.md)

本文对应母版 §10–§14：**PS、SE、I/O 管理器、FS、ALPC**。**实现状态**：目标设计；模块桩见 [crates/nt10-kernel/src/](../../crates/nt10-kernel/src/) 下对应目录。

## 1. 进程与线程管理器（PS）

### 1.1 EPROCESS（规划要点）

母版示例字段包括：`KProcess`（调度控制块，需为首字段）、PID、链表、令牌、锁、**VAD 根**、PEB 指针、会话 ID、映像名、Job、**缓解标志**（CFG/ASLR/DEP）、**PsProtection（PPL）** 等。

### 1.2 受保护进程（PPL）

NT10 相关：**受保护进程轻量级（PPL）**，对 LSASS 等关键进程限制未授权句柄访问；规划在 [ps/process.rs](../../crates/nt10-kernel/src/ps/process.rs) 实现签名级别与类型检查。

### 1.3 目标模块（本仓库）

[ps/process.rs](../../crates/nt10-kernel/src/ps/process.rs)、`thread.rs`、`peb.rs`、`job.rs`、`affinity.rs`、`server.rs` 等。

## 2. 安全子系统（SE）

### 2.1 强制完整性控制（MIC）

在 DAC 之上叠加完整性级别，例如：System、High、Medium（普通用户默认）、Low、Untrusted。**No-Write-Up**：低完整性不可写向高完整性对象。

### 2.2 HVCI 接口

[se/hvci.rs](../../crates/nt10-kernel/src/se/hvci.rs) 规划向 VTL1 安全内核提交代码完整性策略；无 Hyper-V 时可退化为本地策略检查。

### 2.3 其他目标文件（本仓库）

`token.rs`、`sid.rs`、`acl.rs`、`audit.rs`、`privilege.rs`、`integrity.rs` 等（目录 [se/](../../crates/nt10-kernel/src/se/)）。

## 3. I/O 管理器与驱动框架

### 3.1 IRP 分层

典型路径：`NtReadFile` → `IoCallDriver` → 过滤驱动链 → FDO → 完成 `IoCompleteRequest`。

### 3.2 I/O 完成端口（IOCP）

规划与 ALPC 完成列表等机制结合，对应 `NtCreateIoCompletion` / `NtRemoveIoCompletion` 等语义。

### 3.3 WDF 桩

[io/wdf.rs](../../crates/nt10-kernel/src/io/wdf.rs)：KMDF 核心对象（WDFDEVICE、WDFREQUEST、WDFQUEUE）最小桩，便于移植驱动。

### 3.4 目标模块（本仓库）

[io/iomgr.rs](../../crates/nt10-kernel/src/io/iomgr.rs)、`irp.rs`、`device.rs`、`driver.rs`、`cancel.rs`、`completion.rs`、`wdf.rs`。

**内核驱动目录**：[drivers/video/](../../crates/nt10-kernel/src/drivers/video/)、`storage/`、`input/`、`net/`、`bus/`（VirtIO、AHCI、NVMe、E1000 等，见母版树）。

## 4. 文件系统层（FS）

母版 **NTFS** 分阶段目标包括：MFT、`$DATA`、目录索引、`$LogFile`、USN、重解析点等；另有 **FAT32**（系统卷）、可选 **CDFS**。

目标模块：[fs/vfs.rs](../../crates/nt10-kernel/src/fs/vfs.rs)、`fat32.rs`、`ntfs/*.rs`、`cdfs.rs`。

母版中 **特性与 Phase 对照表** 见 [ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md) §13。

## 5. ALPC（高级本地过程调用）

NT10 以 **ALPC** 替代旧式 LPC。核心概念：

- 端口对象（命名/匿名）与安全描述符
- **ALPC_MESSAGE_ATTRIBUTES**（令牌、视图、句柄、直接内存等）
- **完成列表**（高吞吐、少拷贝路径）
- 大消息经 **节映射** 传递

规划导出的系统调用形态包括：`NtAlpcCreatePort`、`NtAlpcConnectPort`、`NtAlpcSendWaitReceivePort`、`NtAlpcAcceptConnectPort`、`NtAlpcDisconnectPort` 等（见母版 §14）。

目标目录：[alpc/port.rs](../../crates/nt10-kernel/src/alpc/port.rs)、`message.rs`、`connection.rs`、`completion.rs`。

## 6. 相关文档

- [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md)
- [Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md)
