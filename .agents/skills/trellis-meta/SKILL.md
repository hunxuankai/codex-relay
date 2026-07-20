---
name: trellis-meta
description: "理解并定制用户项目中的本地 Trellis 架构。修改 .trellis 及平台钩子、设置、代理、Skill、命令、提示词、工作流、Channel 运行时（trellis channel）、.trellis/agents/ 下的内置运行时代理、可选工作流模板、注册表驱动的规范刷新、trellis init 生成的跨会话记忆（trellis mem），或面向 AI 的内置 Skill（trellis-channel、trellis-session-insight、trellis-spec-bootstrap）及其自动派发流程时使用。"
---

# Trellis 元架构

此 Skill 面向已在项目中运行 `trellis init` 的本地 Trellis 用户。阅读后，AI 应理解该用户项目内的 Trellis 架构、运行模型和定制入口，再根据用户请求修改生成的 `.trellis/` 和平台目录文件。

Trellis v0.6 在此前的工作流/持久化/平台模型之上增加了三个架构面。第一，多代理协作运行时：`trellis channel` 通过项目范围的 JSONL 事件日志 `~/.trellis/channels/<project>/<channel>/events.jsonl` 协调多个 AI Worker 进程，并提供 Worker OOM 防护、forum/thread 频道、持久幂等键和内置 `.trellis/agents/{check,implement}.md` 运行时定义。第二，跨会话记忆：`trellis mem list | search | context | extract | projects` 读取磁盘上已有的 Claude Code、Codex 和 Pi Agent 原始 JSONL，按 `--phase brainstorm|implement|all` 切片，且从不上传内容。第三，双 npm 包发布：`@mindfoldhq/trellis`（CLI）与 `@mindfoldhq/trellis-core`（具有 `/channel`、`/task`、`/mem`、`/testing` 子路径的 SDK）按同一版本同步发布。应把这些与各平台集成文件一起视为一等定制面。

默认操作范围是用户项目中的本地文件：

- `.trellis/`：工作流、配置、任务、规范、工作区、脚本、内置运行时代理和运行时状态。
- 平台目录：`.claude/`、`.codex/`、`.cursor/`、`.opencode/`、`.kiro/`、`.gemini/`、`.qoder/`、`.codebuddy/`、`.github/`、`.factory/`、`.pi/`、`.reasonix/`、`.kilocode/`、`.agent/`、`.devin/` 及类似目录。在文件布局之上，Pi 还提供原生 `trellis_subagent` 工具，支持 `single` / `parallel` / `chain` 派发模式、限频进度卡和 `isTrellisAgent()` 验证。Reasonix 把工作流 Skill 和子代理 Skill 都存为 `.reasonix/skills/<name>/SKILL.md`；子代理 Skill 的 frontmatter 带有 `runAs: subagent`。
- 共享 Skill 层：`.agents/skills/`。
- 项目树外、用户拥有的频道存储：`~/.trellis/channels/<project>/<channel>/events.jsonl`。
- 可通过 `trellis mem` 查询的平台原始对话日志：`~/.claude/projects/`、`~/.codex/sessions/` 和 `~/.pi/agent/sessions/`（v0.6 系列中 OpenCode 适配器处于降级状态）。

不要假设用户拥有 Trellis 源码仓库。不要默认修改全局 npm 安装目录或 `node_modules`；`@mindfoldhq/trellis` 和 `@mindfoldhq/trellis-core` 都是发布包，每次发布共享同一版本和 Git tag。

## 使用方式

1. 先读取 `references/local-architecture/overview.md`，建立本地 Trellis 系统模型。
2. 如果请求涉及特定 AI 工具，读取 `references/platform-files/platform-map.md` 和相关平台文件说明。
3. 如果请求涉及多代理派发或 Channel Worker，读取 `references/local-architecture/multi-agent-channel.md` 和内置 `.trellis/agents/` 文件。
4. 如果用户想改变行为，读取 `references/customize-local/overview.md`，再打开具体定制主题。
5. 编辑前读取用户项目中的真实文件，并以本地内容为权威来源。

## 参考资料

### 本地架构

