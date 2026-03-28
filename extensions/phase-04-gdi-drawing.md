# Phase 04：GDI 与 GDI+

## 本阶段目标

梳理 **设备上下文、位图、区域、绘制原语** 的 Win32 语义，指导 `gdi32` 桩与未来 Win32k 真实现的分层（Roadmap Phase 10）。阅读材料仅作索引。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/gdi/windows-gdi.md](../references/win32/desktop-src/gdi/windows-gdi.md)
- [references/win32/desktop-src/gdiplus/-gdiplus-gdi-start.md](../references/win32/desktop-src/gdiplus/-gdiplus-gdi-start.md)
- [references/win32/desktop-src/LearnWin32/painting-the-window.md](../references/win32/desktop-src/LearnWin32/painting-the-window.md)
- [references/win32/desktop-src/LearnWin32/simple-drawing-sample.md](../references/win32/desktop-src/LearnWin32/simple-drawing-sample.md)
- [references/win32/desktop-src/monitor/monitor-configuration.md](../references/win32/desktop-src/monitor/monitor-configuration.md)
- [references/win32/desktop-src/wcs/windows-color-system.md](../references/win32/desktop-src/wcs/windows-color-system.md)
- [references/win32/desktop-src/wic/-wic-lh.md](../references/win32/desktop-src/wic/-wic-lh.md)
- [references/win32/desktop-src/graphics-and-multimedia.md](../references/win32/desktop-src/graphics-and-multimedia.md)

## 实现 TODO：设备上下文与坐标

- [ ] 定义 **HDC** 生命周期：屏幕 DC、内存 DC、兼容 DC 与 HWND、位图句柄的关系。
- [ ] 将 **视口/窗口原点与变换** 与线性帧缓冲的 x/y 映射写成简短公式级说明（原创，不抄 MSDN 段落）。
- [ ] 将 **GDI 坐标系** 与 framebuffer 线性映射文档化。

## 实现 TODO：绘制原语与缓冲

- [ ] 在 `gdi32.rs` 现有 `fill_rect_bgra` 之外，为 **位块传送、图案填充、矩形边框** 等排序实现优先级（按「窗口最小可视」）。
- [ ] 评估 **双缓冲**：内存 DC + 位图与当前 Fluent 全屏重绘的性能与撕裂取舍。

## 实现 TODO：位图、图标与元文件

- [ ] 规划 **DIB / DIB section** 与 **图标/cursor 资源** 解码路径（与构建期 ICO、可选 WIC 阅读材料的关系）。
- [ ] 定义 **HRGN** 与点击测试、脏矩形合并的潜在用途（窗口裁剪）。
- [ ] 记录 **元文件（EMF）** 是否纳入范围（默认否）；**打印 DC** 与显示 DC 区分，避免与 `printdocs` 混淆。

## 实现 TODO：显示、颜色与字体

- [ ] 对照 monitor 文档，列出 **多显示器**、**VidPN** 与 `display_mgr` 的依赖顺序（文档级）。
- [ ] 将 **颜色管理（WCS）**、**HDR** 标为远期，记录与 ICC 等差距（标题级）。
- [ ] 核对 **字体**：当前构建期栅格化与将来 GDI/Uniscribe/DirectWrite 链路的差距列表（条目级）。

## 实现 TODO：与其它阶段边界

- [ ] 标注 **GDI+ 与 Direct2D** 为上层可选栈；内核优先 GDI 兼容子集或走合成（见 [phase-06-dwm-composition.md](./phase-06-dwm-composition.md)）。
- [ ] 与 **WDDM 占位**（`drivers/video/wddm2`）对齐：GDI 软件路径 vs 将来硬件加速的边界说明。
- [ ] 为 **OpenGL ICD** 与 GDI 互操作标注「非 NT10 bring-up 路径」。
