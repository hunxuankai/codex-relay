# 修改本地 Skill、命令、提示词和工作流

当用户希望修改 AI 入口、自动触发规则或显式命令行为时，编辑本地平台目录中的 Skill、命令、提示词或工作流。

编辑前，先判断即将修改的 Skill 属于哪一类：

- **上游 Bundled Skill**：`trellis-meta`、`trellis-spec-bootstrap`、`trellis-session-insight`、`trellis-channel`。事实来源位于 Trellis CLI 仓库的 `packages/cli/src/templates/common/bundled-skills/<name>/`；`trellis init` / `trellis update` 时，`getBundledSkillTemplates()` 会自动把它派发到每个平台的 Skill 根目录。本地编辑由 `.trellis/.template-hashes.json` 跟踪，并会在下次更新时标记出来。
- **项目本地 Skill**：`.{platform}/skills/` 下除此之外的所有内容。它们归用户所有，不会由 `trellis update` 刷新。

本文余下部分用“Skill”指本地文件；这两类文件的覆盖和冲突规则不同。

## 编辑前先读取这些文件

1. `.trellis/workflow.md`
2. 目标平台的 Skill/命令/提示词/工作流目录
3. 相关 Agent 或 hook 文件
4. `.trellis/spec/` 中是否已存在项目规则
5. `.trellis/.template-hashes.json`：即将编辑的 Skill 有记录则归上游所有，无记录则属于项目本地

## 如何选择入口类型

| 目标 | 建议 |
| --- | --- |
| 让 AI 自动知道某项能力 | 添加或修改 Skill。 |
| 让用户通过命令手动触发 | 添加或修改命令/提示词/工作流。 |
| 团队项目约定 | 优先使用 `.trellis/spec/` 或项目本地 Skill，绝不能放进 Bundled Skill 目录。 |
| 为用户自己的项目调整 Bundled Skill（如 `trellis-meta`） | 创建名称不同、用于覆盖意图的项目本地同级 Skill，或编辑 `.trellis/spec/`。Bundled Skill 目录内的编辑只会保留到下次 `trellis update`，且每次都需要选择“keep”。 |
| 将改动贡献回上游 | 编辑 Trellis CLI 仓库中的 `packages/cli/src/templates/common/bundled-skills/<name>/`，而不是已派发的副本。 |
| 修改 Trellis 流程语义 | 同步 `.trellis/workflow.md`。 |

## 修改 Skill

Skill 通常采用以下结构：

```text
<skill-name>/
├── SKILL.md
└── references/
```

`SKILL.md` 应保持简短，只负责触发和路由。长篇内容放在 `references/` 中，让 AI 按需读取。

frontmatter 的 `description` 应明确说明何时使用该 Skill。例如：

```yaml
description: "Use when customizing this project's deployment workflow and release checklist."
```

不要编写“有帮助的项目 Skill”之类含糊描述，否则可能错误触发。

### Bundled Skill 与项目本地 Skill

两种截然不同的所有权模型使用相同的目录结构：

| 方面 | Bundled（`trellis-meta`、`trellis-spec-bootstrap`、`trellis-session-insight`、`trellis-channel`） | 项目本地 |
| --- | --- | --- |
| 事实来源 | Trellis CLI 仓库中的 `packages/cli/src/templates/common/bundled-skills/<name>/` | 用户项目本身 |
| 派发 | `trellis init` / `trellis update` 时，由 `getBundledSkillTemplates()`（`packages/cli/src/templates/common/index.ts`）自动派发到每个平台的 Skill 根目录 | 由用户（或另一个 Skill）创建，且永不移动 |
| 哈希跟踪 | 每个文件都记录在 `.trellis/.template-hashes.json` 中；更新时提示冲突 | 不跟踪 |
| 本地编辑 | 允许，但下次更新时会标记为“用户已修改” | 可自由编辑 |
| 正确定制方式 | 添加一个名称不同、用于补充（或取代）Bundled Skill 的*新*项目本地 Skill | 直接编辑文件 |

如果目标是“让项目 AI 在讨论发布说明时采用不同做法”，几乎总应使用项目本地 Skill，而不是修改 `trellis-meta/`。

## 修改命令/提示词/工作流

显式入口应说明：

- 用户如何触发。
- 需要读取哪些 `.trellis/` 文件。
- 需要运行哪些脚本。
- 完成后如何报告。

如果命令只是重复工作流规则，优先让它引用/读取 `.trellis/workflow.md`，不要维护第二份流程副本。

## 常见路径

| 平台 | 入口目录 |
| --- | --- |
| Claude Code | `.claude/skills/`, `.claude/commands/` |
| Cursor | `.cursor/skills/`, `.cursor/commands/` |
| OpenCode | `.opencode/skills/`, `.opencode/commands/` |
| Codex | `.agents/skills/`, `.codex/skills/` |
| Gemini CLI | `.agents/skills/`, `.gemini/commands/` |
| Kiro | `.kiro/skills/` |
| Qoder | `.qoder/skills/`, `.qoder/commands/` |
| CodeBuddy | `.codebuddy/skills/`, `.codebuddy/commands/` |
| GitHub Copilot | `.github/skills/`, `.github/prompts/` |
| Factory Droid | `.factory/skills/`, `.factory/commands/` |
| Pi Agent | `.pi/skills/` |
| Reasonix | `.reasonix/skills/`（没有单独的命令目录；斜杠命令内置于平台） |
| ZCode | `.zcode/skills/`, `.zcode/commands/` |
| Kilo / Antigravity / Devin | workflows + skills |

以上每个目录都是四个 Bundled Skill 的派发目标。每个平台都会在 `trellis init` 时收到完整副本，并在 `trellis update` 时刷新；无需手动连接。

## 添加项目本地 Skill

如果用户希望记录团队私有定制，请创建项目本地 Skill。绝不要把项目私有内容放入 Bundled Skill 目录，因为 `trellis update` 会覆盖它。

```text
.claude/skills/project-trellis-local/
└── SKILL.md
```

对于多平台项目，在每个平台的 Skill 目录中添加等价版本；对于支持共享层的平台（Codex、Gemini CLI），也可以使用 `.agents/skills/`。

选择一个**不与** Bundled Skill 集合冲突的名称：

- `trellis-meta`
- `trellis-spec-bootstrap`
- `trellis-session-insight`
- `trellis-channel`

复用名称会导致 `getBundledSkillTemplates()` 在下次更新时覆盖项目本地副本。常见约定是添加项目前缀：`acme-trellis-deploy`、`acme-trellis-onboarding`。

## 注意事项

- 不要把所有平台的语法混在同一个文件中。
- 不要只修改一个平台入口，却声称支持所有平台。
- 不要把长期工程约定隐藏在命令中；应写入 `.trellis/spec/`。
- 不要手动编辑任意 `.{platform}/skills/` 目录下 `trellis-meta/`、`trellis-spec-bootstrap/`、`trellis-session-insight/` 或 `trellis-channel/` 中的文件，并期待改动永久保留；它们是 Bundled Skill，会由 `trellis update` 刷新。应将改动贡献到上游，或添加用于补充它们的项目本地 Skill。
- 当 `trellis update` 报告 Bundled Skill 文件存在“modified by you”冲突时，只有在接受手动维护差异的情况下才选择 **keep**；否则接受覆盖，并用项目本地 Skill 重新实现该意图。
