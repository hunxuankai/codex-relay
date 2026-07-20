# 内置 Bundled Skill

“Bundled Skill”是随 Trellis CLI npm 包发布的多文件内置 Skill。与 Marketplace Skill（由用户单独安装到自己的 `.claude/skills/` 或其他平台 Skill 根目录）不同，Bundled Skill 会由 `trellis init` 自动写入每个受支持平台的 Skill 根目录，并由 `trellis update` 保持同步。它们是 Trellis 自身的一部分，而不是第三方内容。

Bundled Skill 是 `packages/cli/src/templates/common/bundled-skills/<skill>/` 下的目录，其中已有带 YAML frontmatter 的 `SKILL.md`，还可以包含 `references/`、资源或其他辅助文件。Trellis 将整个目录树原样复制到每个平台的 Skill 根目录，因此参考资料可以继续按需加载，而不会被扁平化进一个过大的 `SKILL.md`。

## Bundled Skill 与相邻概念的区别

| 源路径 | 类型 | 派发方式 |
| --- | --- | --- |
| `templates/common/bundled-skills/<name>/` | Bundled Skill（多文件） | 整个目录复制到每个平台的 Skill 根目录 |
| `templates/common/skills/<name>.md` | 单文件工作流 Skill | 包裹 frontmatter 后写为 `<root>/<name>/SKILL.md` |
| `templates/common/commands/<name>.md` | 斜杠命令/提示词 | 写入每个平台的命令目录（`.claude/commands/trellis/`、`.cursor/commands/trellis-*.md`、`.gemini/commands/trellis/*.toml` 等） |
| `templates/<platform>/skills/` | 平台专属 Skill | 只写入该平台目录（例如 `.codex/skills/`） |
| `.claude/skills/<my-skill>/` 等目录下的用户 Skill | Marketplace Skill 或用户自建 Skill | 完全不由 Trellis 管理 |

Trellis CLI 绝不会改动并非由自身模板加载器生成的内容。用户手动放入平台 Skill 根目录的任何内容都会保持不变。

## 当前 Bundled Skill（v0.6.0）

运行时通过枚举 `templates/common/bundled-skills/` 下的目录来发现此集合：

| Skill | 用途 |
| --- | --- |
| `trellis-meta` | 本 Skill。向在用户项目中工作的 AI 说明本地 Trellis 架构和定制入口。 |
| `trellis-session-insight` | 封装 `trellis mem` CLI，让 AI 知道何时以及如何检索过往 Claude Code / Codex / Pi Agent 对话日志。 |
| `trellis-spec-bootstrap` | 基于真实代码库创建或刷新 `.trellis/spec/` 的平台中立工作流，可选集成 GitNexus / ABCoder。 |
| `trellis-channel` | 指导 AI 何时使用 `trellis channel` 进行多 Agent 协作、forum/thread 持久看板和派发方等待模式的能力 Skill。 |

该列表在运行时发现，因此只需在 `bundled-skills/` 下添加新目录即可注册新 Skill（参见下文“添加新的 Bundled Skill”）。

## 各平台的 Bundled Skill 写入位置

`trellis init` 期间，每个平台配置器都会调用 `writeSkills(<root>, <workflowSkills>, resolveBundledSkills(ctx))`。`resolveBundledSkills` 读取 `templates/common/bundled-skills/` 下的每个目录、解析占位符，并返回 `{relativePath, content}` 条目的扁平列表。随后，`writeSkills` 将其镜像到平台的 Skill 根目录。

