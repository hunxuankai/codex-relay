# 实施计划：复核内部文档中文化任务完成度

## 当前状态

- 阶段：用户扩展范围后重新进入 Phase 2.1。
- 已完成：创建并激活审计任务；读取原任务材料和三笔工作提交；完成受管清单、文档残留、结构/语义、workspace 生成器、自动提交主题、长期规则、Channel 命令例外、全量项目门禁和最终审计报告。
- 待修复缺口：任务脚手架仍生成英文，且 workspace/自动提交/任务脚手架行为没有持久回归测试。已有两个失败探针证明 `_default_prd_content` 与 `_write_seed_jsonl` 因目标行为缺失而失败。
- 下一步：先添加持久回归测试并确认红色，再最小修改生成器使其变绿；随后接入 `npm run check` 并完成全量验证。

## 审计步骤

- [x] 建立原任务 8 条需求、6 条验收标准与证据命令的一一映射。
- [x] 盘点三笔工作提交、当前受管文件和生成器入口，确认范围覆盖与未提交状态。
- [x] 运行 Markdown/TOML/YAML/JSON/Python/工作流解析结构审计。
- [x] 运行排除代码与技术字面量后的英文残留扫描，并人工复核全部高置信度候选。
- [x] 在安全临时目录验证 workspace 首次生成、连续更新、旧英文标签兼容和任务 PRD 默认生成。
- [x] 验证会话日志、归档提交主题的配置、默认值、实现和行为证据。
- [x] 复核长期中文规则、入口 Skill/参考资料/工作流一致性和 Git 提交历史。
- [x] 运行 Trellis 校验、差异/密钥审计和 `npm run check`。
- [x] 编写 `audit.md`，先列发现，再给逐项矩阵、总体结论、限制和建议下一步。
- [x] 按 `trellis-check` 复核审计材料与证据，判断是否需要 `trellis-update-spec`。

## 修复步骤

- [x] 添加任务脚手架公开 CLI 回归测试，确认默认 PRD 和子代理 JSONL 中文断言先失败。
- [ ] 最小中文化 `_default_prd_content` 与 `_SEED_EXAMPLE`，保持结构、占位符和命令不变，使脚手架测试通过。
- [ ] 添加 workspace 首次生成/连续更新/旧标签兼容的持久测试。
- [ ] 添加会话与归档自动提交主题、暂存范围的 mocked Git 持久测试。
- [ ] 将 `test:trellis` 接入 `npm run check`，先跑专项测试，再跑完整门禁。
- [ ] 更新 `audit.md`、PRD 勾选和验证证据，重新执行 `trellis-check` 与规范更新判断。
- [ ] 仅提交生成器、测试接入、测试文件和当前任务材料，排除并行任务改动；随后归档并记录会话。

## 验证命令

- `git show` / `git diff-tree` / `git ls-files`：提交与受管清单。
- `rg` 加结构化扫描脚本：英文候选、生成器和旧提交主题。
- `python ./.trellis/scripts/task.py validate 07-20-audit-chinese-docs-and-commits`
- `python ./.trellis/scripts/get_context.py --mode phase` 及代表性步骤解析。
- 标准库 `tomllib`、`json`、`ast` 与项目 `parse_simple_yaml`：结构解析。
- `TemporaryDirectory` + mocked Git：生成器和自动提交边界。
- `git diff --check`、`git diff --cached --check`、高置信度密钥与敏感路径扫描。
- `npm run check`

## 风险与检查点

- 英文扫描会把平台名、命令和代码示例误报为遗漏，任何结论必须人工复核上下文。
- 生成器验证最容易误触真实目录；所有写入目标必须先证明位于系统临时目录。
- 原任务曾修正文档中的 Channel 命令，不能把有意的命令差异误判为翻译破坏。
- 当前审计任务自身会增加 `.trellis/**/*.md`，扫描时必须包含它，并区分审计材料与原任务交付物。
- 长时间检查前更新本文件；任何首次失败均保留，不用成功重试覆盖。

## 完成条件

- `audit.md` 完成且所有原需求/验收项都有证据和结论。
- 所有审计命令结果、失败、限制与路径安全说明已记录。
- 审计发现的生成器和测试缺口已修复，最终差异严格限于设计列出的本任务路径。

