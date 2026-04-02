# Clean-room 实施原则（ZirconOSFluent）

本仓库以**公开文档**所描述的行为与 ABI（例如 MSDN/WDK 类公开说明）以及我们在 QEMU / 受控环境下的**自测**为主要依据。

## 允许

- 公开规范、ABI 与行为说明。
- 自编测试程序与 QEMU/硬件实验。
- 第三方对**已发布接口**的文字描述（注意许可证与引用方式）。

## 禁止

- 复制或改写 Windows 零售/预览二进制中的代码。
- 使用泄露源码或保密材料作为实现依据。
- 未经独立验证便把内部符号或私有布局当作权威「抄本」。

## 本树实践

- 使用 **ZirconOSFluent 自有**命名与模块边界；语义对齐公开文档，而非以逆向 dump 为主要来源。
- 若必须与外部约定一致（布局/调用约定），以文档 + 自测交叉验证，并在注释中简要标明依据类型。

另见：[Roadmap-and-TODO.md](Roadmap-and-TODO.md) 基线说明；英文版 [Clean-Room-Implementation.md](../en/Clean-Room-Implementation.md)。