- `references/local-architecture/overview.md`：分层的本地 Trellis 架构（工作流/持久化/平台/Channel 运行时）和定制原则。
- `references/local-architecture/generated-files.md`：`trellis init` 生成的文件及其定制边界，包括 `.trellis/agents/`。
- `references/local-architecture/workflow.md`：`.trellis/workflow.md` 中的阶段、路由、workflow-state 块和可选工作流模板（`native`、`tdd`、`channel-driven-subagent-dispatch`、Marketplace）。
- `references/local-architecture/task-system.md`：任务目录、活动任务、JSONL 上下文、父/子任务树和任务运行时。
- `references/local-architecture/spec-system.md`：`.trellis/spec/` 如何组织、注入，以及如何从 `registry.spec` 来源刷新。
- `references/local-architecture/workspace-memory.md`：`.trellis/workspace/` 日志、`trellis mem` 跨会话回忆和 `@mindfoldhq/trellis-core/mem` SDK。
- `references/local-architecture/context-injection.md`：钩子、子代理前导指令和 Channel 运行时 Worker 收件箱路由。
- `references/local-architecture/multi-agent-channel.md`：`trellis channel` 子命令、项目范围事件存储、forum/thread 频道、Worker OOM 防护、持久幂等和内置 `.trellis/agents/` 运行时代理。
- `references/local-architecture/bundled-skills.md`：自动派发的内置 Skill（`trellis-meta`、`trellis-spec-bootstrap`、`trellis-session-insight`），以及 `getBundledSkillTemplates()` 如何把它们分发到每个平台的 Skill 根目录。

### 平台文件

- `references/platform-files/overview.md`：共享 `.trellis/` 文件与平台目录的关系，以及四种平台集成模式（钩子驱动、代理前导、主会话工作流、Channel 运行时）。
- `references/platform-files/platform-map.md`：所有 15 个受支持平台的 Skill、代理、钩子和扩展目录/路径，包括 Reasonix 和 Pi 原生 `trellis_subagent` 扩展。
- `references/platform-files/hooks-and-settings.md`：设置/配置文件、钩子、插件和扩展如何连接 Trellis；涵盖 `channel.worker_guard.*` 和 `codex.dispatch_mode`。
- `references/platform-files/agents.md`：各平台的 `trellis-research` / `trellis-implement` / `trellis-check` 子代理文件，以及 Channel 运行时内置的 `.trellis/agents/{check,implement}.md`。
- `references/platform-files/skills-and-commands.md`：Skill、命令、提示词和工作流之间的差异，以及修改方式。

### 本地定制

- `references/customize-local/overview.md`：为用户请求选择正确的本地定制入口。
- `references/customize-local/change-workflow.md`：修改阶段、路由、下一步、workflow-state 和所选工作流模板。
- `references/customize-local/change-task-lifecycle.md`：修改任务创建、状态、归档行为、父/子链接、归档 slug 冲突处理和生命周期钩子。
- `references/customize-local/change-context-loading.md`：修改任务、规范、日志、钩子上下文、Channel 收件箱消息和 `trellis mem` 回忆的加载方式。
- `references/customize-local/change-hooks.md`：修改平台钩子、设置、任务生命周期钩子（`hooks.after_*`）和 Shell 会话桥接。
- `references/customize-local/change-agents.md`：修改各平台子代理、内置 Channel 运行时代理和 Codex `dispatch_mode` 开关中的 research、implement、check 代理行为。
- `references/customize-local/change-skills-or-commands.md`：添加或修改本地 Skill、命令、提示词和工作流；涵盖上游内置 Skill 自动派发。
- `references/customize-local/change-spec-structure.md`：调整 `.trellis/spec/` 下的项目规范结构，包括注册表驱动来源。
- `references/customize-local/add-project-local-conventions.md`：把团队规则放入项目本地规范或本地 Skill。

## 当前规则

