# 本地工作流系统

`.trellis/workflow.md` 是用户项目内 Trellis 工作流的事实来源。AI 不需要 Trellis 源码就能理解当前项目应如何推进任务；该文件已经足够。

## 文件职责

`.trellis/workflow.md` 有三项职责：

1. **说明工作流阶段**：规划、实施、完成。
2. **定义 Skill 路由**：用户表达某种意图时，AI 应使用哪个 Skill 或代理。
3. **提供 workflow-state 提示块**：钩子可把当前状态对应的提示块注入对话。

## 当前阶段模型

```text
Phase 1: Plan    -> clarify what to build, produce prd.md and required research
Phase 2: Execute -> implement against the PRD and specs, then check
Phase 3: Finish  -> final verification, preserve lessons, and wrap up
```

每个阶段包含编号步骤，例如 `1.3 Configure context`。这些编号不是 `task.json` 中的运行时字段，而是供 AI 和人类阅读的工作流结构。

## Skill 路由

`workflow.md` 按平台能力区分路由：

- 支持子代理的平台：默认派发 `trellis-implement` 实施，派发 `trellis-check` 检查。
- 不支持子代理的平台：主会话读取 `trellis-before-dev` 等 Skill，然后直接执行。

修改本地 AI 行为时，先更新 `workflow.md` 中的路由说明，再检查对应平台 Skill、命令或代理文件是否需要同步。

## Workflow-State 提示块

`workflow.md` 底部可以包含如下状态块：

```text
[workflow-state:no_task]
...
[/workflow-state:no_task]
```

钩子根据当前任务状态选择正确的块并注入对话。常见状态包括：

| 状态 | 含义 |
| --- | --- |
| `no_task` | 当前会话没有活动任务。 |
| `planning` | 任务仍处于需求、研究或上下文配置阶段。 |
| `in_progress` | 任务已进入实施和检查阶段。 |
| `completed` | 任务已完成，等待收尾或归档。 |

如果用户希望修改“没有任务时是否创建任务”“何时可以跳过任务创建”或“是否必须使用子代理”等策略，应编辑这些状态块及其上方的路由表。

## 本地修改模式

常见改动：

| 目标 | 编辑位置 |
| --- | --- |
| 添加阶段 | 更新阶段索引、阶段正文、路由和状态块。 |
| 修改任务创建策略 | 更新 `no_task` 状态块和阶段 1 说明。 |
| 修改默认实施/检查路径 | 更新阶段 2 和 Skill 路由。 |
| 修改收尾流程 | 更新阶段 3 和 `finish-work` 相关说明。注意当前拆分：阶段 3.4 = AI 驱动代码提交（分批、经用户确认）；阶段 3.5 = `/finish-work`（归档 + 记录会话）。工作树有未提交内容时，`/finish-work` 会拒绝运行。 |
| 修改平台差异 | 更新按平台分组的路由说明。 |

编辑后让 AI 重新读取 `.trellis/workflow.md`；不要假设旧对话中的流程仍然有效。

## 与平台文件的关系

`workflow.md` 是本地工作流的语义中心，但每个平台也可能有自己的入口文件：

- Skill，例如 `trellis-brainstorm` 和 `trellis-check`。
- 命令/提示词/工作流，例如 continue 和 finish-work。
- 钩子，例如 session-start 或 workflow-state 注入。

如果只修改 `workflow.md`，平台入口文件可能仍包含旧说明。用户希望改变“AI 实际执行什么”时，也要检查相关平台目录。
