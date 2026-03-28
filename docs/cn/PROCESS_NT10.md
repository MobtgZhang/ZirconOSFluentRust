# NT10 开发流程与文档维护

**English**: [../en/PROCESS_NT10.md](../en/PROCESS_NT10.md)

## 1. 文档与母版关系

- **对外长篇**：`docs/cn/*.md`、`docs/en/*.md`（双语同名对照）。
- **技术母版（单文件全集）**：[ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md)。

当设计变更时：

1. 先更新 **ideas** 或先在 **docs** 定型（团队约定一种为源），再同步另一处，避免漂移。
2. 修改 **Phase 状态** 时，同时更新 `docs/cn/Roadmap-and-TODO.md` 与 `docs/en/Roadmap-and-TODO.md`。

## 2. 新代码应落在何处（本仓库 Rust 树）

母版第 4 节 `src/...` 映射到 **Cargo 工作区**如下：

- **UEFI 引导** → [crates/nt10-boot-uefi/](../../crates/nt10-boot-uefi/)（ZBM10；日后可再拆 `boot/zbm10/` 资源目录）
- **架构相关** → [crates/nt10-kernel/src/arch/<arch>/](../../crates/nt10-kernel/src/arch/)
- **HAL** → [crates/nt10-kernel/src/hal/<arch>/](../../crates/nt10-kernel/src/hal/)
- **执行体 / MM / OB / PS / SE / IO / ALPC / FS** → [ke/](../../crates/nt10-kernel/src/ke/)、[mm/](../../crates/nt10-kernel/src/mm/)、[ob/](../../crates/nt10-kernel/src/ob/)、[ps/](../../crates/nt10-kernel/src/ps/)、[se/](../../crates/nt10-kernel/src/se/)、[io/](../../crates/nt10-kernel/src/io/)、[alpc/](../../crates/nt10-kernel/src/alpc/)、[fs/](../../crates/nt10-kernel/src/fs/)
- **驱动** → [drivers/](../../crates/nt10-kernel/src/drivers/)
- **用户态库（内核侧桩/共享类型）** → [libs/](../../crates/nt10-kernel/src/libs/)
- **系统服务进程** → [servers/](../../crates/nt10-kernel/src/servers/)
- **Win32 子系统** → [subsystems/win32/](../../crates/nt10-kernel/src/subsystems/win32/)
- **Fluent 桌面（模块桩）** → [desktop/fluent/](../../crates/nt10-kernel/src/desktop/fluent/)

## 3. 资源路径约定

静态资源见仓库根 **[`resources/`](../../resources/)** 与 **[`resources/manifest.json`](../../resources/manifest.json)**。

## 4. 提交与评审建议

- 内核行为变更：附带或更新 **中英文** 对应小节。
- 跨子系统：更新 [Architecture.md](Architecture.md) 或相关分册的「相关文档」互链。
- 商标与第三方：不引入非授权资源；许可证文件若在仓库根添加，应与 MANIFEST/声明一致。
- `references/` 下离线文档（如类 Learn 正文）：向仓库粘贴前请先读 [References-Policy.md](References-Policy.md)。

## 5. 入口索引

- [docs/cn/README.md](README.md)
- [docs/README.md](../README.md)
