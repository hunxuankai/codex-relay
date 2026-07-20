# 平台文件概览

Trellis 将同一套本地架构连接到不同的 AI 工具。`.trellis/` 保存共享运行时；平台目录保存适配器文件，用于定义各 AI 工具如何进入 Trellis。

本地 AI 修改 Trellis 时，应首先区分两类文件：

- **共享文件**：`.trellis/workflow.md`、`.trellis/tasks/`、`.trellis/spec/`、`.trellis/scripts/`。
- **平台文件**：`.claude/`、`.codex/`、`.cursor/`、`.opencode/`、`.kiro/`、`.gemini/`、`.qoder/`、`.codebuddy/`、`.github/`、`.factory/`、`.pi/`、`.trae/`、`.kilocode/`、`.agent/`、`.devin/`、`.reasonix/`、`.zcode/` 等目录。

平台文件不保存业务状态。它们让对应的 AI 工具能够读取 Trellis 状态、调用 Trellis 脚本并加载 Trellis Skill/Agent/hook。

## 平台文件类别

| 类别 | 常见路径 | 用途 |
| --- | --- | --- |
| settings/config | `.claude/settings.json`、`.codex/hooks.json`、`.qoder/settings.json`、`.trae/hooks.json` | 注册 hook、插件、扩展或平台行为。 |
| hooks/plugins/extensions | `.claude/hooks/`、`.opencode/plugins/`、`.pi/extensions/` | 在会话开始、用户输入、Agent 启动、Shell 执行等事件中注入上下文。 |
| agents | `.claude/agents/`、`.codex/agents/`、`.kiro/agents/`、`.zcode/agents/` | 定义 `trellis-research`、`trellis-implement` 和 `trellis-check`。 |
| skills | `.claude/skills/`、`.agents/skills/`、`.qoder/skills/`、`.zcode/skills/` | 描述可自动触发或按需读取的能力。 |
| commands/prompts/workflows | `.cursor/commands/`、`.github/prompts/`、`.devin/workflows/`、`.zcode/commands/` | 用户显式调用的入口。 |

## 三种平台集成模式

### 1. Hook / 扩展驱动

这些平台可以在特定事件上触发脚本或插件，并主动向 AI 注入 Trellis 上下文。

常见能力：

- 会话开始时注入 `.trellis/` 概览。
- 在每个用户轮次提供工作流状态提示。
- 子 Agent 启动时注入 PRD/spec/research。
- Shell 命令继承会话身份。

要修改“AI 在什么时机知道什么”，首先检查 hook/插件/扩展和设置。

### 2. Agent 前置指令 / 拉取型

某些平台无法可靠地让 hook 重写子 Agent 提示词，因此由 Agent 文件自身指示 Agent 在启动后读取活动任务、PRD 和 JSONL 上下文。

要修改子 Agent 加载上下文的方式，检查 Agent 文件本身。

### 3. 主会话工作流

某些平台不具备 Trellis 子 Agent 或 hook 能力。它们依赖工作流/Skill/命令，引导主会话 AI 读取文件、运行脚本并推进任务。

要修改行为，检查平台工作流/Skill/命令和 `.trellis/workflow.md`。

## 本地修改顺序

用户要求定制某个平台的行为时，AI 应按以下顺序检查文件：

1. 读取 `.trellis/workflow.md`，确认共享流程。
2. 读取目标平台的 settings/config，了解已注册哪些 hook/Agent/Skill/命令。
3. 读取目标平台的 Agent/Skill/命令/hook。
4. 修改最接近用户需求的本地文件。
5. 如果改动影响共享流程，同步 `.trellis/workflow.md` 或 `.trellis/spec/`。

不要只修改平台文件而忘记共享工作流。也不要只修改 `.trellis/workflow.md`，却忘记平台入口可能仍包含旧说明。
