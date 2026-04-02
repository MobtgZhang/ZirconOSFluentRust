# Phase 02：窗口站、桌面与会话

## 本阶段目标

把 **Window station、Desktop、Session** 的内核对象语义与启动链（SMSS → CSRSS → Winlogon 等）和本仓库 `\Sessions\N` 命名空间、csrss 桩对齐（Roadmap Phase 5、9）。阅读材料仅作索引。

## 参考文档（仓库内路径）

- [references/win32/desktop-src/winstation/window-stations-and-desktops.md](../references/win32/desktop-src/winstation/window-stations-and-desktops.md)
- [references/win32/desktop-src/user-interface.md](../references/win32/desktop-src/user-interface.md)
- [references/win32/desktop-src/desktop-app-technologies.md](../references/win32/desktop-src/desktop-app-technologies.md)
- [references/win32/desktop-src/TermServ/terminal-services-portal.md](../references/win32/desktop-src/TermServ/terminal-services-portal.md)
- [references/win32/desktop-src/SbsCs/isolated-applications-and-side-by-side-assemblies-portal.md](../references/win32/desktop-src/SbsCs/isolated-applications-and-side-by-side-assemblies-portal.md)
- [references/win32/desktop-src/ProcThread/processes-and-threads.md](../references/win32/desktop-src/ProcThread/processes-and-threads.md)
- [references/win32/desktop-src/Services/services.md](../references/win32/desktop-src/Services/services.md)

## 实现 TODO：命名空间与对象模型

- [ ] 用文档术语核对 `ob/namespace.rs` 中 **Session 桶** 与 `\Sessions\0` 默认会话行为。
- [ ] 用一张图或表描述 `\Sessions\N`、会话目录、默认交互会话与当前命名空间实现的对应关系。
- [ ] 定义 ZirconOSFluent **单会话 bring-up** 下允许的简化：窗口站数量、桌面数量、是否仅一个交互桌面。
- [ ] 定义 **Winsta0 / Default** 桌面与登录桌面的对象图（即使先为单会话单桌面）。

## 实现 TODO：API 与 csrss/win32k backlog

- [ ] 将 **CreateWindowStation / OpenWindowStation / SetProcessWindowStation** 等 API 列为 csrss/win32k 侧待实现清单（名称级即可）。
- [ ] 核对 **桌面句柄** 与「剪贴板按窗口站隔离」等边界在单会话模型下的简化假设（不支持项需写明）。
- [ ] 在 `csrss_host.rs` 中为「注册子系统 / 创建默认窗口站与桌面」类消息预留枚举值与注释。
- [ ] 设计 csrss **命名端口 / ALPC** 与「窗口站/桌面创建」消息的对应关系（协议草图，原创表述）。

## 实现 TODO：启动链与文档一致

- [ ] 在 `smss.rs` 桩上区分 **文档中的阶段启动顺序** 与 **当前已实现部分**（注释或内部文档）。
- [ ] 交叉引用 [Loader-Win32k-Desktop.md](../docs/cn/Loader-Win32k-Desktop.md) 第 4 节启动链，保持单一叙述来源。

## 实现 TODO：Session 0、多会话与远程

- [ ] 明确 **服务会话（Session 0）与交互会话** 隔离策略在本项目中的裁剪范围（是否长期仅 Session 0 bring-up）。
- [ ] 对照 TermServ 门户，标注 **RDP / 多会话** 为远期，当前里程碑不启用。
- [ ] 将 **Disconnect / Connect** 会话状态机标为未实现；保留 TermServ 路径作日后查阅索引。
- [ ] 记录 **Winlogon 桌面切换**、安全注意序列（如 Ctrl+Alt+Del）所需 **安全桌面** 与当前缺失的内核原语列表。

## 实现 TODO：进程、Token 与 SxS

- [ ] 评估 **Session ID** 在 token / 进程对象中的暴露方式（与 `EProcess` 等内部字段对齐）。
- [ ] 为 **Side-by-side 清单** 与 Session 目录下设备/路径重解析预留说明（与 SbsCs、命名空间交叉）。
