# 本地 Trellis 架构概览

`trellis-meta` 面向已经运行 `trellis init` 的用户项目。用户机器通常只有通过 npm 安装的 `trellis` 命令和项目内生成的 Trellis 文件，不一定拥有 Trellis CLI 源码。

因此，AI 使用此 Skill 时，默认定制目标是用户项目中的本地文件：

- `.trellis/`：工作流、任务、规范、记忆、脚本和运行时状态。
- 平台目录：`.claude/`、`.codex/`、`.cursor/`、`.opencode/`、`.kiro/`、`.gemini/`、`.qoder/`、`.codebuddy/`、`.github/`、`.factory/`、`.pi/`、`.kilocode/`、`.agent/`、`.devin/`、`.reasonix/`、`.zcode/` 及类似目录。
- 共享 Skill 层：`.agents/skills/`。

不要默认引导用户 fork Trellis CLI 仓库。只有用户明确希望修改 Trellis 上游源码、发布 npm 包或贡献 PR 时，才把上游源码视为操作目标。

## 本地系统模型

Trellis 在用户项目内提供三层结构：

1. **工作流层**：`.trellis/workflow.md` 定义阶段、路由、下一步和提示块。
2. **持久化层**：`.trellis/tasks/`、`.trellis/spec/` 和 `.trellis/workspace/` 存储任务、规范和会话记忆。
3. **平台集成层**：平台目录中的钩子、设置、代理、Skill、命令、提示词和工作流把 Trellis 工作流连接到不同 AI 工具。

三层都位于用户项目内，因此 AI 可以直接读取和修改。

## 核心路径

| 路径 | 用途 |
| --- | --- |
| `.trellis/workflow.md` | 工作流阶段、Skill 路由和 workflow-state 提示块。 |
| `.trellis/config.yaml` | 项目配置、任务生命周期钩子、Monorepo 包配置和日志配置。 |
| `.trellis/spec/` | 用户项目专属的编码约定和思考指南。 |
| `.trellis/tasks/` | 每个任务的 PRD、技术注记、研究文件和 JSONL 上下文。 |
| `.trellis/workspace/` | 按开发者划分的日志和跨会话记忆。 |
| `.trellis/scripts/` | 命令、钩子和上下文注入使用的本地 Python 运行时。 |
| `.trellis/.runtime/` | 会话级运行时状态，例如当前任务指针。 |
| `.trellis/.template-hashes.json` | Trellis 管理文件的模板哈希，update 用它判断本地文件是否已被用户修改。 |

## AI 定制原则

1. **先找到本地事实来源**：不要凭记忆编辑。先读取 `.trellis/workflow.md`、`.trellis/config.yaml`、相关平台目录和任务文件。
2. **编辑用户项目，而不是 npm 包缓存**：修改项目内生成的文件，不要修改 `node_modules` 或全局 npm 安装目录。
3. **保持平台文件与 `.trellis/` 一致**：如果工作流路由变化，也要检查平台 Skill 或命令是否仍描述相同流程。
4. **把项目专属规则放入 `.trellis/spec/` 或本地 Skill**：不要把团队约定放入 `trellis-meta`。
5. **保留用户改动**：如果文件已在本地修改，应基于当前内容工作，不要用默认模板覆盖。

## 如何使用本目录

- 要了解 init 后存在哪些文件，读取 `generated-files.md`。
- 要修改阶段、路由或下一步，读取 `workflow.md`。
- 要修改任务模型、JSONL 上下文或活动任务行为，读取 `task-system.md`。
- 要修改编码约定注入，读取 `spec-system.md`。
- 要了解日志和跨会话记忆，读取 `workspace-memory.md`。
- 要修改钩子或子代理上下文加载，读取 `context-injection.md`。
