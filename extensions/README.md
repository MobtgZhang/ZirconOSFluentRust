# Win32 参考文档 → ZirconOS（NT10）扩展任务索引

本目录将 [references/win32/desktop-src](../references/win32/desktop-src)（Microsoft 桌面 Win32 文档镜像）按 **主题阶段** 拆成可勾选的实现 TODO，并与本仓库路线图、内核/子系统桩代码对齐。

## 方法说明

- **不复制 MSDN 正文**：只列仓库内已有 `.md` 路径作为阅读入口；细节以原文为准。extensions 内说明以原创总结为主，版权与写作约定见 [phase-00-inventory.md](./phase-00-inventory.md) 首节。
- **以 TOC 为纲**：官方章节结构见 [references/win32/desktop-src/toc.yml](../references/win32/desktop-src/toc.yml)。`toc.yml` 中 `href` 多为相对路径（如 `./winmsg/windowing.md`），本仓库在磁盘上部分目录名为 **PascalCase**（如 `LearnWin32/`、`Controls/`、`SbsCs/`），引用时请按实际路径打开（见 [phase-00-inventory.md](./phase-00-inventory.md)）。
- **增量维护**：新增能力时优先在本阶段文件增加「参考文档链接 + TODO 条目」，避免另起长篇设计文。

## 阶段文件一览

| 阶段文件 | 主题 | 主要对齐 Roadmap Phase | 主要代码区域 |
|----------|------|------------------------|--------------|
| [phase-00-inventory.md](./phase-00-inventory.md) | 索引规则与维护约定 | — | — |
| [phase-01-win32-foundation.md](./phase-01-win32-foundation.md) | LearnWin32、头文件、64 位、子系统概念 | 7–8 | `crates/nt10-kernel/src/loader/`、`libs/`、`subsystems/win32/` |
| [phase-02-sessions-desktops.md](./phase-02-sessions-desktops.md) | 窗口站、桌面、会话 | 5、9 | `ob/namespace.rs`、`servers/smss.rs`、`subsystems/win32/csrss_host.rs` |
| [phase-03-windowing-menus.md](./phase-03-windowing-menus.md) | 窗口、消息、对话框、菜单、控件 | 9–10 | `subsystems/win32/user32.rs`、`desktop/fluent/` |
| [phase-04-gdi-drawing.md](./phase-04-gdi-drawing.md) | GDI / GDI+ | 10 | `subsystems/win32/gdi32.rs`、未来 Win32k |
| [phase-05-input-stack.md](./phase-05-input-stack.md) | 指针、触摸、键盘鼠标、TSF、高 DPI | 10、14 | `desktop/fluent/session.rs`、输入管线 |
| [phase-06-dwm-composition.md](./phase-06-dwm-composition.md) | DWM、合成、交换链 | 10、14 | `desktop/fluent/dwm.rs`、`drivers/video/wddm2/` |
| [phase-07-shell-environment.md](./phase-07-shell-environment.md) | Shell、属性、搜索、剪贴板/数据交换 | 14 | `desktop/fluent/shell.rs`、`explorer_view.rs`、`resources/` |
| [phase-08-ipc-services.md](./phase-08-ipc-services.md) | IPC、RPC、服务、进程线程 | 5–6、8 | `alpc/`、`servers/`、`subsystems/win32/` |
| [phase-99-backlog-graphics.md](./phase-99-backlog-graphics.md) | DirectX/Direct2D/DXGI 等（大粒度 backlog） | 非 14 前置 | 长期 |

## 仓库内其它文档

- 中文路线图：[docs/cn/Roadmap-and-TODO.md](../docs/cn/Roadmap-and-TODO.md)
- 加载器 / Win32k / Fluent 映射：[docs/cn/Loader-Win32k-Desktop.md](../docs/cn/Loader-Win32k-Desktop.md)

## 可选：TOC 路径附录

从 `toc.yml` 抽取 **仅含 `.md` 路径列表** 的附录：运行仓库根目录下 [`scripts/gen-extensions-index.sh`](../scripts/gen-extensions-index.sh)。默认输出为 `extensions/REFERENCE-INDEX.auto.md`（已在 [`.gitignore`](../.gitignore) 中忽略；若需入库可改 `EXTENSIONS_INDEX_OUT` 或从 `.gitignore` 移除对应行）。
