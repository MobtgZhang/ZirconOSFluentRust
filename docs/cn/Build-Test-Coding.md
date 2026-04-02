# 构建系统、测试策略与编码规范

**English**: [../en/Build-Test-Coding.md](../en/Build-Test-Coding.md)

本文对应母版 §24、§26、§27，并说明 **本仓库当前 Cargo 工作区** 与母版「完整内核工程」目标的差异。

## 1. 构建系统

### 1.1 目标内核工程（母版 §24 — **规划**）

以下命令描述 **未来** 扩展后的预期体验（特性开关、ISO、QEMU 一键运行等），**不等于** 当前仓库已具备全部能力：

```bash
cargo build -p nt10-kernel --release --target x86_64-unknown-none
cargo xtask iso          # 规划：需 xtask 或 shell 脚本
cargo xtask run-qemu     # 规划
cargo test               # 规划：host 可测 crate 或 cfg(test) 策略
```

母版中的特性开关概念（`enable_vbs`、`enable_hyperv`、`enable_la57` 等）在 Rust 侧可通过 **Cargo features** 或 **`cfg` 标志** 落地；具体命名待与 [Cargo.toml](../../Cargo.toml) 同步。

### 1.2 本仓库 **当前** Cargo 工作区（事实）

| 项 | 说明 |
|----|------|
| 根清单 | [Cargo.toml](../../Cargo.toml)：`nt10-kernel`、`nt10-boot-uefi` |
| 工具链 | [rust-toolchain.toml](../../rust-toolchain.toml)：stable + `x86_64-unknown-none` |
| 内核库 | [crates/nt10-kernel/](../../crates/nt10-kernel/)：`#![no_std]` rlib，**不**注入链接脚本（避免与可执行 crate 的 `-T` 冲突） |
| 可引导内核 ELF | [crates/nt10-kernel-bin/](../../crates/nt10-kernel-bin/)：[build.rs](../../crates/nt10-kernel-bin/build.rs) 在 `x86_64-unknown-none` 下传入 [link/x86_64-uefi-load.ld](../../link/x86_64-uefi-load.ld)（物理 `0x0800_0000` / 128 MiB）；[link/x86_64.ld](../../link/x86_64.ld) 仅作高半核实验参考 |
| 快捷别名 | [.cargo/config.toml](../../.cargo/config.toml)：`cargo kcheck` → `check -p nt10-kernel --target x86_64-unknown-none` |
| 引导占位 | [crates/nt10-boot-uefi/](../../crates/nt10-boot-uefi/)：无 `uefi-rs` 依赖 |

**常用命令**：

```bash
rustup target add x86_64-unknown-none
cargo check --workspace
cargo check -p nt10-kernel --target x86_64-unknown-none
# 或
cargo kcheck
```

### 1.3 脚本与资源

| 脚本 | 说明 |
|------|------|
| [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh) | QEMU + OVMF；默认调用 [`scripts/pack-esp.sh`](../../scripts/pack-esp.sh) 生成含 `BOOTX64.EFI` 与 `EFI/ZirconOSFluent/NT10KRNL.BIN` 的 ESP；环境变量 `PROFILE=release` 时临时 ESP 使用 release 产物 |
| [`scripts/run-qemu-kernel.sh`](../../scripts/run-qemu-kernel.sh) | `qemu-system-x86_64 -kernel` 直跑 ELF；部分主机 QEMU 会报缺少 PVH note，可改用 UEFI 脚本冒烟 |
| [`scripts/fetch-ofl-fonts.sh`](../../scripts/fetch-ofl-fonts.sh) | 拉取 OFL **Noto Sans** 等到 `third_party/fonts/latin/`（构建 UI 栅格可选但推荐） |
| [`scripts/pack-esp.sh`](../../scripts/pack-esp.sh) | 构建 `zbm10.efi` 与扁平内核二进制并写入指定目录；`PROFILE=release` 等价于 `cargo --release` |
| [`xtask/`](../../xtask/) | `cargo run -p xtask -- build|pack-esp|qemu|qemu-kernel` 封装上述脚本与构建（`--release` 会设置 `PROFILE=release`） |
| [`scripts/generate-resource-icons.py`](../../scripts/generate-resource-icons.py) | 自 `resources/icons/_sources/` 导出多尺寸 PNG（需 `pip install pillow`） |

ISO 镜像生成仍为后续规划；**xtask** 已提供 `build` / `pack-esp` / `qemu` 入口。

