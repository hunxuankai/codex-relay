# Hook 与设置

Hook/settings 是连接平台与 Trellis 的入口层。它们决定平台在各类事件上运行哪些脚本、插件或扩展。

## Settings 职责

settings/config 文件通常注册：

- 会话启动 hook：新会话开始或上下文重置时注入 Trellis 概览。
- 工作流状态 hook：解析 `.trellis/workflow.md` 中的 `[workflow-state:STATUS]` 块，并在每次用户输入时输出与当前任务 `status` 匹配的正文。它只负责解析；脚本不内嵌回退内容。
- 子 Agent 上下文 hook：implementation/check/research Agent 启动时注入任务上下文。
- Shell/会话桥接：让 Shell 命令看到同一个 Trellis 会话身份。
- 平台插件或扩展入口。

常见文件：

| 平台 | settings/config |
| --- | --- |
| Claude Code | `.claude/settings.json` |
| Cursor | `.cursor/hooks.json` |
| Codex | `.codex/hooks.json`, `.codex/config.toml` |
| OpenCode | `.opencode/package.json`, `.opencode/plugins/*` |
| Kiro | `.kiro/hooks/` + 平台配置 |
| Gemini CLI | `.gemini/settings.json` |
| Qoder | `.qoder/settings.json` |
| CodeBuddy | `.codebuddy/settings.json` |
| GitHub Copilot | `.github/copilot/hooks.json` |
| Factory Droid | `.factory/settings.json` |
| Pi Agent | `.pi/settings.json`, `.pi/extensions/trellis/` |
| Trae IDE | `.trae/hooks.json` |

Reasonix 和 ZCode 是拉取型平台，不使用 hook 或 settings 文件；其 Agent 文件包含启动后读取上下文的前置指令。

项目中是否存在这些文件，取决于用户运行了哪些 `trellis init --<platform>` 参数。

## Hook 脚本类型

| 脚本 | 用途 |
| --- | --- |
| `session-start.py` | 生成会话启动上下文。 |
| `inject-workflow-state.py` | 解析 `.trellis/workflow.md` 中的 `[workflow-state:STATUS]` 块，并输出与当前任务状态匹配的正文。没有匹配块时回退为 `Refer to workflow.md for current step.`。 |
| `inject-subagent-context.py` | 向子 Agent 注入 PRD、JSONL 上下文和相关 spec/research。 |
| `inject-shell-session-context.py` | 让 Shell 命令继承 Trellis 会话身份。 |

并非每个平台都具备所有 hook。不要仅仅因为某个平台缺少一个 hook 就从其他平台复制文件；应先确认该平台是否支持对应事件。

## 本地修改场景

| 用户需求 | 编辑位置 |
| --- | --- |
| AI 在新会话中应看到更多/更少上下文 | 平台 `session-start` hook。 |
| 修改逐轮提示策略 | `.trellis/workflow.md` 中的 `[workflow-state:STATUS]` 块。hook 会逐字解析 workflow.md，无需修改脚本。 |
| 子 Agent 无法读取 PRD/spec | `inject-subagent-context` hook 或 Agent 前置指令。 |
| Shell 中的 `task.py current` 找不到活动任务 | Shell/会话桥接 hook 或平台环境变量配置。 |
| 禁用自动注入 | settings/config 中对应的 hook 注册。 |

## 修改原则

1. **Settings 负责接线；hook 定义行为**。如果只修改 hook，平台可能永远不会调用它；如果只修改 settings，行为可能不会变化。
2. **先确认平台事件名称**。不同平台对 SessionStart、UserPromptSubmit、AgentSpawn、Shell 执行等事件使用不同名称。
3. **Hook 读取本地 `.trellis/`，而不是上游源码**。用户项目中的 `.trellis/scripts/` 和 `.trellis/workflow.md` 是默认目标。
4. **错误必须可见**。Hook 失败时应告知用户哪些内容未被注入，而不是静默地让 AI 缺少上下文。

## 故障排查路径

如果用户说“AI 没有读取 Trellis 状态”：

1. 检查平台 settings 是否注册了该 hook。
2. 检查 hook 文件是否存在。
3. 手动运行 hook 依赖的 `.trellis/scripts/get_context.py` 或 `task.py current --source` 命令。
4. 检查 `.trellis/.runtime/sessions/` 中是否存在活动任务状态。
5. 检查平台 Shell 是否传递会话身份。
