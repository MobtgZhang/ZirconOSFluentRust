# 第三方参考（`references/`）

**英文**：[../en/References-Policy.md](../en/References-Policy.md)

仓库可能在 `references/` 下附带**只读**目录供离线查阅，**除非另行声明，否则不代表你可按 ZirconOSFluent 仓库许可证随意复制其中正文**。

## `references/win32`（类 Microsoft Learn 文档树）

- **仅**用于理解公开的 API 名称、调用顺序、子系统角色等行为层面信息。
- **禁止**将其中段落、长表格、教程正文或示例代码原样粘贴到本仓库源码或文档中。
- 若对外需要引用，优先在评审/讨论中链到微软官方文档；**仓库内表述须为原创**。
- 兼容目标与商标说明见 [Architecture.md](Architecture.md)。

## `references/r-efi`（UEFI Rust 绑定）

- UEFI 为**公开规范**；`r-efi` 有助于对照**语义**（内存类型、协议形状等）。
- 本工作区以 [`nt10-boot-protocol`](../../crates/nt10-boot-protocol/src/lib.rs) 与 ZBM10 为 handoff **规范来源**；内核侧无互操作需求时不要重复引入 `r-efi` 类型。
- 代码注释应**简短且原创**，避免逐字抄录规范正文。

## 实施建议

- **标准**：UEFI、ACPI、SMBIOS、PE/COFF 等——在必要时用标准名称指代，按语义实现，不复制版权正文。
- **Rust 模块**：用模块文档描述 **ZirconOSFluent** 自身行为；流程约定见 [PROCESS_NT10.md](PROCESS_NT10.md)。
