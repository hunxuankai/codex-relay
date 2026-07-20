# 实施计划：统一内部文档与提交信息中文规范

## 当前状态

- 阶段：Phase 3.5 收尾中。
- 已完成：创建并激活任务；盘点受管文件；完成 PRD/设计/实施计划自检；加载工作流、文档、测试、安全、数据保留、Trellis 本地架构、Codex 平台文件和任务生命周期规范；确认 `.agents` 中的 Bundled Skill 会被 `trellis update` 识别为本地定制，且不得修改模板哈希；完成 `.agents` 全部 46 个受管 Markdown 的人工翻译与分批结构验证。
- 当前工作：Phase 2.2、Phase 3.3 和 Phase 3.4 已完成；恢复会话后已取得新鲜全量验证与专项审计证据，正在执行 `trellis-finish-work`。
- 下一步：归档当前任务，再以 Phase 3.4 的三笔工作提交记录本次会话。

## 已完成

- 受管清单：`.agents` Markdown 46 个，`.trellis` Markdown 41 个，`.codex` TOML 4 个。
- 基线规模：上述 Markdown 共 7,952 行；英文自然语言特征扫描命中 3,278 行。
- 已全局搜索旧自动提交主题，确认会话日志默认值位于 `.trellis/config.yaml` 与 `.trellis/scripts/common/config.py`，归档主题位于 `.trellis/scripts/common/task_store.py`，相关工作流/Skill 文档也需同步。

## 关键决策

- 文档迁移使用“失败预演 → 分批翻译 → 结构/引用校验”，不为纯文档行为虚构单元测试。
- 每批只翻译解释性自然语言；命令、代码块、路径、配置键、占位符、协议字面量和 frontmatter `name` 保持不变。
- 自动提交主题作为独立运行时代码行为，后续严格执行一个失败测试到最小实现的红/绿循环。
- Channel 参考资料中存在与本机 Trellis 0.6.7 帮助和已安装实现矛盾的旧命令/事件名；翻译时同步删除无效 `--tag`、修正 forum/thread/context 命令，并将中断与标题事件改为真实的 `interrupt_requested` / `interrupted` 和 `kind: "channel", action: "title"`。
- Workspace Markdown 中的标题和区块由 `.trellis/scripts/common/developer.py` 与 `.trellis/scripts/add_session.py` 生成；仅翻译现有文件会在下次初始化/记录会话时回退英文，因此同步修改生成器，并兼容读取旧英文与新中文会话计数字段。
- 当前仓库不存在 `packages/cli/src/templates/trellis/scripts/` 上游模板镜像；本任务只修改用户项目内的本地 Trellis 文件，不修改全局 npm 包或模板哈希。

## 实施步骤

- [x] 加载 Phase 1.4 与 Phase 2.1 细则，执行 `task.py start`，再使用 `trellis-before-dev` 加载相关规范。
- [x] 建立受管文档清单，排除运行时缓存、生成哈希和可执行源码中的非文档内容。
- [x] 翻译 `.agents` 下所有 Skill 与参考 Markdown，保持 frontmatter、命令、路径和约束语义。
- [x] 翻译 `.trellis` 下核心工作流、Agent 指令、思考指南、工作区模板及其他残留英文 Markdown。
- [x] 翻译 `.codex/agents/*.toml` 的自然语言提示词和相关配置注释，验证 TOML 可解析。
- [x] 更新 `AGENTS.md` 与 `.trellis/spec/workflow/documentation.md`，固化文档与 Git 提交信息中文规则。
- [x] 先增加或调整针对性测试，再修改 Trellis 自动提交主题的配置默认值与归档实现。
- [x] 运行英文残留扫描并逐条区分必须保留的标识符/技术名词与遗漏翻译。
- [x] 运行 Markdown/配置语法检查、Python 针对性测试和 Trellis 完整检查。
- [x] 将每批进度、验证命令、真实结果、限制和下一步持续写回本文件。

## 验证计划

- `git diff --check`
- 使用 PowerShell/Python 解析 `.codex/*.toml`、`.trellis/config.yaml` 与相关 JSON，确认结构有效。
- 针对 Trellis 自动提交信息运行现有测试；若无覆盖，补充最小回归测试。
- 使用 `rg` 扫描文档中的整句英文和旧提交主题，人工复核保留项。
- 按 `trellis-check` 执行 spec 合规、测试、差异和安全检查。