| 平台 | Bundled Skill 根目录 | 说明 |
| --- | --- | --- |
| Claude Code | `.claude/skills/<skill>/` | `configureClaude` |
| Cursor | `.cursor/skills/<skill>/` | `configureCursor` |
| Codex | `.agents/skills/<skill>/` | `configureCodex` 写入共享 `.agents/skills/` 根目录；Gemini CLI 0.40+ 也会读取该目录 |
| Gemini CLI | `.agents/skills/<skill>/` | 与 Codex 使用同一个共享根目录；两个配置器必须生成字节完全相同的输出 |
| Kiro | `.kiro/skills/<skill>/` | `configureKiro`（基于 Skill 的平台，没有命令） |
| Qoder | `.qoder/skills/<skill>/` | `configureQoder` |
| Codebuddy | `.codebuddy/skills/<skill>/` | `configureCodebuddy` |
| Copilot | `.github/skills/<skill>/` | `configureCopilot` |
| Droid | `.factory/skills/<skill>/` | `configureDroid` |
| Antigravity | `.agent/skills/<skill>/` | `configureAntigravity` |
| Devin | `.devin/skills/<skill>/` | `configureDevin` |
| Kilo | `.kilocode/skills/<skill>/` | `configureKilo` |
| ZCode | `.zcode/skills/<skill>/` | `configureZcode` |
| OpenCode | （由 `collectOpenCodeTemplates` 处理） | 使用相同的 `resolveBundledSkills(ctx)` 输出 |
| Pi、Reasonix | （各自的收集器） | 使用相同的 `resolveBundledSkills(ctx)` 输出 |

两条路径使用同一份数据：

1. `configureX(cwd)` 在 `trellis init` 期间写入文件。
2. `collectPlatformTemplates(platformId)`（位于 `configurators/index.ts`）返回 `Map<filePath, content>`，供 `trellis update` 检测偏移并填充 `.trellis/.template-hashes.json`。两条路径必须生成字节完全相同的输出，因此都会调用 `resolveBundledSkills(ctx)` 和 `collectSkillTemplates(root, …, resolveBundledSkills(ctx))`。

## 派发连接（代码路径）

将 Bundled Skill 自动派发到平台 Skill 根目录的机制位于两个文件中：

1. `packages/cli/src/templates/common/index.ts`
   - `listDirectories("bundled-skills")` 枚举磁盘上的 Skill。
   - `listBundledSkillFiles(skillDir)` 递归遍历每个 Skill 目录，并为每个文件返回 `{relativePath, content}`。
   - `getBundledSkillTemplates()` 返回缓存的 `CommonBundledSkill[]`。

2. `packages/cli/src/configurators/shared.ts`
   - `resolveBundledSkills(ctx)` 将列表扁平化为 `ResolvedSkillFile[]`，路径形式为 `<skill>/<relativePath>`，并解析占位符。
   - `writeSkills(skillsRoot, workflowSkills, bundledSkills)` 在 `skillsRoot` 下写入工作流 Skill 和 Bundled Skill 文件。
   - `collectSkillTemplates(skillsRoot, workflowSkills, bundledSkills)` 为更新/哈希管线返回形态相同的 `Map<filePath, content>`。

每个支持 Skill 的平台配置器都会导入这两个辅助函数（参见 `claude.ts`、`cursor.ts`、`codex.ts`、`gemini.ts`、`kiro.ts`、`qoder.ts`、`codebuddy.ts`、`copilot.ts`、`droid.ts`、`antigravity.ts`、`devin.ts`、`kilo.ts`）。`index.ts` 的 `PLATFORM_FUNCTIONS` 注册表也会在每个 `collectTemplates` 闭包中调用 `resolveBundledSkills(ctx)`，以保持 `trellis update` 跟踪一致。

## 添加新的 Bundled Skill

目录结构和派发连接已通用化，因此添加 Skill 只需要修改文件并验证分发结果。

1. **创建目录树。**

   ```
   packages/cli/src/templates/common/bundled-skills/<my-skill>/
     SKILL.md                     # YAML frontmatter + body
     references/                  # optional
       <topic>.md
     assets/                      # optional (anything readable as utf-8)
   ```

2. **编写有效的 `SKILL.md` 头部。** frontmatter 至少必须包含：

   ```yaml
   ---
   name: <my-skill>
   description: "When the AI should reach for this skill. Triggering phrases go here."
   ---
   ```

   每个平台的自动触发机制都匹配 `description`，因此它应描述用户意图触发条件，而不是 Skill 内部实现。

3. **在适当位置使用占位符。** Bundled Skill 内容会经过 `resolvePlaceholders(file.content, ctx)`。`resolvePlaceholders` 支持的 `{{platform_name}}`、`{{python_cmd}}` 等 token 会按平台替换。

4. **无需添加派发连接。** `listDirectories("bundled-skills")` 会自动发现新目录，因此所有平台都会在下次 `trellis init` 或 `trellis update` 时收到它。

