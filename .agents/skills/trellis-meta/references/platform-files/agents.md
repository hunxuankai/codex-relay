# Agent

Trellis Agent 文件定义专门角色。用户项目中常见的 Trellis Agent 包括：

- `trellis-research`
- `trellis-implement`
- `trellis-check`

文件位置和格式因平台而异，但职责边界应保持一致。

## Agent 职责

| Agent | 职责 |
| --- | --- |
| `trellis-research` | 调查问题，并将发现写入当前任务的 `research/`。 |
| `trellis-implement` | 根据 `prd.md`、可选的 `design.md` / `implement.md`、`implement.jsonl` 及相关 spec/research 实施。 |
| `trellis-check` | 审查改动、修复发现的问题并运行必要检查。 |

Agent 文件不应变成通用聊天提示词。它们应定义输入来源、写入边界、是否允许修改代码以及结果报告方式。

## 常见路径

| 平台 | Agent 路径 |
| --- | --- |
| Claude Code | `.claude/agents/trellis-*.md` |
| Cursor | `.cursor/agents/trellis-*.md` |
| OpenCode | `.opencode/agents/trellis-*.md` |
| Codex | `.codex/agents/trellis-*.toml` |
| Kiro | `.kiro/agents/trellis-*.json` |
| Gemini CLI | `.gemini/agents/trellis-*.md` |
| Qoder | `.qoder/agents/trellis-*.md` |
| CodeBuddy | `.codebuddy/agents/trellis-*.md` |
| Factory Droid | `.factory/droids/trellis-*.md` |
| Pi Agent | `.pi/agents/trellis-*.md` |
| Reasonix | `.reasonix/skills/trellis-*/SKILL.md`（子 Agent frontmatter） |
| ZCode | `.zcode/agents/trellis-*.md` |

GitHub Copilot 的 Agent/提示词支持由 `.github/agents/`、`.github/prompts/` 和 `.github/skills/` 等目录共同提供；应检查用户项目中实际生成的文件。

Kilo、Antigravity 和 Devin 等主会话工作流平台可能没有 Trellis 子 Agent 文件。它们通常依赖工作流/Skill 引导主会话。

## 两种上下文加载模式

### hook 推送

平台 hook 在 Agent 启动前注入任务上下文。Agent 文件本身可以更专注于职责和边界。

常见于支持 Agent hook 的平台。

### Agent 拉取

Agent 文件指示 Agent 在启动后读取：

- `python ./.trellis/scripts/task.py current --source`
- `implement.jsonl` or `check.jsonl`
- JSONL 引用的 spec/research 文件
- 当前任务的 `prd.md`
- 存在时读取 `design.md`
- 存在时读取 `implement.md`

此模式适合 hook 无法可靠重写子 Agent 提示词的平台。

## 本地修改场景

| 用户需求 | 编辑位置 |
| --- | --- |
| Implement Agent 必须遵循额外限制 | 平台的 `trellis-implement` Agent 文件。 |
| Check Agent 必须运行项目专属命令 | `trellis-check` Agent 文件，必要时再修改 `.trellis/spec/`。 |
| Research Agent 必须输出固定格式 | `trellis-research` Agent 文件。 |
| Agent 无法读取任务上下文 | Agent 前置指令或 `inject-subagent-context` hook。 |
| 添加项目专属 Agent | 平台 Agent 目录 + 相关工作流/命令/Skill 入口。 |

## 修改原则

1. **保持职责单一**。不要把 research、implement 和 check 职责混入同一个 Agent。
2. **明确读取顺序**。Agent 必须知道先从活动任务开始，读取 jsonl/spec 上下文，再读取 `prd.md`、存在时的 `design.md` 和存在时的 `implement.md`。
3. **明确写入边界**。Research 通常只写入 `research/`；implement 可以写代码；check 可以修复问题。
4. **在多平台项目中保持语义同步**。如果用户同时配置 Claude、Codex 和 Cursor，应判断对一个平台 Agent 的修改是否也要应用到其他平台。

## 默认不要编辑上游模板

本地 AI 默认应修改用户项目内的平台 Agent 文件。只有用户明确希望把改动贡献回 Trellis 时，才讨论上游模板源文件。