## 当前验证证据

- 提交与清单：翻译前受管 `.agents` Markdown 46 个，`e96edf3` 全部修改；Codex TOML 4 个，全部修改；`.trellis` Markdown 38 个中修改 10 个，其余文件由本轮全量残留扫描覆盖。三笔工作提交共涉及 70 个唯一路径，随后归档与会话提交主题均为中文。
- 文档残留与结构：审计 90 个当前 Markdown、12 个 Skill frontmatter 和 44 个相对链接，围栏、frontmatter、链接错误均为 0。14 个长英文候选全部是框架/平台名、平台 tag 或路径；12 个纯英文标题全部是 Agent、Worker、GitNexus、ABCoder、`Phase Index` 等允许保留的技术/解析边界。
- 配置与源码语义：4 个 Codex TOML 除 `description` / `developer_instructions` 外与翻译前解析结果等价，6 个自然语言字段均含中文且无高置信度英文段落。Trellis YAML 唯一语义变化是 `session_commit_message` 从 `chore: record journal` 改为 `chore: 记录会话日志`；28 个 Python 文件、4 个任务 JSON 和 12 个 Skill `name` 均通过解析或等价检查。
- 首次结构组合审计因临时脚本错误假定 YAML 存在 `git` 子表而退出 1；对照 `.trellis/config.yaml` 和公开 getter 后确认该值是顶层键，修正临时脚本后整组退出 0。该失败不是项目失败，但按证据规则保留。
- 工作流与 fenced block：Phase 1.1、2.1、3.4 入口均成功解析。57 个翻译 Markdown 中有 8 个文件、12 个 fenced block 改变；8 个是中文目录树/输出模板/阶段说明，4 个是 Channel 命令纠错。Trellis 0.6.7 帮助和已安装实现确认无 `--tag`、forum/thread/context 形态正确，并确认 `interrupt_requested` / `interrupted` 与 `kind: channel, action: title`。
- workspace 生成：`TemporaryDirectory` 内首次初始化、连续两次索引更新、旧英文 `Total Sessions` 兼容读取、日志轮换和中文会话模板全部通过。
- 自动提交：项目配置和默认 getter 均返回 `chore: 记录会话日志`；mocked Git 捕获 `chore(task): 归档 07-20-test` 与 `chore: 记录会话日志`。真实历史中的 `484eee5` / `d69feb1` 进一步证明两类自动提交已实际使用中文主题。
- 任务脚手架红色探针：`_default_prd_content` 的中文标题断言按预期失败，实际得到 `Goal / Requirements / Acceptance Criteria / Notes`；`_write_seed_jsonl` 的中文说明断言按预期失败，实际 `_example` 为完整英文说明。两者源头均在 `.trellis/scripts/common/task_store.py`，而 `da1be7a` 只修改了同文件的归档提交主题。
- 全量项目门禁：`npm run check` 退出 0；Vue 类型检查和 15 个 Vitest 文件/65 个测试通过，Rust fmt、Clippy、107 个单元测试、2 个路径安全测试和 1 个 Provider 工作流测试通过。
- 并行工作隔离：最终状态检查发现 `adaptive-trellis-task-policy` 任务在本审计期间修改 6 个工作流/Skill 文件与 `AGENTS.md`。本审计未触碰这些路径，随后改用 `d69feb1` Git 归档快照重跑 87 个 Markdown、44 个链接、长期规则和工作流解析审计，结果通过。
- Phase 3.3 规范判断：本轮发现属于现有“任务材料默认中文”和“生成内容同步生成器”规则的具体违反，未新增 API、基础设施或跨层契约。审计范围又明确禁止修改产品/规范，因此不重复扩写 spec；源头、失败证据和必需测试保留在 `audit.md`，交由后续修复任务执行。
- 持久测试红色证据：新增 `test:trellis` 后首次运行 5 个测试，workspace 与两类自动提交的 3 个测试通过；inline 任务 PRD 标题仍为 `Goal / Requirements / Acceptance Criteria / Notes`，子代理 JSONL `_example` 不含中文，2 个测试因目标行为缺失按预期失败。
