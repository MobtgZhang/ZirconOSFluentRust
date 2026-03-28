# Phase 99：DirectX / 游戏与多媒体大粒度 Backlog

## 本阶段目标

收录 [toc.yml](../references/win32/desktop-src/toc.yml) 中 **Graphics and Gaming**、**Audio and Video** 等体量巨大的分支，明确其 **非 Fluent / Win32k bring-up 前置**，避免与 Phase 10–14 混淆。

## 参考文档（仓库内路径，入口级）

- [references/win32/desktop-src/directx.md](../references/win32/desktop-src/directx.md)
- [references/win32/desktop-src/getting-started-with-directx-graphics.md](../references/win32/desktop-src/getting-started-with-directx-graphics.md)
- [references/win32/desktop-src/prog-dx-with-com.md](../references/win32/desktop-src/prog-dx-with-com.md)
- [references/win32/desktop-src/direct3dgetstarted/building-your-first-directx-app.md](../references/win32/desktop-src/direct3dgetstarted/building-your-first-directx-app.md)
- [references/win32/desktop-src/direct2d/direct2d-portal.md](../references/win32/desktop-src/direct2d/direct2d-portal.md)
- [references/win32/desktop-src/direct3d.md](../references/win32/desktop-src/direct3d.md)
- [references/win32/desktop-src/getting-started-with-direct3d.md](../references/win32/desktop-src/getting-started-with-direct3d.md)
- [references/win32/desktop-src/direct3d12/direct3d-12-graphics.md](../references/win32/desktop-src/direct3d12/direct3d-12-graphics.md)
- [references/win32/desktop-src/direct3d11/atoc-dx-graphics-direct3d-11.md](../references/win32/desktop-src/direct3d11/atoc-dx-graphics-direct3d-11.md)
- [references/win32/desktop-src/direct3dhlsl/dx-graphics-hlsl.md](../references/win32/desktop-src/direct3dhlsl/dx-graphics-hlsl.md)
- [references/win32/desktop-src/directwrite/direct-write-portal.md](../references/win32/desktop-src/directwrite/direct-write-portal.md)
- [references/win32/desktop-src/dxmath/directxmath-portal.md](../references/win32/desktop-src/dxmath/directxmath-portal.md)
- [references/win32/desktop-src/xaudio2/xaudio2-apis-portal.md](../references/win32/desktop-src/xaudio2/xaudio2-apis-portal.md)
- [references/win32/desktop-src/xinput/xinput-game-controller-apis-portal.md](../references/win32/desktop-src/xinput/xinput-game-controller-apis-portal.md)
- [references/win32/desktop-src/audio-and-video.md](../references/win32/desktop-src/audio-and-video.md)
- [references/win32/desktop-src/CoreAudio/core-audio-apis-in-windows-vista.md](../references/win32/desktop-src/CoreAudio/core-audio-apis-in-windows-vista.md)
- [references/win32/desktop-src/medfound/microsoft-media-foundation-sdk.md](../references/win32/desktop-src/medfound/microsoft-media-foundation-sdk.md)
- [references/win32/desktop-src/OpenGL/opengl.md](../references/win32/desktop-src/OpenGL/opengl.md)
- [references/win32/desktop-src/mixedreality/mixed-reality-portal.md](../references/win32/desktop-src/mixedreality/mixed-reality-portal.md)

## 实现 TODO（长期，按需启用）

- [ ] 在 WDDM/dxg 路线清晰前，**不实现** D3D12 用户态运行时；本文件仅作索引。
- [ ] 若引入 **DXGI Factory / 适配器枚举**，与 Phase 06 `dxcore` 任务合并评审。
- [ ] 将 **D2D + DirectWrite** 列为「替换 GDI 文本绘制」的候选栈，依赖字体与合成。
- [ ] **XAudio2 / XInput**：游戏音频与手柄，排在 HID 与 USB 栈稳定之后。
- [ ] **Media Foundation / DirectShow**：视频播放管线，默认超出 NT10 内核最小集。
- [ ] **OpenGL ICD**：标记为驱动模型远期，与 Vulkan 取舍单独立项（未在本仓库范围）。
- [ ] **Mixed Reality**：仅保留门户链接，无实现承诺。
- [ ] 维护一份 **「若实现 D3D12，需同步的内核对象」** 速查（command queue、fence、heap — 不展开细节）。
- [ ] 将 `direct3darticles`、`direct3dtools` 等外链归入开发者工具类，不参与内核里程碑。

## 弱相关企业/网络主题（仅 TOC 指针）

以下与当前 Win32k/Fluent 主线弱相关，**不展开 TODO**：`AD/`、`WinSock/`、`wsw/`、`HyperV_v2/`（用户态管理面）等；需要时从 [toc.yml](../references/win32/desktop-src/toc.yml) 的 **Networking**、**Security**、**Server** 节自行下钻。
