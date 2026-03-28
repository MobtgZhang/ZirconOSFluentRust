# Phase 01：Win32 基础与平台约定

## 本阶段目标

对齐 **PE 子系统、入口点、消息泵雏形、头文件与 64 位约定** 与 ZirconOS 加载器及用户态 API 桩（Roadmap Phase 7–8），为后续真 user32/消息循环打术语与行为基础。阅读材料仅作索引；实现说明用本仓库原创表述。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/desktop-programming.md](../references/win32/desktop-src/desktop-programming.md)
- [references/win32/desktop-src/whats-new.md](../references/win32/desktop-src/whats-new.md)
- [references/win32/desktop-src/LearnWin32/learn-to-program-for-windows.md](../references/win32/desktop-src/LearnWin32/learn-to-program-for-windows.md)
- [references/win32/desktop-src/LearnWin32/introduction-to-windows-programming-in-c--.md](../references/win32/desktop-src/LearnWin32/introduction-to-windows-programming-in-c--.md)
- [references/win32/desktop-src/LearnWin32/winmain--the-application-entry-point.md](../references/win32/desktop-src/LearnWin32/winmain--the-application-entry-point.md)
- [references/win32/desktop-src/LearnWin32/what-is-a-window-.md](../references/win32/desktop-src/LearnWin32/what-is-a-window-.md)
- [references/win32/desktop-src/LearnWin32/window-messages.md](../references/win32/desktop-src/LearnWin32/window-messages.md)
- [references/win32/desktop-src/LearnWin32/writing-the-window-procedure.md](../references/win32/desktop-src/LearnWin32/writing-the-window-procedure.md)
- [references/win32/desktop-src/LearnWin32/module-4--user-input.md](../references/win32/desktop-src/LearnWin32/module-4--user-input.md)
- [references/win32/desktop-src/LearnWin32/overview-of-the-windows-graphics-architecture.md](../references/win32/desktop-src/LearnWin32/overview-of-the-windows-graphics-architecture.md)
- [references/win32/desktop-src/WinProg/using-the-windows-headers.md](../references/win32/desktop-src/WinProg/using-the-windows-headers.md)
- [references/win32/desktop-src/WinProg64/programming-guide-for-64-bit-windows.md](../references/win32/desktop-src/WinProg64/programming-guide-for-64-bit-windows.md)
- [references/win32/desktop-src/desktop-app-technologies.md](../references/win32/desktop-src/desktop-app-technologies.md)
- [references/win32/desktop-src/Dlls/dynamic-link-libraries.md](../references/win32/desktop-src/Dlls/dynamic-link-libraries.md)
- [references/win32/desktop-src/apiindex/api-index-portal.md](../references/win32/desktop-src/apiindex/api-index-portal.md)

## 实现 TODO：PE 与启动

- [ ] 在文档或代码注释中固定 **GUI 子系统 PE** 与 `WinMain` / `wWinMain` 入口约定，与当前 `load_pe` bring-up 行为对照。
- [ ] 梳理 `IMAGE_SUBSYSTEM_WINDOWS_GUI` 与控制台子系统差异，标注 ZirconOS 当前仅验证路径。
- [ ] 文档化 GUI 入口与 CRT/启动例程的期望关系（不要求一步实现完整 CRT）。
- [ ] 列出「首个可运行 ring3 GUI 最小进程」所需：映像映射、用户栈、PEB/TEB、到达用户入口的最短依赖链。
- [ ] 对照可选头与数据目录：重定位、导入、TLS、延迟导入、异常目录 — 标出已实现与未实现。
- [ ] 在 `loader/pe_image.rs`（及相关模块）侧维护「与 Win32 启动路径相关字段」检查表，随实现更新。

## 实现 TODO：消息与窗口模型（概念）

- [ ] 将 **消息队列、取消息、分发、窗口过程** 语义映射到 `user32` 桩的未来行为表（应用线程视角）。
- [ ] 标注 **投递（post）与发送（send）** 的差异，以及当前内核线程模型下可延后的部分。
- [ ] 规划 **第一个 ring3 Win32 迷你程序**（消息泵 + 空窗口）所需的最低 syscall/API 集合并列清单。

## 实现 TODO：64 位与 ABI

- [ ] 对照 [WinProg64 指南](../references/win32/desktop-src/WinProg64/programming-guide-for-64-bit-windows.md)，列出 `HWND`、`LONG_PTR`、`WPARAM`、`LPARAM` 等在 Rust FFI 中的宽度与 `#[repr]` 原则。
- [ ] 记录 **x64 调用约定** 与 **syscall 网关寄存器** 约定，避免文档与 FFI 混用 32 位习惯。

## 实现 TODO：错误与 API 分层

- [ ] 为 `ntdll` / `kernel32` 桩约定 **错误码与 SetLastError/GetLastError** 传播策略（即使暂为 stub）。
- [ ] 约定 Win32 层 `GetLastError` 与 `NTSTATUS` 的边界（哪一层转换、哪一层只透传）。
- [ ] 核对 `Rtl*` / `Nt*` 命名与 Win32 层 API 分层，与 [Loader-Win32k-Desktop.md](../docs/cn/Loader-Win32k-Desktop.md) 第 3 节一致化说明。

## 实现 TODO：DLL、清单与安全交叉引用

- [ ] 将 DLL 搜索顺序、激活上下文（见 [SbsCs](../references/win32/desktop-src/SbsCs/isolated-applications-and-side-by-side-assemblies-portal.md)）与加载器 `import_` 解析顺序文档化交叉引用。
- [ ] 评估 **CFG / DEP / ASLR** 等标志与 PE Optional Header 字段对应关系（详细实现链到 Roadmap Phase 12）。
- [ ] 对齐 **Unicode（UTF-16）** 字符串约定与内核/用户缓冲边界（未来 ALPC 显示名等）。

## 实现 TODO：版本与 COM 钩子

- [ ] 维护内部 **NT10 目标构建号** 矩阵（如 19041）：从 `whats-new` 类文档只记 API/行为 **标题级** 笔记，不抄正文。
- [ ] 为 COM 初识保留钩子：阅读 LearnWin32 中 COM 模块，标注与 `combase` 远期关系（本阶段不实现）。
