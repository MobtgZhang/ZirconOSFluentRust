# Phase 05：输入栈（指针、触摸、键盘鼠标、TSF、高 DPI）

## 本阶段目标

覆盖 **指针消息、触摸命中、注入、旧式触摸、键盘鼠标、TSF、高 DPI** 与 ZirconOSFluent 当前 UEFI/内核输入管线及 Fluent 会话的对接（Roadmap Phase 10、14）。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/user-interaction.md](../references/win32/desktop-src/user-interaction.md)
- [references/win32/desktop-src/inputmsg/messages-and-notifications-portal.md](../references/win32/desktop-src/inputmsg/messages-and-notifications-portal.md)
- [references/win32/desktop-src/input_pointerdevice/pointer-device-stack-portal.md](../references/win32/desktop-src/input_pointerdevice/pointer-device-stack-portal.md)
- [references/win32/desktop-src/input_touchhittest/touch-hit-testing-portal.md](../references/win32/desktop-src/input_touchhittest/touch-hit-testing-portal.md)
- [references/win32/desktop-src/input_touchinjection/touch-injection-portal.md](../references/win32/desktop-src/input_touchinjection/touch-injection-portal.md)
- [references/win32/desktop-src/inputdev/user-input.md](../references/win32/desktop-src/inputdev/user-input.md)
- [references/win32/desktop-src/wintouch/windows-touch-portal.md](../references/win32/desktop-src/wintouch/windows-touch-portal.md)
- [references/win32/desktop-src/input_intcontext/interaction-context-portal.md](../references/win32/desktop-src/input_intcontext/interaction-context-portal.md)
- [references/win32/desktop-src/input_feedback/input-feedback-configuration-portal.md](../references/win32/desktop-src/input_feedback/input-feedback-configuration-portal.md)
- [references/win32/desktop-src/input_sourceid/input-source-identification-portal.md](../references/win32/desktop-src/input_sourceid/input-source-identification-portal.md)
- [references/win32/desktop-src/input_radial/radialcontroller-portal.md](../references/win32/desktop-src/input_radial/radialcontroller-portal.md)
- [references/win32/desktop-src/input_ink/input-ink-portal.md](../references/win32/desktop-src/input_ink/input-ink-portal.md)
- [references/win32/desktop-src/directmanipulation/direct-manipulation-portal.md](../references/win32/desktop-src/directmanipulation/direct-manipulation-portal.md)
- [references/win32/desktop-src/tsf/text-services-framework.md](../references/win32/desktop-src/tsf/text-services-framework.md)
- [references/win32/desktop-src/hidpi/high-dpi-desktop-application-development-on-windows.md](../references/win32/desktop-src/hidpi/high-dpi-desktop-application-development-on-windows.md)
- [references/win32/desktop-src/LearnWin32/mouse-movement.md](../references/win32/desktop-src/LearnWin32/mouse-movement.md)
- [references/win32/desktop-src/LearnWin32/keyboard-input.md](../references/win32/desktop-src/LearnWin32/keyboard-input.md)
- [references/win32/desktop-src/LearnWin32/dpi-and-device-independent-pixels.md](../references/win32/desktop-src/LearnWin32/dpi-and-device-independent-pixels.md)
- [references/win32/desktop-src/legacy-user-interaction-features.md](../references/win32/desktop-src/legacy-user-interaction-features.md)

## 实现 TODO

- [ ] 将 **POINTER_* / WM_POINTER*** 系列与当前 USB 鼠标/键盘事件映射表写出（含修饰键）。
- [ ] 定义 **客户区命中测试** 顺序：从顶层窗口到子控件，与 Fluent `session.rs` 点击路径对齐。
- [ ] 评估 **触摸注入 / 指针注入** API 与安全边界（Session 0 vs 交互会话）。
- [ ] 列出 **触摸命中测试（Touch hit testing）** 与 DWM 缩略图/透明窗口的依赖（链 Phase 06）。
- [ ] 将 **Interaction context** 与惯性滚动标为可选用户态库，不阻塞内核 bring-up。
- [ ] 规划 **TSF**：输入法编辑器与 `conhost` / 控制台 UTF-8 路径的关系。
- [ ] 实现 **高 DPI 感知** 标志与每监视器 DPI V2 行为表（先文档后代码）。
- [ ] 将 **Radial / Ink / Direct Manipulation** 标为设备类扩展，列入远期 backlog。
- [ ] 统一 **屏幕坐标 / 物理像素 / DIP** 三者在 GOP 帧缓冲上的换算（与 `dpi-and-device-independent-pixels.md` 对照）。
- [ ] 为 **游戏独占全屏** 与桌面合成器输入抢占写约束说明（简化为「单模式」亦可）。
- [ ] 核对 **键盘布局**、**死键**、**Alt+数字** 与当前 keymap 桩的差距。
- [ ] 将 **指针捕获**（`SetCapture`）与右键拖拽菜单交互建模。
- [ ] 记录 **原始输入（Raw Input）** 与游戏手柄（链 Phase 99 XInput）的分流点。
- [ ] 评估 **手写笔压力/倾斜** 是否进入 HID 报告解析路径（硬件抽象层）。
- [ ] 在 `display_mgr` 与输入模块间定义 **垂直同步与输入采样** 的时序说明（减少撕裂感）。
