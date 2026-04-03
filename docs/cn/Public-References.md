# 实现用公开参考（短表）

仅作行为或结构提示；以原创实现为主，勿将厂商受版权保护的长文或示例代码粘贴进本仓库。总则见 [Clean-Room-Implementation.md](Clean-Room-Implementation.md)、[References-Policy.md](References-Policy.md)。

| 主题 | 建议的公开依据 |
|------|----------------|
| UEFI 内存类型 / handoff | UEFI 规范（描述符类型等）。 |
| PE/COFF / DLL | 公开 PE 格式说明、必要时 ECMA-335。 |
| x86_64 分页 / MSR | Intel SDM（分页、`SYSCALL`、`#PF` 错误码位）。 |
| VirtIO blk / MMIO | VirtIO 1.x 规范。 |
| FAT32 | 公开 BPB/FAT 字段语义。 |
| Win32 API 名（可选兼容面） | Microsoft Learn 的签名与已文档化行为。 |
| NT x64 **syscall 编号**（按系统构建分行） | 第三方**事实表**（如 j00ru [windows-syscalls](https://github.com/j00ru/windows-syscalls)）；注明构建键（如 Windows 10 22H2），勿向源码大段粘贴表体。 |
| x86_64 分页 / `#PF` | Intel SDM（四级分页、页故障错误码、`INVLPG`、NX）；AMD APM 交叉核对。 |
| PE 节区与特征 | 公开 PE/COFF 说明（节区标志、NX 兼容）。 |
| MM 架构（本项目） | [MM-Goals-and-Invariants.md](MM-Goals-and-Invariants.md) — 仅 ZirconOSFluent 命名。 |

英文版：[../en/Public-References.md](../en/Public-References.md)
