# Phase 03：窗口、消息、对话框、菜单与控件

## 本阶段目标

覆盖 **窗口类、消息分发、对话框与菜单、通用控件** 的用户态语义，驱动 `user32` 桩扩展与 Fluent bring-up 事件模型统一（Roadmap Phase 9–10）。阅读材料仅作索引。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/winmsg/windowing.md](../references/win32/desktop-src/winmsg/windowing.md)
- [references/win32/desktop-src/dlgbox/dialog-boxes.md](../references/win32/desktop-src/dlgbox/dialog-boxes.md)
- [references/win32/desktop-src/menurc/resources.md](../references/win32/desktop-src/menurc/resources.md)
- [references/win32/desktop-src/Controls/window-controls.md](../references/win32/desktop-src/Controls/window-controls.md)
- [references/win32/desktop-src/windows-application-ui-development.md](../references/win32/desktop-src/windows-application-ui-development.md)
- [references/win32/desktop-src/AppUIStart/getting-started-developing-user-interfaces-portal.md](../references/win32/desktop-src/AppUIStart/getting-started-developing-user-interfaces-portal.md)
- [references/win32/desktop-src/LearnWin32/creating-a-window.md](../references/win32/desktop-src/LearnWin32/creating-a-window.md)
- [references/win32/desktop-src/LearnWin32/closing-the-window.md](../references/win32/desktop-src/LearnWin32/closing-the-window.md)
- [references/win32/desktop-src/LearnWin32/accelerator-tables.md](../references/win32/desktop-src/LearnWin32/accelerator-tables.md)
- [references/win32/desktop-src/LearnWin32/the-desktop-window-manager.md](../references/win32/desktop-src/LearnWin32/the-desktop-window-manager.md)
- [references/win32/desktop-src/windowsribbon/-uiplat-windowsribbon-entry.md](../references/win32/desktop-src/windowsribbon/-uiplat-windowsribbon-entry.md)
- [references/win32/desktop-src/uianimation/-main-portal.md](../references/win32/desktop-src/uianimation/-main-portal.md)

## 实现 TODO：句柄与内部表示

- [ ] 建立 **HWND** 规范：用户可见句柄、内核侧窗口对象 ID、与 `user32.rs` 当前队列模型的映射。
- [ ] 定义窗口类（类名/atom、实例、额外实例字节等）的 **最小数据模型**。

## 实现 TODO：消息与默认过程

- [ ] 列出 **必备窗口消息**（如创建、销毁、绘制、关闭、退出、尺寸变化等）及 **默认窗口过程** 应完成的最小行为表。
- [ ] 实现或桩化 **RegisterClassEx / CreateWindowEx** 最小闭环（可先单窗口）。
- [ ] 将 **定时器消息** 与内核 tick 或用户态 wait 的建模方式文档化。

## 实现 TODO：窗口关系、顺序与坐标

- [ ] 将 **Z-order**、**父子窗口**、**owner/owned** 与 Fluent 层任务栏、上下文菜单等叠放规则对齐。
- [ ] 统一 **客户区坐标与屏幕坐标** 的转换约定（与 [LearnWin32/mouse-movement.md](../references/win32/desktop-src/LearnWin32/mouse-movement.md) 及 Phase 05 DPI 衔接）。

## 实现 TODO：对话框、菜单与资源

- [ ] 设计 **模态对话框** 消息泵与主循环的互斥关系（对照 dlgbox，自写要点）。
- [ ] 将资源节中 **菜单、对话框模板** 与 PE `.rsrc` 解析挂钩（链到 loader 与 `menurc`）。
- [ ] 为 **快捷键表**（accelerator）预留 `TranslateAccelerator` 路径；字符串表优先级在最小程序中排序。

## 实现 TODO：控件与 Shell 相关 UI

- [ ] 对照 `Controls` 文档，列出 **ListView / Edit / Button** 等与 Shell 文件管理相关的优先控件。
- [ ] 在 `explorer_view` 中标注未来 **列表/选中/焦点** 与 Win32 控件语义的对应关系。
- [ ] 评估 **通用控件 v6** manifest 依赖与 `SbsCs` 激活上下文（远期）。

## 实现 TODO：线程模型与低优先级 UI 技术

- [ ] 文档化 **GUI 线程亲和** 与当前内核线程模型限制。
- [ ] 为 **跨线程 SendMessage** 死锁风险写内部注意事项（单线程 bring-up 可暂缓）。
- [ ] **DPI 感知窗口** 创建标志与 [phase-05-input-stack.md](./phase-05-input-stack.md) 交叉引用。
- [ ] **Ribbon / UIAnimation** 标为低优先级，仅保留参考链接。
