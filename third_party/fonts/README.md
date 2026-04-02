# 开源字体（OFL / 兼容许可）

本目录仅用于 **SIL OFL** 或与 CONTRIBUTING 一致的开源字体。**不包含** Windows 内置字体（如 Segoe UI）。

## 构建 `nt10-kernel` 所需

- 推荐：`latin/NotoSans-Regular.ttf`（Noto Sans，OFL）
- 获取方式：在仓库根目录执行 `./scripts/fetch-ofl-fonts.sh`（需 `curl` 或 `wget`）
- 亦可手动从 [noto-fonts](https://github.com/googlefonts/noto-fonts) 取得同名文件放入 `latin/`。

## 备选文件名（`build.rs` 会依次尝试）

1. `latin/NotoSans-Regular.ttf`
2. `latin/LiberationSans-Regular.ttf`
3. `latin/DejaVuSans.ttf`

若三者皆无且 **未** 设置 `NT10_KERNEL_REQUIRE_OFL_FONT=1`，构建将使用 **占位条带** 栅格（可读性弱，便于空仓库通过 CI）。

## 许可证全文

见 [licenses/](licenses/)（如 `OFL-NotoSans.txt`）。
