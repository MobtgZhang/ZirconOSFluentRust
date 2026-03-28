# Hyper-V awareness, modern security, WinRT/UWP, multi-arch

**中文**：[../cn/Virtualization-Security-WinRT.md](../cn/Virtualization-Security-WinRT.md)

Draft §20–§23. **Status**: design target; stubs in [hyperv/](../../crates/nt10-kernel/src/hyperv/), [vbs/](../../crates/nt10-kernel/src/vbs/), [arch/x86_64/cet.rs](../../crates/nt10-kernel/src/arch/x86_64/cet.rs), …

## 1. Hyper-V awareness (§20)

### 1.1 Detection

**CPUID** leaves `0x40000000`–`0x40000006` to detect **Microsoft Hypervisor** (e.g. `"Microsoft Hv"`).

### 1.2 Enlightenments

Examples: `HVCALL_FLUSH_VIRTUAL_ADDRESS_LIST`, `HVCALL_NOTIFY_LONG_SPIN_WAIT`, reference time page, **SynIC**, **VMBus**. Degrade to no-ops when not a Hyper-V guest.

### 1.3 Paths in this repo

[hyperv/detect.rs](../../crates/nt10-kernel/src/hyperv/detect.rs), `hypercall.rs`, `synic.rs`, `enlighten.rs`, `vmbus.rs`.

## 2. Modern security (§21)

### 2.1 CFG

Per-process **CFG bitmap** (~16-byte granularity); loader fills; indirect calls validate targets.

### 2.2 CET

Shadow stack: [arch/x86_64/cet.rs](../../crates/nt10-kernel/src/arch/x86_64/cet.rs) plans CR4.CET and MSRs; bad `RET` → `#CP`.

### 2.3 DEP

**NX** in page tables; `VirtualProtect`-style updates in [mm/vm.rs](../../crates/nt10-kernel/src/mm/vm.rs).

### 2.4 Others

MIC, HVCI: [Processes-Security-IO.md](Processes-Security-IO.md) and [se/hvci.rs](../../crates/nt10-kernel/src/se/hvci.rs).

## 3. VBS

[vbs/vsm.rs](../../crates/nt10-kernel/src/vbs/vsm.rs), `skci.rs`, `credguard.rs`: VSM, SKCI, Credential Guard stubs.

## 4. WinRT / UWP (§22)

- **AppModel**: package identity, AppContainer, **RoActivateInstance** → ALPC → out-of-proc COM.
- **IAsyncOperation** and related async patterns.
- Prefer stubs early; Win32 subsystem completeness first.

## 5. Multi-arch (§23)

| Arch | Priority | Syscall | Paging | Virt |
|------|----------|---------|--------|------|
| x86_64 | Primary | SYSCALL/SYSRET | 4 (5 w/ LA57) | VMX |
| aarch64 | Secondary | SVC #0 | 4-level | HVC |
| riscv64 | Experimental | ECALL | Sv48 | — |
| loongarch64 | Experimental | SYSCALL | 4-level | — |
| mips64el | Experimental | SYSCALL | 3/4-level | — |

**WOW64**: x86 PE32 on x64; ARM32 + PE32 on ARM64 (draft).

**This repo**: [arch/](../../crates/nt10-kernel/src/arch/) contains `x86_64`, `aarch64`, etc. stubs.

## 6. Related docs

- [Kernel-Executive-and-HAL.md](Kernel-Executive-and-HAL.md)
- [Build-Test-Coding.md](Build-Test-Coding.md)
