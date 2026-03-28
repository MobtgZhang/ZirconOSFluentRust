# 加载器、Win32k/WDDM、用户态库与会话 — 及本仓库映射

**English**: [../en/Loader-Win32k-Desktop.md](../en/Loader-Win32k-Desktop.md)

本文对应母版 §15–§19，并说明 **本仓库** 中 Loader / Win32 / Fluent 相关 **Rust 模块桩** 的位置。

## 1. 加载器（Loader）

### 1.1 PE32+ 映像加载（规划流程）

```
NtCreateSection(SEC_IMAGE)
  → 解析 MZ / COFF / Optional Header
  → 校验 Machine / Subsystem / DllCharacteristics
  → 映射 .text / .data / .rdata / .rsrc …
  → 重定位（ASLR 非零基址时）
  → 解析导入表并递归加载 DLL
  → 页面保护（NX / 只读 / 可写）
  → CFG 位图（若启用 Guard CF）
  → TLS
  → 填充 PEB.Ldr（LDR_DATA_TABLE_ENTRY 链表）
```

### 1.2 ASLR

三级随机化：映像基址、堆、栈（[loader/aslr.rs](../../crates/nt10-kernel/src/loader/aslr.rs) 规划）。

### 1.3 目标路径（本仓库）

[loader/pe.rs](../../crates/nt10-kernel/src/loader/pe.rs)、`pe32.rs`、`elf.rs`（WSL 桩）、`import_.rs`、`reloc.rs`、`aslr.rs`。

## 2. 图形子系统（Win32k）与 WDDM 2.x（目标）

```
用户态                          内核态
D3D12 Runtime    →  dxgkrnl（DirectX 内核）
user32 / gdi32   →  win32k.sys
                   → wddm2 / VidPN / 显示 DDI
                   → framebuffer（QEMU GOP 阶段）
```

**DWM（桌面窗口管理器）**（目标）：每窗口离屏缓冲、Alpha 合成、Acrylic 模糊、Mica、动画缓动等。

**本仓库**：Win32k/WDDM 仅为 [drivers/video/wddm2/](../../crates/nt10-kernel/src/drivers/video/wddm2/) 等目录下 **占位模块**。

## 3. 用户态 API 库（目标）

- **ntdll**：`Nt*` / `Zw*` / `Rtl*` / `Ldr*`，syscall 使用 NT10（19041 基准）编号。
- **kernel32 / kernelbase**：NT6+ 分层，实质 API 多在 kernelbase。
- **combase / winrt_rt**：COM 与 WinRT 激活（`RoActivateInstance` 等），见母版 §17、§22。

**本仓库桩**：[libs/](../../crates/nt10-kernel/src/libs/) 目录。

## 4. 会话管理器与 Win32 子系统（目标）

- **SMSS**：页面文件、`\Sessions\0`、ApiPort、启动 csrss / wininit。
- **csrss**：窗口站/桌面、消息队列、Csr* 注册、输入分发。
- **ConPTY**：`conhost`、VT 序列，见母版 §19。

启动链（摘要）：内核 Phase 0–7 → `smss` → `csrss` → `wininit` → `services` / `lsass` / `winlogon` → `explorer`。

**本仓库桩**：[servers/](../../crates/nt10-kernel/src/servers/)、[subsystems/win32/](../../crates/nt10-kernel/src/subsystems/win32/)。

---

## 5. Fluent 桌面（本仓库）

当前仓库 **未实现** 可运行的 Fluent Shell 或独立宿主程序；与母版 Phase 14 视觉目标对应的仅是 **[desktop/fluent/](../../crates/nt10-kernel/src/desktop/fluent/)** 下的 **模块桩**（`shell.rs`、`acrylic.rs`、`mica.rs`、`dwm.rs` 等），供后续与 Win32k/DWM 内核栈对接。

**UEFI bring-up 会话**（`session.rs`）：在 GOP 线性帧缓冲上绘制壁纸、任务栏、开始菜单与桌面快捷方式；**软件鼠标指针**作为与 Win32 类似的 **最顶层合成层**（先重绘桌面场景，再 `pointer_capture_under` / `pointer_paint_on_fb`；移动时用 `pointer_remove_from_fb` 恢复像素）。客户区式像素坐标说明见 `references/win32/desktop-src/LearnWin32/mouse-movement.md`。

桌面快捷方式标签为 **中文**（此电脑、用户文档、回收站、网络），构建期用 **Source Han Serif 思源宋体**（OFL，[`third_party/fonts/cjk/`](../../third_party/fonts/cjk/)）栅格化为 BGRA；右键菜单英文标签用 **霞鹜文楷 LXGW WenKai**（OFL，楷体风格拉丁字形，[`third_party/fonts/kai/`](../../third_party/fonts/kai/)）。另含 **Libertinus Serif**（Times 类衬线，[`third_party/fonts/latin/`](../../third_party/fonts/latin/)）、**Source Han Sans**（[`third_party/fonts/cjk/`](../../third_party/fonts/cjk/)）作系统字体包。完整 [kiwi0fruit/open-fonts](https://github.com/kiwi0fruit/open-fonts) 目录可用 [`scripts/sync-open-fonts-catalog.sh`](../../scripts/sync-open-fonts-catalog.sh) 可选克隆（约 825MB，已 `.gitignore`）。

静态资源已置于仓库根 **[`resources/`](../../resources/)**，机器可读清单为 **[`resources/manifest.json`](../../resources/manifest.json)**（壁纸、多尺寸图标、`misc/` 等；原创素材说明见 [`resources/README.md`](../../resources/README.md)）。

## 6. 相关文档

- [Architecture.md](Architecture.md) §5
- [Roadmap-and-TODO.md](Roadmap-and-TODO.md)（Phase 14）
- [Build-Test-Coding.md](Build-Test-Coding.md)
- [extensions/README.md](../../extensions/README.md)（`references/win32/desktop-src` 分阶段实现 TODO 索引）
