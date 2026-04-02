# Phase 06：DWM、DirectComposition 与合成交换链

## 本阶段目标

对齐 **桌面窗口管理器、合成 API、交换链** 与 ZirconOSFluent 的 `desktop/fluent/dwm.rs` 占位及 WDDM 2.x 目标栈（Roadmap Phase 10、14）。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/dwm/dwm-overview.md](../references/win32/desktop-src/dwm/dwm-overview.md)
- [references/win32/desktop-src/directcomp/directcomposition-portal.md](../references/win32/desktop-src/directcomp/directcomposition-portal.md)
- [references/win32/desktop-src/comp_swapchain/comp-swapchain-portal.md](../references/win32/desktop-src/comp_swapchain/comp-swapchain-portal.md)
- [references/win32/desktop-src/LearnWin32/the-desktop-window-manager.md](../references/win32/desktop-src/LearnWin32/the-desktop-window-manager.md)
- [references/win32/desktop-src/direct3ddxgi/dx-graphics-dxgi.md](../references/win32/desktop-src/direct3ddxgi/dx-graphics-dxgi.md)
- [references/win32/desktop-src/dxcore/dxcore.md](../references/win32/desktop-src/dxcore/dxcore.md)
- [references/win32/desktop-src/graphics-and-multimedia.md](../references/win32/desktop-src/graphics-and-multimedia.md)
- [references/win32/desktop-src/classic-directx-graphics.md](../references/win32/desktop-src/classic-directx-graphics.md)

## 实现 TODO

- [ ] 用 `dwm-overview` 中的概念核对 `dwm.rs` 占位：离屏表面、合成顺序、缩略图。
- [ ] 定义 **每窗口离屏缓冲** 与当前「全桌面一帧缓冲」迁移的里程碑。
- [ ] 列出 **DwmExtendFrameIntoClientArea**、**Blur behind**、**Acrylic/Mica** 与 Fluent `acrylic.rs` / `mica.rs` 的对应关系。
- [ ] 评估 **DirectComposition** 作为用户态合成树与内核 `dxgkrnl` 边界的 IPC 设计草图。
- [ ] 将 **composition swapchain** 文档与 DXGI flip model、VSync 对齐到 WDDM 占位模块注释。
- [ ] 标注 **无 DWM 时的回退路径**（经典主题 / XOR 绘制）是否支持（默认不支持以减负）。
- [ ] 规划 **窗口截图/缩略图**（`DwmRegisterThumbnail`）所需内核对象与权限。
- [ ] 将 **玻璃效果与 HDR** 交互标为远期（颜色空间链到 Phase 04 WCS）。
- [ ] 记录 **合成失败时的黑名单**（驱动问题）在 ZirconOSFluent 中的简化策略。
- [ ] 交叉引用 [Loader-Win32k-Desktop.md](../docs/cn/Loader-Win32k-Desktop.md) 第 2 节 Win32k/WDDM 图。
- [ ] 为 **多平面叠加（MPO）** 与任务栏全屏视频场景写「非初始范围」说明。
- [ ] 将 **DXCore 适配器枚举** 与 QEMU GOP / 未来 virtio-gpu 设备节点挂钩。
- [ ] 明确 **DWM 与 GDI 共存**（互操作位图）在本项目中的优先级（低）。
- [ ] 增加 **合成调试覆盖层**（显示脏矩形/FPS）作为可选内核编译选项。
