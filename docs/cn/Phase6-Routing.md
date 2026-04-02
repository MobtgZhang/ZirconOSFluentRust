# Phase 6 消息路由与会话边界（中期）

**English summary**: Ring-3 csrss will own WinSta/Desktop registration; the kernel keeps syscall entry points. This note records the intended split and bring-up fallbacks (clean-room; not Windows-internal layout).

## 1. 回退开关

| 符号 | 文件 | 含义 |
|------|------|------|
| `NT10_PHASE6_RING3_CSRSS_FALLBACK_TO_KERNEL_HOST` | `servers/smss.rs` | `true` 时 QEMU/CI 继续使用内核 `csrss_host`。 |
| `PHASE6_CSRSS_OWNS_WINSTA_IN_RING3` | `subsystems/win32/csrss_host.rs` | `false` 表示桌面对象仍由内核桩注册；将来由 csrss 接管时改为 `true` 并迁移协议。 |

## 2. ALPC 与跨地址空间拷贝

- `alpc::cross_proc::post_cross_address_space`：`target_cr3 == 0` 或与当前 CR3（x86_64）一致时，拷贝到内核 bounce；其他值返回错误直至按进程页表切换实现。
- `alpc::phase6_csrss::ZR_ALPC_CSRSS_API_PORT_UTF8`：自有端口名字节串（文档用途）；非 Windows 协议克隆。

## 3. 会话策略

- 与 `alpc::session_policy` 一致：交互会话与非交互端口校验保持；禁止随意跨会话挂接桌面（现有测试覆盖 session0-only 端口）。

## 4. 启动链（目标顺序）

1. Ring-3 smss 映像（`try_launch_ring3_smss_from_vfs` 脚手架）。
2. smss 经 ALPC 唤醒 csrss（`try_smss_alpc_start_csrss_stub` / `try_alpc_handoff_csrss_spawn_stub`）。
3. csrss 接管 WindowStation/Desktop；内核 win32k 退化为 syscall 与能力检查快速路径。

## 5. 验证

- 源码关键字：[`scripts/verify-phase6-serial-keywords.sh`](../../scripts/verify-phase6-serial-keywords.sh)。
- 实机交互验收仍以 QEMU 串口与手跑清单为准（与 Phase 5 相同原则）。
