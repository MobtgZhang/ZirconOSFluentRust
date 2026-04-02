# Phase 07：Shell、属性模型、搜索与数据交换

## 本阶段目标

梳理 **Shell 命名空间、属性系统、Windows Search、剪贴板/拖放** 与 Fluent `shell.rs`、`explorer_view` 及 VFS 的演进路线（Roadmap Phase 14）。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/shell/shell-entry.md](../references/win32/desktop-src/shell/shell-entry.md)
- [references/win32/desktop-src/properties/windows-properties-system.md](../references/win32/desktop-src/properties/windows-properties-system.md)
- [references/win32/desktop-src/search/windows-search.md](../references/win32/desktop-src/search/windows-search.md)
- [references/win32/desktop-src/dataxchg/data-exchange.md](../references/win32/desktop-src/dataxchg/data-exchange.md)
- [references/win32/desktop-src/desktop-win32-code-samples.md](../references/win32/desktop-src/desktop-win32-code-samples.md)
- [references/win32/desktop-src/FileIO/file-systems.md](../references/win32/desktop-src/FileIO/file-systems.md)
- [references/win32/desktop-src/uxguide/guidelines.md](../references/win32/desktop-src/uxguide/guidelines.md)
- [references/win32/desktop-src/Intl/international-support.md](../references/win32/desktop-src/Intl/international-support.md)
- [references/win32/desktop-src/winauto/accessibility.md](../references/win32/desktop-src/winauto/accessibility.md)

## 实现 TODO

- [ ] 将 **已知文件夹 ID（KNOWNFOLDERID）** 与 ZirconOSFluent 虚拟路径（桌面、文档、回收站）对照表写入设计备注。
- [ ] 列出 **IShellFolder / IShellItem** 等最小接口集，用于未来 `explorer_view` 与 VFS 枚举对接。
- [ ] 规划 **桌面图标视图**：列表/大图标与 `DefView` 行为的子集目标。
- [ ] 将 **属性处理程序、列处理程序** 标为 Shell 扩展远期。
- [ ] 定义 **回收站** 语义：假删除 vs 真实移动与 FAT/NTFS 特性（链 `fs/`）。
- [ ] 评估 **Windows Search / 索引器** 是否引入（默认否）；若否，在文档中明确「仅路径遍历搜索」。
- [ ] 实现 **剪贴板（CF_UNICODETEXT、CF_HDROP）** 最小子集，与 `data-exchange.md` 对齐。
- [ ] 将 **OLE 拖放** 与 **IDataObject** 标为 COM 依赖，排在 combase 成熟之后。
- [ ] 将 **快捷方式（.lnk）** 解析纳入 loader 或用户态库任务清单（二进制格式）。
- [ ] 核对 **上下文菜单扩展** 与当前 `shell.rs` 右键菜单硬编码的迁移路径。
- [ ] 引用 [resources/manifest.json](../resources/manifest.json) 中图标 ID 与 Shell 图标索引的命名一致性检查。
- [ ] 记录 **任务栏固定列表（Jump List）** 为远期。
- [ ] 将 **辅助功能（MSAA/UIA）** 与 Fluent 控件树预留钩子。
- [ ] 国际化：对照 `Intl`，列出 Shell 字符串资源与字体回退策略（与 Loader 文档第 5 节一致）。
- [ ] 将 **通知区域（托盘）** 与气泡通知 API 列入 Shell 第二阶段。
