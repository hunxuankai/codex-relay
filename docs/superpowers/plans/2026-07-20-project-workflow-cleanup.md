# 项目工作流清理实施计划

> **面向代理执行者：** 必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans` 逐项执行本计划。所有步骤使用复选框（`- [ ]`）跟踪。

**目标：** 删除已经完成使命的 `项目提示词.txt`，清理旧文档引用，记录文档语言规则，并保持当前 `AGENTS.md + Superpowers + docs + Git` 工作流而不引入 Trellis。

**架构：** 这是一次纯仓库文档整理，不修改应用源码、依赖或运行行为。先记录当前文件与三处旧引用作为失败基线，再删除文件、精确改写三处文档并将简体中文要求固化到 `AGENTS.md`，最后验证旧引用消失、语言规则存在、`.trellis/` 未创建且 Git 差异只包含批准范围。

**技术栈：** Git、Markdown、PowerShell、ripgrep

---

### 任务 1：记录清理前基线

**文件：**
- 检查：`项目提示词.txt`
- 检查：`AGENTS.md`
- 检查：`docs/verification-report.md`
- 检查：`docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- 检查：`docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`

- [x] **步骤 1：确认旧提示词文件仍然存在**

运行：

```powershell
Test-Path -LiteralPath '项目提示词.txt'
```

预期：输出 `True`。相对于“旧提示词文件应当不存在”的批准要求，这是修改前的失败基线。

- [x] **步骤 2：确认三个旧引用仍然存在**

运行：

```powershell
rg -n '项目提示词\.txt|项目提示词' `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md
```

预期：输出三处匹配，分别是发布签名说明、原完成定义和历史 `git add` 命令。

- [x] **步骤 3：确认仓库规则尚未记录文档语言要求**

运行：

```powershell
rg -n '项目文档.*简体中文|设计文档.*简体中文' AGENTS.md
if ($LASTEXITCODE -eq 1) { Write-Output 'Document language rule is absent'; exit 0 }
exit $LASTEXITCODE
```

预期：输出 `Document language rule is absent`，作为新增持久化规则前的失败基线。

### 任务 2：删除旧提示词并清理文档引用

**文件：**
- 删除：`项目提示词.txt`
- 修改：`AGENTS.md`
- 修改：`docs/verification-report.md`
- 修改：`docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- 修改：`docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`

- [x] **步骤 1：删除旧提示词文件**

应用以下补丁：

```diff
*** Begin Patch
*** Delete File: 项目提示词.txt
*** End Patch
```

- [x] **步骤 2：在仓库规则中固化简体中文要求**

在 `AGENTS.md` 末尾新增：

```markdown
13. 项目文档、设计文档、实施计划、验证报告和 README 默认使用简体中文；命令、代码标识符、文件名以及没有通行中文译名的技术专有名词可以保留原文。
```

- [x] **步骤 3：把发布报告中的旧引用改为长期有效的发布要求**

在 `docs/verification-report.md` 中，将：

```markdown
- 代码签名未完成：仓库没有发布证书或签名授权；项目提示词要求在正式发布前说明签名，
  未把实际签名列为本次最终验收条件。
```

替换为：

```markdown
- 代码签名未完成：仓库没有发布证书或签名授权；正式发布要求需要说明签名状态，未把实际
  签名列为本次最终验收条件。
```

- [x] **步骤 4：移除原设计对已删除提示词的依赖**

在 `docs/superpowers/specs/2026-07-20-codex-relay-design.md` 中，将完成定义段落替换为：

```markdown
完成必须同时满足已批准设计和项目验收要求：Provider CRUD 与切换可用；主窗口和托盘共享后端；配置写入保留无关内容并可回滚；自检、监控、单实例、自启动、通知和托盘行为实现；测试不触碰真实数据；文档齐全；所有检查、Debug、Release 和 NSIS 构建都实际成功；最终报告逐项列出产物、命令结果、已知限制和人工验证步骤。
```

- [x] **步骤 5：从历史初始化命令中移除已删除文件**

在 `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md` 中，将：

```powershell
git add .gitignore package.json package-lock.json index.html tsconfig*.json vite.config.ts vitest.setup.ts src src-tauri 项目提示词.txt
```

替换为：

```powershell
git add .gitignore package.json package-lock.json index.html tsconfig*.json vite.config.ts vitest.setup.ts src src-tauri
```

### 任务 3：验证批准后的仓库状态

**文件：**
- 验证不存在：`项目提示词.txt`
- 验证语言规则：`AGENTS.md`
- 验证决策不变：`docs/superpowers/specs/2026-07-20-project-workflow-design.md`
- 验证没有生成框架文件：`.trellis/`
- 验证差异：全部修改和删除的文件

- [x] **步骤 1：确认旧提示词文件已经不存在**

运行：

```powershell
Test-Path -LiteralPath '项目提示词.txt'
```

预期：输出 `False`。

- [x] **步骤 2：确认三个旧文档不再引用它**

运行：

```powershell
rg -n '项目提示词\.txt|项目提示词' `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md
if ($LASTEXITCODE -eq 1) { Write-Output 'No legacy references found'; exit 0 }
exit $LASTEXITCODE
```

预期：输出 `No legacy references found` 且退出码为 0。工作流决策文档和本实施计划为了保留删除原因与审计依据，可以提及被删除文件。

- [x] **步骤 3：确认简体中文要求已经成为仓库规则**

运行：

```powershell
rg -n '项目文档、设计文档、实施计划、验证报告和 README 默认使用简体中文' AGENTS.md
```

预期：输出 `AGENTS.md` 中新增的第 13 条规则并以退出码 0 结束。

- [x] **步骤 4：确认没有初始化 Trellis**

运行：

```powershell
Test-Path -LiteralPath '.trellis'
Test-Path -LiteralPath '.agents/skills'
```

预期：两个值都为 `False`。

- [x] **步骤 5：检查差异范围和空白字符**

运行：

```powershell
git diff --check
git status --short
git diff --stat
```

预期：`git diff --check` 退出码为 0。状态包含已删除的提示词、`AGENTS.md`、三个清理后的旧文档和本实施计划的执行记录；不得修改应用源码、依赖、包锁文件、Tauri 配置或生成任何 Trellis 文件。工作流设计的验收澄清已经在计划提交前单独提交，不属于本次未提交差异。

### 任务 4：提交清理结果

**文件：**
- 删除：`项目提示词.txt`
- 修改：`AGENTS.md`
- 修改：`docs/verification-report.md`
- 修改：`docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- 修改：`docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`
- 修改：`docs/superpowers/plans/2026-07-20-project-workflow-cleanup.md`

- [x] **步骤 1：暂存准确的批准范围**

运行：

```powershell
git add -A -- `
  '项目提示词.txt' `
  AGENTS.md `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md `
  docs/superpowers/plans/2026-07-20-project-workflow-cleanup.md
```

- [x] **步骤 2：验证暂存内容**

运行：

```powershell
git diff --cached --check
git diff --cached --stat
```

预期：空白字符检查退出码为 0；暂存文件与任务 4 的文件列表一致，不包含应用源码或 Trellis 生成文件。

- [x] **步骤 3：提交**

运行：

```powershell
git commit -m "chore: remove obsolete project prompt"
```

预期：在 `master` 分支提交成功。

- [x] **步骤 4：确认最终状态**

运行：

```powershell
git status --short
git log -3 --oneline
```

预期：工作树干净；最新提交为 `chore: remove obsolete project prompt`，之前的计划提交和工作流决策提交仍保留在历史中。
