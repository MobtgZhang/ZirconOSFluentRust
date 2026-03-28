# Phase 00：参考源索引与维护约定

## 本阶段目标

约定如何从 [references/win32/desktop-src/toc.yml](../references/win32/desktop-src/toc.yml) 导航到本仓库内的 Markdown，并说明与 `toc.yml` 字面路径不一致时的处理，避免后续链接失效；同时约定 **extensions 正文写作方式**，避免侵犯第三方文档版权。

## 版权与写作约定

- **references/win32/desktop-src** 下的内容为第三方文档镜像，著作权与许可以原权利人与你方获取镜像时的条款为准。
- **extensions 目录**中的说明应以 **本项目原创总结** 为主：可列出镜像内 **文件路径** 作为索引，用中文自写要点；**勿**大段复制微软文档原文或官方示例代码。
- 需要引用术语时，使用一般技术描述或短词组即可，避免整段摘录。

## 从 toc.yml 抽取 href 的规则

1. **仅处理本仓库内文件**：`href` 以 `./` 开头且以 `.md` 结尾的，映射为  
   `references/win32/desktop-src/<去掉 ./ 后的路径>`。
2. **外部 URL**（`https://`、`/windows/`、`/previous-versions/` 等）：镜像中可能不存在对应文件；extensions 中可保留为「官方在线补充」，不强制本地存在。
3. **缺省 `./` 前缀**：个别条目写作 `windows-portable-devices.md` 或 `wsdapi/wsd-portal.md`，应补全为相对根目录的路径再拼接。

## 路径归一化（toc 与磁盘大小写）

以下在 `toc.yml` 中为小写或缩写，本仓库目录名为 **实际大小写**（Linux 区分大小写）：

| toc.yml 片段 | 仓库内实际目录/文件 |
|--------------|---------------------|
| `./learnwin32/...` | `LearnWin32/...` |
| `./controls/...` | `Controls/...` |
| `./sbscs/...` | `SbsCs/...`（如 `isolated-applications-and-side-by-side-assemblies-portal.md`） |
| `./appuistart/...` | `AppUIStart/...`（若存在） |

新增链接时：**以 `find` 或 IDE 文件树为准**，不要仅照抄 `toc.yml` 的大小写。发现新的系统性不一致时，**在本表追加一行**。

## 链接与增量维护约定

- 每个 [phase-*.md](./) 文件只维护 **入口页 / portal / overview** 级链接；API 细节通过入口页的 TOC 跳转。
- extensions 内链接统一使用 **仓库相对路径**（指向 `references/...`），避免将在线短链作为主引用。
- 实现某一 TODO 后：改为 `- [x]`，或在条末标注 `(done: 路径或 commit)`；避免与 [docs/cn/Roadmap-and-TODO.md](../docs/cn/Roadmap-and-TODO.md) 重复长篇叙述。

## 实现 TODO：流程与质量

- [ ] 每条新加的「参考文档」链接在本地做一次存在性检查（`test -f` 或 IDE），注意大小写。
- [ ] 维护「toc 片段 → 实际目录名」对照表；发现新不一致时在上一节表格追加一行。
- [ ] 在 [README.md](./README.md) 或本文件中固定「完成标注」格式，全目录统一执行。

## 实现 TODO：可选自动化

- [ ] （可选）实现 `scripts/gen-extensions-index.sh`：解析 `toc.yml` 中 `href: ./**/*.md`，生成仅含路径列表的附录（附录正文为你方索引，不嵌入 MSDN 段落）。
- [ ] 决定附录文件名（如 `extensions/REFERENCE-INDEX.auto.md`）是否纳入 Git，或由 CI 生成；在 README 中说明更新方式。
- [ ] 若脚本生成文件不入库，在 `.gitignore` 中排除并在文档中注明。
