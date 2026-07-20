# 平台文件映射

本页按平台列出用户项目中常见的 Trellis 文件位置。实际项目中是否存在某个平台目录，取决于用户运行了哪些 `trellis init --<platform>` 命令。

## 矩阵

| 平台 | CLI 参数 | 主目录 | Skill 目录 | Agent 目录 | Hook/扩展 |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `--claude` | `.claude/` | `.claude/skills/` | `.claude/agents/` | `.claude/hooks/` + `.claude/settings.json` |
| Cursor | `--cursor` | `.cursor/` | `.cursor/skills/` | `.cursor/agents/` | `.cursor/hooks.json` + `.cursor/hooks/` |
| OpenCode | `--opencode` | `.opencode/` | `.opencode/skills/` | `.opencode/agents/` | `.opencode/plugins/` |
| Codex | `--codex` | `.codex/` | `.agents/skills/` | `.codex/agents/` | `.codex/hooks/` + `.codex/hooks.json` |
| Kilo | `--kilo` | `.kilocode/` | `.kilocode/skills/` | 通常没有 | `.kilocode/workflows/` |
| Kiro | `--kiro` | `.kiro/` | `.kiro/skills/` | `.kiro/agents/` | `.kiro/hooks/` |
| Gemini CLI | `--gemini` | `.gemini/` | `.agents/skills/` | `.gemini/agents/` | `.gemini/settings.json` + `.gemini/hooks/` |
| Antigravity | `--antigravity` | `.agent/` | `.agent/skills/` | 通常没有 | `.agent/workflows/` |
| Devin | `--devin` | `.devin/` | `.devin/skills/` | 通常没有 | `.devin/workflows/` |
| Qoder | `--qoder` | `.qoder/` | `.qoder/skills/` | `.qoder/agents/` | `.qoder/hooks/` + `.qoder/settings.json` |
| CodeBuddy | `--codebuddy` | `.codebuddy/` | `.codebuddy/skills/` | `.codebuddy/agents/` | `.codebuddy/hooks/` + `.codebuddy/settings.json` |
| GitHub Copilot | `--copilot` | `.github/` | `.github/skills/` | `.github/agents/` | `.github/copilot/hooks/` + 提示词 |
| Factory Droid | `--droid` | `.factory/` | `.factory/skills/` | `.factory/droids/` | `.factory/hooks/` + settings |
| Pi Agent | `--pi` | `.pi/` | `.pi/skills/` | `.pi/agents/` | `.pi/extensions/trellis/`（原生 `trellis_subagent` 工具）+ `.pi/settings.json` |
| Trae IDE | `--trae` | `.trae/` | `.trae/skills/` | `.trae/agents/` | `.trae/hooks/` + `.trae/hooks.json` |
| Reasonix | `--reasonix` | `.reasonix/` | `.reasonix/skills/` | 无——子 Agent 是带有 `runAs: subagent` frontmatter 的 Skill | 无 |
| ZCode | `--zcode` | `.zcode/` | `.zcode/skills/` | `.zcode/agents/` | 拉取型前置指令（无 hook） |

## 能力分组

### Trellis 子 Agent 支持

这些平台通常具有 `trellis-research`、`trellis-implement` 和 `trellis-check` 文件：

- Claude Code
- Cursor
- OpenCode
- Codex
- Kiro
- Gemini CLI
- Qoder
- CodeBuddy
- GitHub Copilot
- Factory Droid
- Pi Agent
- Trae IDE
- Reasonix（以 `.reasonix/skills/` 下带有 `runAs: subagent` 的 Skill 提供，而不是单独的 `agents/` 目录）
- ZCode

修改 implementation/check/research 行为时，首先查找对应的平台 Agent 文件。

### 原生 Trellis 子 Agent 工具

某些平台提供宿主运行时能够识别的一等工具。模型像调用其他工具一样调用它，宿主负责呈现进度卡片、根据 `.<platform>/agents/` 验证 Agent 名称，并强制执行派发模式。

- Pi Agent——`trellis_subagent` 工具定义在 `.pi/extensions/trellis/index.ts`。它支持 `single` / `parallel` / `chain` 派发模式，并发送实时 `trellis-subagent-progress` 事件。

修改这些平台的子 Agent 派发行为时，应编辑扩展文件，**而不是** Agent Markdown——Agent Markdown 定义职责，但派发、验证和进度呈现由宿主扩展负责。

### 主会话工作流平台

这些平台更多依赖工作流/Skill 引导主会话：

- Kilo
- Antigravity
- Devin

修改行为时，首先检查工作流和 Skill。不要假设 Trellis 子 Agent 一定存在。

### 共享 `.agents/skills/`

Codex 和 Gemini CLI 写入共享 `.agents/skills/` 层。某些支持 agentskills.io 的工具也能读取该目录。如果用户希望多个兼容工具共享同一个 Skill，优先考虑 `.agents/skills/`，但不要假设所有平台都会读取它。ZCode 将 Trellis 管理的 Skill 保存在 `.zcode/skills/` 下。

## 修改平台文件时的决策规则

1. 用户指定了平台：只修改该平台目录，除非还必须修改共享 workflow/spec 文件。
2. 用户说“所有平台都应这样做”：逐个平台同步等价入口，不要只修改一个目录。
3. 用户只说“我的 AI”：检查项目中实际存在的配置目录，并据此判断当前 AI 平台。
4. 用户需要项目规则：优先使用 `.trellis/spec/` 或项目本地 Skill。
5. 用户需要修改 Trellis 行为：编辑 `.trellis/workflow.md` 以及平台 hook/Agent/Skill/命令。

## 路径不一致时

平台生态会变化，用户项目也可能已经定制。如果本表与本地文件不一致，以用户项目中的实际 settings/config 为准：

- 检查 settings 注册的 hook。
- 检查 command/prompt/workflow 指向的脚本。
- 根据 Agent 文件当前写明的读取规则判断行为。

不要仅仅因为自定义文件未列入本路径表就删除它。

### `.omp/`——Oh My Pi（OMP）

由扩展支持的平台。OMP 原生 Provider 会自动发现所有子目录。

```
.omp/
├── commands/          # Slash 命令（扁平 .md）
├── skills/            # 自动触发的 Skill（每个目录一个 SKILL.md）
├── agents/            # Agent 定义（.md）
└── extensions/
    └── trellis/
        └── index.ts   # Trellis 扩展（上下文注入）
```

没有 `settings.json`——OMP 自动扫描 `.omp/` 子目录。
没有 Python hook——等价的 hook 行为位于 TypeScript 扩展中。