**UEFI 临时 ESP 布局**（[`scripts/pack-esp.sh`](../../scripts/pack-esp.sh)）：`EFI/BOOT/BOOTX64.EFI`（ZBM10）、`EFI/ZirconOSFluent/NT10KRNL.BIN`（扁平内核）、根目录 `startup.nsh`（缓解部分 OVMF 在 QEMU `fat:` 盘上默认启动项 `Unsupported`、进入 Shell 后可自动拉起 `BOOTX64.EFI`）。可选 `EFI/ZirconOSFluent/zbm10.cfg`（如 `kernel=MYKRNL.BIN`）。OVMF：合并镜像用 QEMU `-bios`；分体 `*CODE*.fd` 与同目录 `OVMF_VARS.fd` 组成双 pflash（见 [`run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh)）。

**LoongArch UEFI**：上游 `r-efi` 支持 `loongarch64-unknown-uefi` 时，可在本工作区对 `nt10-boot-uefi` 增加对应 target 与链接脚本（与 x86_64 流程对称）。

### 1.4 字体许可与构建（Phase 14 / 桌面）

- **合规**：UI 与标题栅格使用 **SIL OFL 等开源字体**（如 Noto Sans）；**不要**在仓库中捆绑或子集化 **Segoe UI** 等微软受限字体。许可证副本见 [third_party/fonts/licenses/](../../third_party/fonts/licenses/) 与 [third_party/fonts/README.md](../../third_party/fonts/README.md)。
- **获取字体**：执行 `./scripts/fetch-ofl-fonts.sh`，或将 `NotoSans-Regular.ttf` 等放入 `third_party/fonts/latin/`（与 `nt10-kernel` `build.rs` 候选路径一致）。
- **严格模式**：设置 `NT10_KERNEL_REQUIRE_OFL_FONT=1` 时，缺少 TTF 会 **panic**（避免误发无合规字体的构建）。
- **占位构建**：未放置字体且未设上述变量时，构建使用内置 ASCII 占位栅格（功能降级；发行版仍应带 OFL 字体）。

### 1.5 UEFI 桌面会话与 USB 键鼠

长时间看不到 USB 指针时，常见原因是 xHCI 初始化很晚才执行（见 [`session.rs`](../../crates/nt10-kernel/src/desktop/fluent/session.rs) 中 `XHCI_INIT_AFTER_POLLS`）。可选手段：

- **编译期**在运行 `cargo build` / `cargo check` 前导出任意非空的 **`NT10_SKIP_XHCI`**（例如 `NT10_SKIP_XHCI=1`），`option_env!` 会启用 `SKIP_XHCI_INIT`（仅 PS/2）。
- 或直接调小源码中的 `XHCI_INIT_AFTER_POLLS`（权衡：过早 init 可能拖住主循环）。

## 2. 测试策略（母版 §26 — 目标内核）

- **单元测试**：规划置于 `tests/` 或各 crate 的 `#[cfg(test)]`；内核 `no_std` 部分需在 host 上用 mock/`std` 或独立 host crate 测算法。
- **集成测试**：QEMU 下冒烟（规划）。
- **一致性测试**：对照 NT10 文档 / ReactOS 等（规划）。
- **CI**：[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 运行 `cargo check --workspace`、`cargo kcheck` 与 UEFI 引导构建。

**本仓库现状**：以 **`cargo check` 通过** 为基线；纯逻辑可在 host 上跑单元测试（如 `cargo test -p nt10-kernel` 测 `mm::vad`）。**QEMU / 串口**里程碑（COM1 上的 `nt10-kernel: …` 日志）见 [scripts/run-qemu-x86_64.sh](../../scripts/run-qemu-x86_64.sh) 与根目录 [Makefile](../../Makefile) 的 `run-debug`。

**Phase 1（UEFI + OVMF）Ring-3 冒烟**：在串口输出中应依次看到（顺序可能紧邻）：`PML4[256] high-half 512MiB mirror + CR3 switch OK`、`UEFI user thread starting (ring3 + demand stack)`、`demand-zero #PF handled`（首次用户栈按需映射）、`user syscall smoke` 与 `syscall num 0x… return`。若 `high-half` 切换失败或 PFN 未初始化，则会退回 `skip ring3 smoke` 或 `UEFI first-user CR3 clone failed` 等提示。

**离线核对（无 QEMU）**：[`scripts/verify-phase1-serial-keywords.sh`](../../scripts/verify-phase1-serial-keywords.sh) 用 ripgrep 确认上述子串仍存在于内核源码中；CI 在 [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) 中运行该脚本。

**Phase 3（消息泵 / Win32 bring-up）串口验收**：在 UEFI + OVMF + COM1 日志中（`bringup_kernel_thread_smoke` 路径）应能看到 `Phase3 msg pump smoke begin`、`Phase3 WndProc dispatched`、`Phase3 msg pump smoke OK`。对应实现见 [`crates/nt10-kernel/src/subsystems/win32/csrss_host.rs`](../../crates/nt10-kernel/src/subsystems/win32/csrss_host.rs) 与 [`msg_dispatch.rs`](../../crates/nt10-kernel/src/subsystems/win32/msg_dispatch.rs)。

**维护者手跑（CI 不替代）**：Phase 1/3 的 [`scripts/verify-phase1-serial-keywords.sh`](../../scripts/verify-phase1-serial-keywords.sh) / [`scripts/verify-phase3-serial-keywords.sh`](../../scripts/verify-phase3-serial-keywords.sh) 仅检查源码子串；发布或合并前建议在本地执行 `./scripts/run-qemu-x86_64.sh`（或 `cargo run -p xtask -- qemu`），对照本节关键字核对 **真实串口输出**。

**Phase 3 离线核对**：[`scripts/verify-phase3-serial-keywords.sh`](../../scripts/verify-phase3-serial-keywords.sh)。

**与 content1.2「第三阶段」差异（脚注）**：当前为 bring-up 实现：`GetMessage` 空队列在 [`MsgWaitGen`](../../crates/nt10-kernel/src/ke/msg_wait.rs) 与 [`ke/sched::yield_message_wait`](../../crates/nt10-kernel/src/ke/sched.rs) 上协作让出，**不是**完整 NT 式 `KeWait` + 抢占睡眠；`SendMessage` 已由 [`send_message_kernel`](../../crates/nt10-kernel/src/subsystems/win32/msg_dispatch.rs) 覆盖同线程与跨线程单槽同步。用户态 TEB 镜像与独立 `libs/user32` crate 仍属后续工作。

**Phase 4（离屏 + 合成 + GDI bring-up）**：单元测试覆盖离屏 BGRA、[`text_bringup`](../../crates/nt10-kernel/src/subsystems/win32/text_bringup.rs) 位图字串与 [`compositor`](../../crates/nt10-kernel/src/subsystems/win32/compositor.rs) 合成；串口关键字 `Phase4 compositor smoke begin` / `Phase4 compositor smoke OK` 见 [`csrss_host.rs`](../../crates/nt10-kernel/src/subsystems/win32/csrss_host.rs)。**Ring-3 syscall 演示字节**（`mov rax, 0x102` + `syscall`）见 [`bringup_user.rs`](../../crates/nt10-kernel/src/mm/bringup_user.rs) 中 `USER_RING3_GETMESSAGE_SYSCALL_DEMO`（ZirconOS 自有寄存器约定，非 Windows 布局照抄）。

**Phase 4 离线核对**：[`scripts/verify-phase4-serial-keywords.sh`](../../scripts/verify-phase4-serial-keywords.sh)。

**Phase 4 收尾（UEFI + GOP）**：[`session_win32.rs`](../../crates/nt10-kernel/src/desktop/fluent/session_win32.rs) 在每帧将 [`composite_desktop_to_framebuffer`](../../crates/nt10-kernel/src/subsystems/win32/compositor.rs) 结果叠加到 **线性 GOP**（在 Fluent shell 绘制之后、软件指针之前）；合成使用 [`blend_src_over_bgra`](../../crates/nt10-kernel/src/subsystems/win32/window_surface.rs)；`BeginPaint`/`EndPaint` bring-up 见 [`win32_paint.rs`](../../crates/nt10-kernel/src/subsystems/win32/win32_paint.rs) 中 `BringupPaintStruct`。串口关键字示例：`nt10-phase4: GOP_COMPOSITE`、`nt10-phase4: WM_LBUTTONDOWN`（客户区点击验证）。

**Phase 5（HWND + WM_PAINT Shell）**：同一模块提供壁纸 HWND（全屏 `WIN_EX_NO_HIT_TEST`、资源壁纸经 `WM_PAINT` 写入离屏槽并与 [`CompositeDesktopFilter`](../../crates/nt10-kernel/src/subsystems/win32/compositor.rs) 底部分层一致）、任务栏 HWND（`WS_EX_TOOLWINDOW`）、桌面图标 `BitBlt`（`resources` 构建期栅格）、可拖动测试窗（`WM_NCHITTEST` / `HTCAPTION` / `HTBORDER`）、`WM_TIMER` 任务栏刷新（与 [`ke/timer.rs`](../../crates/nt10-kernel/src/ke/timer.rs) 文档路径一致）、任务栏槽与 [`WindowStack`](../../crates/nt10-kernel/src/desktop/fluent/app_host.rs) 的 **最小化/还原**、时钟弹出 HWND（`CLOCK_FLYOUT` 源码关键字）、桌面右键菜单（`WIN_EX_SHELL_POPUP` + 向壁纸 HWND `PostMessage` [`ZR_WM_MENU_COMMAND`](../../crates/nt10-kernel/src/desktop/fluent/session_win32.rs)）。手跑 QEMU COM1 建议逐项确认：壁纸与左侧快捷方式共存、任务栏分区色带与命中、可拖测试窗、右键菜单项、时钟弹窗时间与日期刷新、槽位与托管应用切换。

**Phase 5 离线核对**：[`scripts/verify-phase5-serial-keywords.sh`](../../scripts/verify-phase5-serial-keywords.sh)。

**Phase 6（ALPC / Ring-3 csrss — 中期）**：[`post_cross_address_space`](../../crates/nt10-kernel/src/alpc/cross_proc.rs) 在同 CR3 / `target_cr3==0` 下写入内核 bounce；[`resolve_imports_for_image_stub`](../../crates/nt10-kernel/src/loader/import_.rs) 对 **零 DLL 导入** 的 PE 返回真；[`ProcessAddressSpaceBringup`](../../crates/nt10-kernel/src/mm/vm.rs) 与 [`servers/smss.rs`](../../crates/nt10-kernel/src/servers/smss.rs) / [`alpc/phase6_csrss.rs`](../../crates/nt10-kernel/src/alpc/phase6_csrss.rs) / [`csrss_host.rs`](../../crates/nt10-kernel/src/subsystems/win32/csrss_host.rs) 中的回退开关为脚手架。路由与职责边界见 [`Phase6-Routing.md`](Phase6-Routing.md)。

**Phase 6 离线核对**：[`scripts/verify-phase6-serial-keywords.sh`](../../scripts/verify-phase6-serial-keywords.sh)。

**字体策略（Phase 4/5）**：内核 `no_std` 路径以 [`text_bringup`](../../crates/nt10-kernel/src/subsystems/win32/text_bringup.rs) 固定栅格为主；[`font_stub`](../../crates/nt10-kernel/src/desktop/fluent/font_stub.rs) 描述 OFL/构建期资源衔接；可选 **fontdue**、**构建期预栅格** 或 **带 `alloc` 的受限堆** 留在 Ring3/工具链阶段集成（与 `build.rs` 生成资源一致即可）。

## 3. 编码规范（母版 §27 摘要，Rust）

### 3.1 命名

| 类别 | 风格 | 示例 |
|------|------|------|
| 类型/结构体 | PascalCase | `EProcess`, `ObjectHeader` |
| 函数/模块 | snake_case | `alloc_phys_frame` |
| 常量 | SCREAMING_SNAKE | `PAGE_SIZE` |
| NT 导出 API | PascalCase + Nt 前缀 | `NtCreateFile` |
| 内部实现细节 | snake_case | `flush_tlb_internal` |

### 3.2 Rust 使用要点

- **布局敏感** 且与 NT 互操作的类型使用 `#[repr(C)]`（或明确对齐的 `repr`）。
- **导出 NT API** 的返回值与内核约定统一映射到 `NTSTATUS`（或 `i32` / newtype，团队选定后保持一致）。
- **IRQL ≥ DISPATCH_LEVEL**：不分配可分页内存、不访问可能缺页的地址；在文档与 `unsafe` 块注释中标明 IRQL 前提。
- 与用户指针交互前必须经过探测/校验路径；禁止未验证的 `unsafe` 解引用。
- HAL 等多实现点可用 **trait 对象**、**泛型参数** 或 **条件编译 `cfg(target_arch)`** 替代母版中的 comptime 多态描述。

### 3.3 安全编码

- `unsafe` 需注释 **不变量** 与 **调用方责任**。
- ISR 保存完整寄存器上下文；DPC 回调避免可分页路径（仅用 NonPagedPool 等）。

## 4. 相关文档

- [PROCESS_NT10.md](PROCESS_NT10.md)
- [Architecture.md](Architecture.md)
