# Processes, security, I/O, filesystem, ALPC

**中文**：[../cn/Processes-Security-IO.md](../cn/Processes-Security-IO.md)

Draft §10–§14: **PS, SE, I/O manager, FS, ALPC**. **Status**: design target; stubs under [crates/nt10-kernel/src/](../../crates/nt10-kernel/src/).

## 1. Process / thread manager (PS)

### 1.1 EPROCESS (planned)

Draft fields include: `KProcess` (first field), PID, lists, token, locks, **VAD root**, PEB pointer, session ID, image name, Job, **mitigations** (CFG/ASLR/DEP), **PsProtection (PPL)**, …

### 1.2 Protected Process Light (PPL)

NT10-style PPL limits handle access to critical processes (e.g. LSASS); planned in [ps/process.rs](../../crates/nt10-kernel/src/ps/process.rs).

### 1.3 Modules in this repo

[ps/process.rs](../../crates/nt10-kernel/src/ps/process.rs), `thread.rs`, `peb.rs`, `job.rs`, `affinity.rs`, `server.rs`, …

## 2. Security (SE)

### 2.1 Mandatory Integrity Control (MIC)

Integrity levels above DAC: System, High, Medium (default user), Low, Untrusted. **No-Write-Up**.

### 2.2 HVCI surface

[se/hvci.rs](../../crates/nt10-kernel/src/se/hvci.rs) plans to submit CI policy to the VTL1 secure kernel; without Hyper-V, degrade to local checks.

### 2.3 Other files

`token.rs`, `sid.rs`, `acl.rs`, `audit.rs`, `privilege.rs`, `integrity.rs` under [se/](../../crates/nt10-kernel/src/se/).

## 3. I/O manager and driver model

### 3.1 IRP layering

`NtReadFile` → `IoCallDriver` → filter stack → FDO → `IoCompleteRequest`.

### 3.2 IOCP

Planned integration with ALPC completion lists; `NtCreateIoCompletion` / `NtRemoveIoCompletion` semantics.

### 3.3 WDF stub

[io/wdf.rs](../../crates/nt10-kernel/src/io/wdf.rs): minimal KMDF objects (WDFDEVICE, WDFREQUEST, WDFQUEUE).

### 3.4 Modules in this repo

[iomgr.rs](../../crates/nt10-kernel/src/io/iomgr.rs), `irp.rs`, `device.rs`, `driver.rs`, `cancel.rs`, `completion.rs`, `wdf.rs`.

**Drivers**: [drivers/video/](../../crates/nt10-kernel/src/drivers/video/), `storage/`, `input/`, `net/`, `bus/` (VirtIO, AHCI, NVMe, E1000, …).

## 4. Filesystem (FS)

NTFS roadmap: MFT, `$DATA`, directory index, `$LogFile`, USN, reparse points; **FAT32** for system volume; optional **CDFS**.

Modules: [fs/vfs.rs](../../crates/nt10-kernel/src/fs/vfs.rs), `fat32.rs`, `ntfs/*.rs`, `cdfs.rs`.

Feature/phase table: [ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md) §13.

## 5. ALPC

NT10 uses **ALPC** instead of classic LPC:

- Port objects + security descriptors
- **ALPC_MESSAGE_ATTRIBUTES** (token, view, handle, direct memory, …)
- **Completion lists**
- Large messages via **section** mapping

Syscall shapes: `NtAlpcCreatePort`, `NtAlpcConnectPort`, `NtAlpcSendWaitReceivePort`, … (draft §14).

Paths: [alpc/port.rs](../../crates/nt10-kernel/src/alpc/port.rs), `message.rs`, `connection.rs`, `completion.rs`.

## 6. Related docs

- [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md)
- [Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md)