## 风险文件与检查点

- `.trellis/workflow.md`：核心工作流，翻译后必须检查阶段编号、条件块和命令未变。
- `.agents/skills/**/SKILL.md`：执行指令，必须保持约束强度和路由语义。
- `.codex/agents/*.toml`：多行提示词与 TOML 语法容易因引号误改而失效。
- `.trellis/scripts/common/task_store.py`、`.trellis/scripts/common/config.py`：影响自动提交主题，需要测试证据。
- `AGENTS.md`：全仓库规则，新增条款必须简洁且不与现有第 8 条冲突。

## 验证证据

- 红色基线：统计命令发现 7,952 行受管 Markdown 中有 3,278 行命中 `[A-Za-z]{4,}\\s+[A-Za-z]{4,}`，证明英文文档行为尚未满足。
- 搜索证据：`rg -n --hidden ... 'record journal|archive ...|commit_message|auto_commit'` 找到会话日志与归档提交主题的配置、默认值、实现和说明文档位置。
- 离线翻译预演：本机 Argos `en -> zh` 模型在 `.agents/skills/trellis-channel/references/workflows.md` 试运行时产生术语误译、异常格式文本和占位符损坏；已在本轮完整回退该文件，未将该方案扩大到其他文档。
- 人工翻译批次验证：`.agents/skills/trellis-meta/references/customize-local/{overview,change-agents}.md` 与 `platform-files/{overview,agents,skills-and-commands}.md` 已完成翻译；针对这 5 个文件运行 `git diff --check` 退出码为 0，英文特征扫描剩余命中均为平台名、命令、路径、frontmatter 或保留技术词。
- 平台文件续批验证：`platform-files/{hooks-and-settings,platform-map}.md` 已完成人工翻译；逐文件运行 `git diff --check` 退出码为 0，保留的英文命中为平台/事件/工具名称、路径、CLI 字面量和代码注释中的技术标识。
- 工作流定制批次验证：`customize-local/{change-hooks,change-workflow}.md` 已完成人工翻译；运行 `git diff --check` 退出码为 0，英文扫描仅保留 hook/Skill/Phase 名称、状态字面量和按设计不改动的 fenced code block 示例。
- 本地架构批次验证：`local-architecture/{context-injection,multi-agent-channel}.md` 已完成人工翻译；运行 `git diff --check` 退出码为 0，英文扫描命中仅为命令、平台/Agent/Worker/channel 术语、环境变量和按设计保留的 JSONL 示例。
- `.agents` 最终批次验证：剩余 11 个 `customize-local`、`local-architecture/bundled-skills.md` 与 `trellis-channel/references` 文档已完成人工翻译；`git diff --check -- <11 files>` 退出码为 0，46/46 个受管 Markdown 均产生任务差异，所有 Markdown 围栏成对。逐块对比 fenced code block，仅 4 处为有意纠正的无效 0.6.0 示例，其余代码块与 `HEAD` 完全一致。
- Channel 纠错证据：本轮运行 `trellis --version` 得到 `0.6.7`；`trellis channel {send,interrupt,wait,messages,forum,thread,context} --help` 均退出 0，确认不存在 `--tag`，并确认 forum/thread/context 命令形态。进一步只读检查全局安装包实现，确认中断请求写入 `interrupt_requested`、Supervisor 写入 `interrupted`，标题写入 `kind: "channel", action: "title"`。
- `.trellis` 文档批次：盘点 41 个 Markdown；翻译英文集中的 `.trellis/workflow.md`、3 个 guides、2 个 Channel 运行时 Agent、2 个 workspace 索引和现有日志，逐行复核其余低命中文件仅保留技术名词、协议字面量与历史提交原文。`get_context.py --mode phase` 和 `--mode phase --step 2.1 --platform codex` 均退出 0 并输出中文，证明 `## Phase Index`、`## Phase 1: Plan`、步骤编号和平台过滤仍可解析。
- Workspace 生成器失败预演：首次临时测试因夹具未创建 `.trellis/` 根目录而在目标断言前失败，不计为红色证据；修正夹具后，测试在日志首行仍为 `# Journal - ...` 处按预期失败。修改生成器后，同一临时目录断言退出 0；独立中文索引续写测试也退出 0，确认会话总数从 0 正确更新为 1，未触碰真实用户目录。
- 自动提交主题红绿证据：临时/mocked Git 边界测试先在默认值 `chore: record journal` 处按预期失败；修改 `.trellis/config.yaml`、`common/config.py` 与 `common/task_store.py` 后重跑退出 0，并捕获归档主题 `chore(task): 归档 07-20-test`。会话默认主题为 `chore: 记录会话日志`。
- 配置与提示词解析：标准库 `tomllib` 成功解析 `.codex/config.toml` 和 3 个 `.codex/agents/*.toml`；项目配置解析器确认 `session_commit_message` 为中文、`codex.dispatch_mode` 仍为 `inline`，相关命令均退出 0。
- Trellis 任务校验：`python ./.trellis/scripts/task.py validate 07-20-chinese-docs-and-commits` 退出 0；inline 模式下缺少 `implement.jsonl` / `check.jsonl` 按设计跳过。
- Markdown 全范围结构审计：检查 `.agents` / `.trellis` 下 87 个受管及当前任务 Markdown，12 个 Skill frontmatter 的 `name` 与 `HEAD` 一致，44 个相对链接目标存在，所有 fenced code block 和 inline code 定界符闭合。唯一内部锚点仍由保留的显式 HTML `id` 提供。
- 英文残留审计：排除 fenced code、inline code、URL 和链接目标后，高置信度英文整句扫描仅命中保留的兼容锚点；放宽扫描仅额外命中 `.codex/config.toml` 的示例绝对路径。16 个纯英文标题逐条复核为 `Phase` 解析器边界或 Agent、Worker、Forum Channel、GitNexus 等技术名称。
- fenced code 差异审计：57 个已修改 Markdown 中共有 12 个代码块与 `HEAD` 不同；4 个是经 Trellis 0.6.7 帮助验证的 Channel 命令纠错，其余是人类可读的目录树注释、Agent 输出模板、工作流说明和 workspace 模板中文化，未发现命令、路径或协议字面量误改。
- 配置/源码结构审计：4 个 Codex TOML 均可解析，且删除 `description` / `developer_instructions` 后与 `HEAD` 语义相同；Trellis YAML 除 `session_commit_message` 外的解析结果与 `HEAD` 相同；4 个已改 Python 文件均通过 `ast.parse`，任务 JSON 可解析。首次组合审计错误地导入不存在的公开函数 `load_config` 而退出 1；检查模块真实接口后改用 `parse_simple_yaml` 和公开 getter，重跑退出 0。该首次失败属于审计脚本假设错误，不是项目失败。
- 检查修复：一致性搜索发现 `.agents/skills/trellis-channel/SKILL.md` 仍继承旧源文档对 `--tag` 的错误说明；本轮 `trellis channel send --help` 证明当前 CLI 没有该参数，已将规则改为等待 `done` / `turn_finished` 并使用专用 `interrupt` 命令。另修复 `.agents/skills/trellis-update-spec/SKILL.md` 三处既有的双反引号开头/单反引号结尾错误，随后 inline code 定界符审计通过。
- 临时路径专项回归：在 `TemporaryDirectory` 中验证新建中文 workspace、连续两次更新中文索引、旧英文 `Total Sessions` 兼容读取、新 journal 轮换与中文会话正文；所有断言退出 0。Mock Git 边界分别捕获 `chore: 记录会话日志` 和 `chore(task): 归档 07-20-test`，未触碰真实用户配置或应用数据目录。
- 项目检查：`npm run typecheck` 退出 0；独立 `npm run test` 运行 15 个 Vitest 文件、65 个测试，全部通过。最终 `npm run check` 退出 0，再次通过 typecheck 与 65 个前端测试，并通过 Rust fmt、Clippy（`-D warnings`）、107 个单元测试、2 个路径安全集成测试和 1 个 Provider 工作流集成测试。
- 差异检查：`git diff --check` 与 `git diff --cached --check` 均退出 0；`git status --short --ignored` 只显示本任务记录的 66 个已跟踪改动、当前任务目录，以及既有忽略目录。`.trellis/.template-hashes.json` 和 `.trellis/.runtime/` 没有任务差异，未发现额外生成文件进入改动集。
- 密钥与敏感文件审计：首次规则把 `auth_service.rs` 中 `openai_api_key: Some(api_key.to_owned())` 的字段赋值误判为环境变量密钥；检查数据来源后确认没有字面量凭据，并将审计规则收紧为高置信度密钥格式。重跑扫描 241 个受管/未跟踪文本文件，无 OpenAI/AWS Key、Bearer/Authorization 值或私钥头命中；`git ls-files` 未发现 `auth.json`、`providers.json`、`.env`、备份或 `.trellis/.runtime`。
- Phase 3.3 规范更新判断：本任务新增的中文默认语言与 Git 提交约定已写入 `AGENTS.md` 和 `.trellis/spec/workflow/documentation.md`。质量检查又发现生成式 Markdown、解析器精确边界和入口 Skill/参考资料一致性属于可复用易错点，因此在同一规范中补充生成器同步、旧标签兼容、命令证据和跨文档搜索规则；一次性命令输出仍只保留在本任务材料中。
- 最终结构审计首次把当前 `task.json` 没有末尾换行报告为失败；对比已归档任务并读取 Trellis `write_json` 唯一写入实现后，确认这是项目既有 JSON 格式而非本任务缺陷。审计保留 JSON 既有格式，仅对 Markdown/TOML/YAML/Python 强制末尾换行，重跑后覆盖 70 个脏路径、88 个 Markdown、12 个 Skill frontmatter 和 44 个相对链接并退出 0。