5. 发布前**验证分发路径**。历史上，跳过以下任一步骤都曾导致功能被记录为已内置，但发布的 npm tarball 中缺少相应文件：

   - 源文件存在于即将打 tag 的分支上。
   - `pnpm --filter @mindfoldhq/trellis build` 将资源复制到 `dist/templates/common/bundled-skills/<skill>/`。
   - `npm pack --dry-run --json` 包含预期的 `dist/**` 路径。
   - 在全新的临时项目中，`trellis init` 写入 `.claude/skills/<skill>/SKILL.md`、`.agents/skills/<skill>/SKILL.md`、`.zcode/skills/<skill>/SKILL.md` 等文件。
   - `.trellis/.template-hashes.json` 列出生成的文件。
   - 在该临时项目中运行 `trellis update --dry-run`，输出“Already up to date!”。

6. 如果该 Skill 是在其他项目将要升级到的版本中新增的，请**添加迁移清单条目**。没有显式清单条目时，文件仍会通过 `trellis update` 的标准“文件缺失”分支落地，但清单能让该变更出现在变更日志中。

## 在本地覆盖 Bundled Skill

目前没有正式的“项目本地 Skill”机制（例如 `.trellis/skills/`）。Bundled Skill 以平台目录为根，因此所有覆盖也以平台目录为根。

支持的模式依赖 `trellis update` 现有的模板哈希差异检测：

1. 直接编辑本地文件。例如：`.claude/skills/trellis-meta/SKILL.md`。
2. 文件哈希随后会与 `.trellis/.template-hashes.json` 中的记录产生差异。
3. 下次 `trellis update` 会检测到用户修改并保持文件不变（没有显式 `--force` 时，Trellis 绝不会覆盖用户修改过的文件）。

注意事项：

- 覆盖只对所编辑目录对应的平台生效。例如，要在 Claude Code 和 Codex 中覆盖同一个 Skill，必须同时编辑 `.claude/skills/<name>/` 和 `.agents/skills/<name>/`。
- 后续的 `trellis update --force` 会覆盖本地编辑。请将覆盖内容纳入版本控制，以便必要时重新应用。
- 安装在同一平台 Skill 根目录下、但目录名称不同的 Marketplace Skill（例如 `.claude/skills/my-custom-meta/`）不会被 Trellis 改动。当目标是增加行为而非修改 Bundled Skill 时，这是更清晰的选择。
- 团队私有约定应放在 `.trellis/spec/` 或独立的 Marketplace 风格本地 Skill 中，不应通过修改 `trellis-meta` 本身实现。参见 `customize-local/add-project-local-conventions.md`。

## 从项目中移除 Bundled Skill

Bundled Skill 没有按项目退出的开关。可选择以下两种方式：

1. **删除每个平台 Skill 根目录中的对应目录。** `trellis update` 会发现文件缺失，并与 `.template-hashes.json` 比较，将删除视为其他用户修改；除非传入 `--force`，否则不会静默重建目录。

2. **固定到尚未包含该 Skill 的 Trellis 版本。** Bundled Skill 集合在构建时确定，因此安装旧版 CLI 是永久排除当前版本所带 Skill 的唯一方式。

不支持第三种方式，即全局禁用所有 Bundled Skill。每个配置器都会无条件派发。若要添加此类开关，需要修改 `configurators/index.ts` 中的 `PLATFORM_FUNCTIONS` 和每个 `configureX` 函数。

## 操作规则

- 将 `templates/common/bundled-skills/` 视为现有哪些 Bundled Skill 的唯一事实来源。不要按平台手动维护 Skill 列表。
- 不要在 Bundled `SKILL.md` 中添加平台专属逻辑。如果行为只适用于特定平台，请放入 `templates/<platform>/skills/`。
- 不要在未通过 Skill 的描述和参考资料明确依赖关系时，将 Bundled Skill 与特定 CLI 命令（例如 `trellis mem`）耦合；旧版本用户可能没有该命令。
- 不要在 Bundled Skill 中存储项目私有内容。Bundled Skill 是面向所有用户发布的公共内容；项目规则应放在 `.trellis/spec/` 或本地 Skill 中。
