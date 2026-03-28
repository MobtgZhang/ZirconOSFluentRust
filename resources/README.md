# ZirconOS NT10 桌面资源包

原创素材（AI 生成 + 脚本导出），**非** Microsoft 资产；与 Windows 默认壁纸/图标无对应关系。

- 机器可读清单：[manifest.json](manifest.json)
- 图标主图位于 `icons/_sources/`（应为带 **alpha 透明底** 的 PNG；若生成图四角为浅色衬底，脚本会尝试剔除后再**按内容裁切并居中留白**，避免缩小后显得压扁）。
- 多尺寸 PNG 由 `scripts/generate-resource-icons.py`（`pip install pillow`）生成。