## 尚未解决的问题

- 无阻塞问题。保留的英文均为命令、路径、协议/配置字面量、解析器精确边界、历史提交原文或没有通行中文译名的技术名称。
- 本任务未修改发布或安装配置，因此未运行构建、签名、安装、升级、卸载或人工 UI 检查，也不对这些行为作成功声明。

## 提交记录

- `e96edf3` `docs(trellis): 统一内部文档与提示词为简体中文`：提交 61 个文档、Skill、Agent、提示词、规范和 workspace 路径。
- `da1be7a` `feat(trellis): 中文化工作区生成与自动提交主题`：提交 workspace 生成器、配置默认值和归档提交主题实现。
- `7b391ea` `docs(task): 记录内部文档中文化验证证据`：提交最终验证证据、验收勾选和收尾状态。

## 续接收尾验证证据

- 恢复检查：`get_context.py` 确认当前任务仍为 `in_progress`，`git status` 初始为干净；Phase 3.4 细则规定干净工作树直接进入 Phase 3.5。
- 新鲜全量门禁：`npm run check` 退出 0；Vue 类型检查通过，Vitest 15 个文件、65 个测试全部通过；Rust fmt 与 Clippy（`-D warnings`）通过，107 个单元测试、2 个路径安全集成测试和 1 个 Provider 工作流集成测试全部通过。
- 专项目首次组合命令因 PowerShell 不按 Bash 规则处理 `\"`，使一行 Python 在解析 f-string 时触发 `SyntaxError`；检查真实模块接口还发现命令误用了不存在的 `get_config`。该失败属于临时审计命令错误，不是项目失败；改用单引号参数和公开 `get_session_commit_message` / `get_codex_dispatch_mode` 后重跑退出 0。
- Trellis 与配置：`task.py validate 07-20-chinese-docs-and-commits` 退出 0，inline 模式按设计跳过不存在的 JSONL；标准库 `tomllib` 成功解析 4 个 Codex TOML，配置 getter 返回 `chore: 记录会话日志` 和 `inline`。
- 文档与提交主题：结构审计覆盖 87 个受管 Markdown 和 12 个 Skill frontmatter，围栏与 frontmatter 错误均为 0；高置信度英文扫描的 13 个候选全部是 `Vitest/Vue Test Utils` 名称或工作流平台 tag。Codex TOML 的高置信度英文候选为 0；运行路径中旧英文提交主题为 0，新中文默认值、配置值和归档实现均被搜索命中。
- 安全与差异：受管敏感路径和高置信度密钥文件命中均为 0；`git diff --check`、`git diff --cached --check` 退出 0。`git status --short --ignored` 仅列出项目既有忽略目录和生成缓存，没有新增未跟踪任务文件。
