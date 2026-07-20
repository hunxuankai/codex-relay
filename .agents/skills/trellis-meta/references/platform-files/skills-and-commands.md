# Skill、命令、提示词和工作流

Skill 和命令是用户与 Trellis 交互的文本入口。不同平台使用不同名称，但核心目的相同：当用户表达某种意图时，告诉 AI 如何进入 Trellis 流程。

## 概念差异

| 类型 | 触发方式 | 最适合 |
| --- | --- | --- |
| skill | AI 自动匹配或用户显式提及 | 长期能力、工作流规则和修改指南。 |
| command | 用户显式调用 | continue、finish-work 等明确操作入口。 |
| prompt | 用户显式调用或平台选择 | 类似 command，但采用平台提示词格式。 |
| workflow | 用户显式选择或平台自动匹配 | 在没有子 Agent/hook 时引导主会话。 |

Trellis 工作流 Skill 通常共享一组语义：brainstorm、before-dev、check、update-spec、break-loop。`trellis-meta` 等多文件内置 Skill 使用分层参考资料。

## 常见路径

| 平台 | 常见入口 |
| --- | --- |
| Claude Code | `.claude/skills/`, `.claude/commands/` |
| Cursor | `.cursor/skills/`, `.cursor/commands/` |
| OpenCode | `.opencode/skills/`, `.opencode/commands/` |
| Codex | `.agents/skills/`, `.codex/skills/` |
| Kilo | `.kilocode/skills/`, `.kilocode/workflows/` |
| Kiro | `.kiro/skills/` |
| Gemini CLI | `.agents/skills/`, `.gemini/commands/` |
| Antigravity | `.agent/skills/`, `.agent/workflows/` |
| Devin | `.devin/skills/`, `.devin/workflows/` |
| Qoder | `.qoder/skills/`, `.qoder/commands/` |
| CodeBuddy | `.codebuddy/skills/`, `.codebuddy/commands/` |
| GitHub Copilot | `.github/skills/`, `.github/prompts/` |
| Factory Droid | `.factory/skills/`, `.factory/commands/` |
| Pi Agent | `.pi/skills/` |
| Reasonix | `.reasonix/skills/` |
| ZCode | `.zcode/skills/`, `.zcode/commands/` |

在用户项目中，以 init 实际生成的文件为准。

## Skill 结构

常见 Skill 是一个目录：

```text
trellis-meta/
├── SKILL.md
└── references/
```

`SKILL.md` 应告诉 AI：

- 何时使用该 Skill。
- 当前任务应首先读取哪份参考资料。
- 哪些事不能做。

较长说明放在参考资料中，避免入口文件容纳所有内容。

## Command/Prompt/Workflow 结构

命令、提示词和工作流通常是单文件。其内容应包括：

- 何时使用。
- 读取哪些 `.trellis/` 文件。
- 运行哪些脚本。
- 完成后如何报告。

它们不应保存任务状态；任务状态属于 `.trellis/tasks/` 和 `.trellis/.runtime/`。

## 本地修改场景

| 用户需求 | 编辑位置 |
| --- | --- |
| 修改 AI 自动触发规则 | 对应 Skill 的 frontmatter `description`。 |
| 修改用户命令行为 | 对应 command/prompt/workflow 文件。 |
| 添加项目本地 Skill | 平台 Skill 目录或共享 `.agents/skills/`。 |
| 让多个平台共享同一能力 | 在各平台 Skill 目录编写等价 Skill，或在支持的平台使用 `.agents/skills/` 共享层。 |
| 修改 finish/continue 入口 | 平台 command/prompt/workflow。 |

## 修改原则

1. **保持入口文件简短；长内容放入参考资料**。这对 `trellis-meta` 等多文件 Skill 尤其重要。
2. **让触发描述具体明确**。描述过宽可能误触发，过窄则可能无法触发。
3. **在各平台保持相同语义**。文件格式可以不同，但行为描述应一致。
4. **把项目专属能力放入本地 Skill**。不要把团队私有流程放入公共 `trellis-meta`。

如果用户只是希望本地 AI 多了解一条项目规则，通常应创建项目本地 Skill 或更新 `.trellis/spec/`，而不是修改 Trellis 内置工作流 Skill。
