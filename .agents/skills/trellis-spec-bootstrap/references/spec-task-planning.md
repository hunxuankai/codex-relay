# 规范任务规划

默认使用单代理执行模型。代理可以创建 Trellis 任务以便追踪，但此 Skill 不应要求特定平台、CLI 或并行 Worker 模型。

## 拆分

围绕真实所有权边界创建规范工作单元：

- 如果一个包有自己的约定，则按一个包拆分。
- 如果同一包具有不同的前端、后端、CLI、Worker 或共享库规则，则按一层拆分。
- 如果模式跨包且不归属于单一层，则编写一个横切指南。

避免人为拆分。小型库通常只需要一次聚焦的规范处理，不需要多个任务。

## 任务结构

适合使用 Trellis 任务时，编写包含以下章节的简洁 PRD：

```markdown
# Fill <package-or-layer> Trellis Specs

## Goal
Write project-specific `.trellis/spec/` guidance for <scope>.

## Scope
- Spec directory:
- Source directories to inspect:
- Tests to inspect:
- Out of scope:

## Architecture Context
Summarize the concrete findings from repository analysis.

## Files To Create Or Update
- `.trellis/spec/.../index.md`
- `.trellis/spec/.../<topic>.md`

## Rules
- Adapt the spec file set to the real codebase.
- Use real source examples with file paths.
- Remove template-only sections that do not apply.
- Do not modify product source code unless the task explicitly asks for it.

## Acceptance Criteria
- [ ] Specs contain concrete examples and anti-patterns from the repository.
- [ ] No placeholder text remains.
- [ ] Index files match the final spec files.
- [ ] Claims are backed by source files, tests, or project docs.
```

## 可选辅助代理

如果宿主支持子代理，辅助代理可以检查独立包或运行验证。它们是可选项；主代理仍负责集成和最终质量。

辅助任务必须有明确所有权：

- 只读研究任务可以检查分配范围所需的任何源码。
- 写入任务应分别拥有互不重叠的规范目录。
- 验证任务应检查占位符清理、失效链接和一致性。

不要在 Skill 中编码辅助代理名称、供应商专属命令或平台专属路由。任务中只写必需工作和验收标准。
