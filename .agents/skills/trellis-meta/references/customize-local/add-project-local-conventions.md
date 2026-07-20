# 添加项目本地约定

用户通常不需要修改 Trellis 机制，而是需要让本地 AI 理解团队约定。在这种情况下，优先使用 `.trellis/spec/` 或项目本地 Skill，而不是编辑 `trellis-meta`。

## 内容应放在哪里

| 内容类型 | 位置 |
| --- | --- |
| 代码必须遵循的规则 | `.trellis/spec/<layer>/` |
| 跨层思考方法 | `.trellis/spec/guides/` |
| 项目特定流程的 AI 能力 | 平台本地 Skill |
| 一次性任务材料 | `.trellis/tasks/<task>/` |
| 会话摘要 | `.trellis/workspace/<developer>/journal-N.md` |

## 创建项目本地 Skill

如果用户希望 AI 了解“本项目如何定制 Trellis”，请创建本地 Skill：

```text
.claude/skills/trellis-local/
└── SKILL.md
```

示例：

```md
---
name: trellis-local
description: "Project-local Trellis customizations for this repository. Use when changing this project's Trellis workflow, hooks, local agents, or team-specific conventions."
---

# Trellis Local

## Local Scope

This skill documents this repository's Trellis customizations only.

## Custom Workflow Rules

- ...

## Local Hook Changes

- ...

## Local Agent Changes

- ...
```

对于多平台项目，在其他平台的 Skill 目录中放置等价版本，或在支持共享层的平台上使用 `.agents/skills/`。

## 写入 `.trellis/spec/`

如果内容属于编码约定，请写入规范。例如：

```text
.trellis/spec/backend/error-handling.md
.trellis/spec/frontend/components.md
.trellis/spec/guides/cross-platform-thinking-guide.md
```

写入后更新对应的 `index.md`，使 AI 能从入口找到新规则。

## 让当前任务使用新约定

写入规范后，将其添加到当前任务上下文：

```bash
python ./.trellis/scripts/task.py add-context <task> implement ".trellis/spec/backend/error-handling.md" "Error handling conventions"
python ./.trellis/scripts/task.py add-context <task> check ".trellis/spec/backend/error-handling.md" "Review error handling"
```

## 不要在 `trellis-meta` 中存放项目私有规则

`trellis-meta` 是用于理解 Trellis 架构和本地定制入口的公共 Skill。项目私有内容应放在：

- `.trellis/spec/`
- 项目本地 Skill
- 当前任务
- 工作区日志

这样可防止 Trellis 内置 `trellis-meta` 的后续更新覆盖团队自己的约定。
