# 修改本地工作流

当用户希望修改 Trellis 阶段、下一步提示、是否创建任务、是否使用子 Agent，或何时检查/收尾时，首先编辑 `.trellis/workflow.md`。

## 首先读取这些文件

1. `.trellis/workflow.md`
2. 当前平台的入口文件，例如 Skill/命令/提示词/工作流
3. 当前任务的 `task.json` 和 `prd.md`

## 常见需求与编辑位置

| 需求 | 编辑位置 |
| --- | --- |
| 修改阶段名称或阶段顺序 | `Phase Index` 和对应的 Phase 章节。 |
| 修改无任务时是否创建任务 | `[workflow-state:no_task]` 状态块。 |
| 修改规划阶段的下一步 | Phase 1 和 `[workflow-state:planning]`。 |
| 修改 in_progress 期间是否必须使用 Agent | Phase 2 和 `[workflow-state:in_progress]`。 |
| 修改完成后的收尾 | Phase 3 和 `[workflow-state:completed]`。 |
| 修改某种用户意图触发哪个 Skill | `Skill Routing` 表。 |

## 修改步骤

1. 在 `.trellis/workflow.md` 中找到相关章节。
2. 修改规则时，保留明确的触发条件和下一步动作。
3. 添加或重命名 Skill/Agent 时，同步平台目录中的对应文件。
4. 修改 workflow-state 时，只需编辑 `.trellis/workflow.md` 中的 `[workflow-state:STATUS]` 块。hook 只负责解析，会读取块中写入的任何内容。开始和结束标签的 STATUS 字符串必须一致（`[workflow-state:foo]…[/workflow-state:foo]`）；不匹配的 STATUS 对会被静默丢弃。
5. 让 AI 重新读取 `.trellis/workflow.md`；不要继续使用旧对话中的规则。

## 示例：放宽任务创建要求

要修改何时可以跳过任务创建，通常编辑 `[workflow-state:no_task]`：

```md
[workflow-state:no_task]
Task is not required when the answer is a one-reply explanation, no files are changed, and no research is needed.
[/workflow-state:no_task]
```

如果正式的 Phase 1 流程也需要变化，同步 Phase 1 章节。

## 示例：某个平台不使用子 Agent

如果用户只希望一个平台避免使用子 Agent，首先确认该平台在工作流中是否有独立分组。然后修改该平台分组的 Phase 2 路由，而不是删除所有平台的 `trellis-implement` / `trellis-check` 指令。

## `/trellis:continue` 路由表

`/trellis:continue` 通过判断下一步应加载哪个阶段步骤来恢复任务。判断依据结合 `task.json.status` 与任务目录中的产物是否存在。映射固定在命令本身；添加自定义状态的 fork 必须同时扩展 workflow.md 标签块和本表。

| `status` | 产物状态 | 恢复位置 |
| --- | --- | --- |
| `planning` | 缺少 `prd.md` | Phase 1.1（加载 `trellis-brainstorm`） |
| `planning` | 轻量任务的 `prd.md` 已完成 | 请求启动审查，然后运行 `task.py start` |
| `planning` | 复杂任务缺少 `design.md` 或 `implement.md` | 补齐缺少的规划产物 |
| `planning` | 复杂任务已有 `prd.md`、`design.md` 和 `implement.md` | 请求启动审查，然后运行 `task.py start` |
| `in_progress` | 对话历史中尚无实施 | Phase 2.1（`trellis-implement`） |
| `in_progress` | 实施完成，尚未运行 `trellis-check` | Phase 2.2（`trellis-check`） |
| `in_progress` | 检查通过 | Phase 3.3（更新 spec）→ 3.4（提交） |
| `completed` | 任务仍在活动任务树中 | Phase 3.5（运行 `/trellis:finish-work` 归档） |

添加自定义状态（例如 `in-review`）时，应在 `.trellis/workflow.md` 中添加 `[workflow-state:in-review]` 块作为逐轮提示，**并且**扩展本路由表——通常通过编辑 `/trellis:continue` 命令文件（`.{platform}/commands/trellis/continue.md` 或等价文件），增加一行来决定从哪里恢复。缺少路由项时，`/trellis:continue` 会落入默认分支，用户无法进入预期步骤。

## 注意事项

`.trellis/workflow.md` 是本地项目工作流，不是不可变模板。用户可以按团队习惯调整它。编辑后，平台入口文件可能仍包含旧说明，因此也要检查这些文件。