- `.trellis/workflow.md` 是本地工作流事实来源；其初始内容在 `trellis init` 时从工作流模板（内置 `native`、`tdd`、`channel-driven-subagent-dispatch` 或 Marketplace 模板）中选择，也可通过 `trellis workflow --template <id>` 重新选择。活动模板引用的 `.trellis/agents/<name>.md` 文件缺失时，会在 stderr 输出一条指向 `trellis update` 的非阻塞警告。
- `.trellis/config.yaml` 是项目级 Trellis 配置入口。它包含任务生命周期钩子（`hooks.after_create` / `after_start` / `after_finish` / `after_archive`）、日志形态（`session_commit_message` / `max_journal_lines` / `session_auto_commit`）、Channel Worker 防护（`channel.worker_guard.idle_timeout` / `max_live_workers`）、Codex 派发模式（`codex.dispatch_mode: inline | sub-agent`）和规范注册表块（`registry.spec.source` + `registry.spec.template`）。
- `.trellis/spec/` 存储用户项目专属的编码约定和设计约束。设置 `registry.spec` 后，文件由 `trellis update` 刷新；本地编辑会在 `.trellis/.template-hashes.json` 中显示为“用户已修改”冲突。
- `.trellis/tasks/` 存储任务 PRD、设计注记、实施计划、研究文件和 JSONL 上下文。任务形成父/子树：`task.py create --parent <slug>`、`task.py add-subtask <parent> <child>`、`task.py remove-subtask <parent> <child>` 和 `task.py list-context <task>`。如果 slug 已存在于 `.trellis/tasks/archive/**`，`task.py create` 会拒绝创建。
- `.trellis/workspace/` 存储**有意写入的**开发者日志。原始跨会话对话**不**存储在这里；它们位于磁盘上的 `~/.claude/projects/`、`~/.codex/sessions/` 和 `~/.pi/agent/sessions/`，通过 `trellis mem search|extract|context` 恢复。内置 `trellis-session-insight` Skill 说明何时使用 `mem`。
- `.trellis/agents/{check,implement}.md` 是平台无关的内置 Channel 运行时代理定义，由 `trellis channel spawn --agent <name>` 加载。它们可编辑；`trellis update` 会补回缺失文件。编辑各平台的 `trellis-implement.md` / `trellis-check.md` **不会**改变 Channel 运行时 Worker 行为。
- `~/.trellis/channels/<project>/<channel>/events.jsonl` 是每个项目、每个频道的 Channel 运行时事件日志。它归用户所有，通过文件锁分配序号，支持持久 `idempotencyKey`；绝不位于 `.trellis/` 下。
- 多文件内置 Skill（`trellis-meta`、`trellis-spec-bootstrap`、`trellis-session-insight`、`trellis-channel`）由 `packages/cli/src/templates/common/index.ts` 中的 `getBundledSkillTemplates()` 自动派发到每个平台的 Skill 根目录。在上游 `packages/cli/src/templates/common/bundled-skills/` 下新增目录，会在下一次 `trellis update` 时分发到所有平台。
- 平台设置/配置文件决定实际运行哪些钩子、代理、Skill、命令、提示词和工作流。Reasonix 没有设置文件，行为编码在 Skill frontmatter 中。
- `.trellis/.template-hashes.json` 和 `.trellis/.runtime/` 是管理/运行时状态文件。编辑前确认必要性。

## 禁止事项

- 不要把 Trellis 上游源码作为本地定制的默认目标。
- 不要通过修改全局 npm 安装目录、`node_modules/@mindfoldhq/trellis` 或 `node_modules/@mindfoldhq/trellis-core` 来实现项目需求；两个包同步发布。
- 不要用默认模板覆盖用户已修改的本地文件；先检查 `.trellis/.template-hashes.json`，优先使用 `.new` 伴随文件，不要破坏性覆盖。
- 不要把团队私有项目规则放入任何公共内置 Skill（`trellis-meta`、`trellis-spec-bootstrap`、`trellis-session-insight`、`trellis-channel`）；应放入 `.trellis/spec/`、项目本地 Skill、当前任务或工作区日志。`trellis update` 会覆盖内置 Skill 目录中的所有内容。
- 不要手动编辑 `~/.trellis/channels/<project>/<channel>/events.jsonl`；序号在文件锁下分配，可安全重放的写入应通过 `trellis channel` CLI 或 `@mindfoldhq/trellis-core/channel` SDK 完成。
- 如果目标是改变 Channel 运行时 Worker 行为，不要编辑 `.claude/agents/trellis-implement.md`（或任何其他平台子代理文件）；应编辑 `.trellis/agents/<name>.md`。
- 不要把已移除或从未发布的机制描述为当前 Trellis 行为；声称某个开关存在前，先与本地 `.trellis/config.yaml` 和已安装 CLI 的 `trellis --help` 交叉核对。
