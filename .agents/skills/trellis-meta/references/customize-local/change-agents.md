# 修改本地 Agent

当用户希望修改 `trellis-research`、`trellis-implement` 或 `trellis-check` 的行为时，编辑用户项目中的平台 Agent 文件。

## 首先读取这些文件

1. 目标平台的 Agent 目录
2. `.trellis/workflow.md` 中的 Phase 2 / research 路由
3. 当前任务的 `prd.md`
4. 当前任务的 `implement.jsonl` / `check.jsonl`
5. 相关 hook 或 Agent 前置指令

## 常见路径

| 平台 | 路径 |
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

以用户项目中的实际路径为准。

## 常见需求

| 需求 | 应编辑的 Agent |
| --- | --- |
| Research 必须写入文件，而不能只在聊天中回复 | `trellis-research` |
| 实施前必须读取特定本地 spec | `trellis-implement` + `implement.jsonl` 配置规则 |
| 检查时必须运行特定命令 | `trellis-check` |
| Agent 不得修改某些目录 | 对应 Agent 的写入边界指令 |
| 必须固定 Agent 输出格式 | 对应 Agent 的最终输出/报告指令 |

## 修改原则

1. **保持角色边界**：research 负责调查并持久化结果；implement 负责编写实现；check 负责审查和修复。
2. **不要把项目 spec 硬编码进 Agent**：长期规范属于 `.trellis/spec/`；Agent 负责读取它们。
3. **明确读取顺序**：活动任务 → PRD → info → JSONL → spec/research。
4. **明确写入边界**：哪些目录可以写入，哪些目录不得写入。
5. **跨平台同步**：用户配置了多个平台时，判断只修改当前平台还是修改所有平台 Agent。

## Agent 拉取型平台

如果 Agent 文件包含“启动后读取任务/上下文”的前置步骤，编辑时不要删除它们。否则 Agent 将只依赖聊天上下文工作，并绕过 Trellis 的核心机制。

## Hook 推送型平台

即使上下文由 hook 注入，Agent 文件仍应保留职责边界。不要仅仅因为 hook 会注入上下文，就从 Agent 中删除 PRD/spec 要求。
