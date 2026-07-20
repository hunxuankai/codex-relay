# Project Workflow Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 删除已经完成使命的 `项目提示词.txt`，清理旧文档引用，并保持当前 `AGENTS.md + Superpowers + docs + Git` 工作流而不引入 Trellis。

**Architecture:** 这是一次纯仓库文档整理，不修改应用源码、依赖或运行行为。先记录当前文件与三处旧引用作为失败基线，再删除文件、精确改写三处文档，最后验证旧引用消失、`.trellis/` 未创建且 Git diff 只包含批准范围。

**Tech Stack:** Git、Markdown、PowerShell、ripgrep

---

### Task 1: Record the cleanup baseline

**Files:**
- Inspect: `项目提示词.txt`
- Inspect: `docs/verification-report.md`
- Inspect: `docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- Inspect: `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`

- [ ] **Step 1: Confirm the obsolete file still exists**

Run:

```powershell
Test-Path -LiteralPath '项目提示词.txt'
```

Expected: `True`. This is the pre-change failure against the approved requirement that the obsolete file be absent.

- [ ] **Step 2: Confirm the three legacy references still exist**

Run:

```powershell
rg -n '项目提示词\.txt|项目提示词' `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md
```

Expected: three matches: the release signing note, the original completion definition, and the historical `git add` command.

### Task 2: Delete the obsolete source and clean legacy references

**Files:**
- Delete: `项目提示词.txt`
- Modify: `docs/verification-report.md`
- Modify: `docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- Modify: `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`

- [ ] **Step 1: Delete the obsolete prompt file**

Apply this patch:

```diff
*** Begin Patch
*** Delete File: 项目提示词.txt
*** End Patch
```

- [ ] **Step 2: Replace the release-report reference with the durable release requirement**

In `docs/verification-report.md`, replace:

```markdown
- 代码签名未完成：仓库没有发布证书或签名授权；项目提示词要求在正式发布前说明签名，
  未把实际签名列为本次最终验收条件。
```

with:

```markdown
- 代码签名未完成：仓库没有发布证书或签名授权；正式发布要求需要说明签名状态，未把实际
  签名列为本次最终验收条件。
```

- [ ] **Step 3: Replace the original design's dependency on the deleted prompt**

In `docs/superpowers/specs/2026-07-20-codex-relay-design.md`, replace the completion-definition paragraph with:

```markdown
完成必须同时满足已批准设计和项目验收要求：Provider CRUD 与切换可用；主窗口和托盘共享后端；配置写入保留无关内容并可回滚；自检、监控、单实例、自启动、通知和托盘行为实现；测试不触碰真实数据；文档齐全；所有检查、Debug、Release 和 NSIS 构建都实际成功；最终报告逐项列出产物、命令结果、已知限制和人工验证步骤。
```

- [ ] **Step 4: Remove the deleted file from the historical initialization command**

In `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`, replace:

```powershell
git add .gitignore package.json package-lock.json index.html tsconfig*.json vite.config.ts vitest.setup.ts src src-tauri 项目提示词.txt
```

with:

```powershell
git add .gitignore package.json package-lock.json index.html tsconfig*.json vite.config.ts vitest.setup.ts src src-tauri
```

### Task 3: Verify the approved repository state

**Files:**
- Verify absence: `项目提示词.txt`
- Verify unchanged decision: `docs/superpowers/specs/2026-07-20-project-workflow-design.md`
- Verify no generated framework: `.trellis/`
- Verify diff: all modified and deleted files

- [ ] **Step 1: Confirm the obsolete file is absent**

Run:

```powershell
Test-Path -LiteralPath '项目提示词.txt'
```

Expected: `False`.

- [ ] **Step 2: Confirm the legacy documents no longer reference it**

Run:

```powershell
rg -n '项目提示词\.txt|项目提示词' `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md
if ($LASTEXITCODE -eq 1) { Write-Output 'No legacy references found'; exit 0 }
exit $LASTEXITCODE
```

Expected: `No legacy references found` and exit code 0. The workflow decision spec and this implementation plan are intentionally excluded because they preserve the reason and evidence for the deletion.

- [ ] **Step 3: Confirm Trellis was not initialized**

Run:

```powershell
Test-Path -LiteralPath '.trellis'
Test-Path -LiteralPath '.agents/skills'
```

Expected: both values are `False`.

- [ ] **Step 4: Check diff scope and whitespace**

Run:

```powershell
git diff --check
git status --short
git diff --stat
```

Expected: `git diff --check` exits 0. Status contains the deleted prompt, the three cleaned legacy documents, and the approved workflow design clarification; no application source, dependency, package lock, Tauri configuration, or generated Trellis file is modified.

### Task 4: Commit the cleanup

**Files:**
- Delete: `项目提示词.txt`
- Modify: `docs/verification-report.md`
- Modify: `docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- Modify: `docs/superpowers/specs/2026-07-20-project-workflow-design.md`
- Modify: `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`
- Add: `docs/superpowers/plans/2026-07-20-project-workflow-cleanup.md`

- [ ] **Step 1: Stage the exact approved scope**

Run:

```powershell
git add -A -- `
  '项目提示词.txt' `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/specs/2026-07-20-project-workflow-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md `
  docs/superpowers/plans/2026-07-20-project-workflow-cleanup.md
```

- [ ] **Step 2: Verify the staged change**

Run:

```powershell
git diff --cached --check
git diff --cached --stat
```

Expected: whitespace check exits 0; staged files match the list in Task 4 and contain no application code or Trellis-generated files.

- [ ] **Step 3: Commit**

Run:

```powershell
git commit -m "chore: remove obsolete project prompt"
```

Expected: commit succeeds on `master`.

- [ ] **Step 4: Confirm final state**

Run:

```powershell
git status --short
git log -2 --oneline
```

Expected: working tree is clean. The newest commit is `chore: remove obsolete project prompt`; the preceding workflow decision commit remains in history.
