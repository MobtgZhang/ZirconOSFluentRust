# 内存管理器（MM）与对象管理器（OB）

**English**: [../en/Memory-and-Objects.md](../en/Memory-and-Objects.md)

本文对应母版 §8–§9。**实现状态**：目标设计；模块桩见 [crates/nt10-kernel/src/mm/](../../crates/nt10-kernel/src/mm/)、[ob/](../../crates/nt10-kernel/src/ob/)。

## 1. 内存管理器（MM）

### 1.1 物理内存

- 早期：**位图分配器**；完整阶段：**伙伴系统**。
- 支持 4KB、2MB 大页、1GB 巨页。
- **NUMA**：按节点维护空闲链表与分配策略。

### 1.2 虚拟地址空间布局（x86_64，LA57 关闭时）

| 区间 | 用途 |
|------|------|
| `0x0000…` – `0x00007FFF_FFFF_FFFF` | 用户空间（128 TB） |
| `0xFFFF8000…` – `0xFFFF8FFF_FFFF_FFFF` | 内核直接映射（16 TB） |
| `0xFFFF9000…` – `0xFFFF9FFF_FFFF_FFFF` | vmalloc 区 |
| `0xFFFFA000…` – `0xFFFFAFFF_FFFF_FFFF` | NonPagedPool |
| `0xFFFFB000…` – `0xFFFFBFFF_FFFF_FFFF` | PagedPool |
| `0xFFFFF800…` – `0xFFFFFFFFFFFFFFFF` | 内核映像、HAL、PFN 数据库等 |

**LA57**：可选 5 级页表，见母版 §23 与 [arch/x86_64/paging.rs](../../crates/nt10-kernel/src/arch/x86_64/paging.rs) 规划。

### 1.3 VAD 树

用户态虚拟区间用 **VAD（Virtual Address Descriptor）AVL 树** 管理：区域类型（堆/栈/映射/保留）、保护属性（如 `PAGE_EXECUTE_READ`）、提交状态等。

### 1.4 节对象（Section）

`NtCreateSection` 语义规划包括：

- 映像节（`SEC_IMAGE`）
- 匿名提交节
- **写时复制（COW）**

### 1.5 目标模块（本仓库）

[mm/phys.rs](../../crates/nt10-kernel/src/mm/phys.rs)、`vm.rs`、`paging.rs`、`heap.rs`、`large_page.rs`、`numa.rs`、`pagefile.rs`、`section.rs`、`working_set.rs` 等。

## 2. 对象管理器（OB）

NT 对象模型的核心：每个对象前有 **对象头**，母版示例（Rust）：

```rust
#[repr(C)]
pub struct ObjectHeader {
    pub pointer_count: i64,
    pub handle_count: i64,
    pub type_index: u8,
    pub flags: ObjectFlags,
    pub security_descriptor: *mut SecurityDescriptor,
}
```

可选信息头（名称、创建者、配额等）可位于该结构之前的变长区域（与真实 NT 布局对齐时需谨慎设计）。

### 2.1 命名空间（逻辑层次）

```
\
├── Device\
├── ObjectTypes\
├── KnownDlls\
├── Windows\WindowStations\WinSta0\Default\
├── Sessions\
└── GLOBAL??（DOS 设备名，如 C: → 卷设备）
```

### 2.2 目标模块（本仓库）

[ob/object.rs](../../crates/nt10-kernel/src/ob/object.rs)、`handle.rs`、`namespace.rs`、`symlink.rs`、`directory.rs`、`wait.rs` 等。

## 3. 相关文档

- [Processes-Security-IO.md](Processes-Security-IO.md)（进程与句柄、安全描述符）
- [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md)（PE 映射与节对象）
