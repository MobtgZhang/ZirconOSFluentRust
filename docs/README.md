# ZirconOSFluent 文档 / Documentation

本目录描述 **ZirconOS NT10** 目标内核架构（对齐 Windows NT **10.0.19045** 设计意图）及本仓库实现范围。详细正文按语言分栏，**中英文文件名一一对应**，便于对照维护。

**实现语言**：**Rust**（内核与引导 crate 为 `no_std`）。**构建**：**Cargo**（工作区根 [Cargo.toml](../Cargo.toml)）。

| 语言 | 入口 |
|------|------|
| 中文 | [docs/cn/README.md](cn/README.md) |
| English | [docs/en/README.md](en/README.md) |

**代码树（内核模块骨架）**：[crates/nt10-kernel/src/](../crates/nt10-kernel/src/)（与 [ideas/ZirconOS_NT10_Architecture.md](../ideas/ZirconOS_NT10_Architecture.md) §4 对应）。**UEFI 引导占位**：[crates/nt10-boot-uefi/](../crates/nt10-boot-uefi/)。

**架构母版（草案，长文单文件）**：[ideas/ZirconOS_NT10_Architecture.md](../ideas/ZirconOS_NT10_Architecture.md)。`docs/cn` 与 `docs/en` 中的文档由其拆分、重写并双语文档化；若与母版冲突，以母版为技术来源、以 `docs` 为对外说明为准（更新时请同步母版或注明差异）。

**免责声明**：ZirconOS 与 Microsoft 无关联。「Windows」「Windows 10」等商标归 Microsoft Corporation 所有；本文档仅用于描述兼容与对标目标。
