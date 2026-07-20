# 本地上下文注入系统

Trellis 上下文注入旨在让 AI 在正确时间读取正确文件，而不是依赖模型记忆。在用户项目中，注入由 `.trellis/` 脚本与平台 hook、Agent 和 Skill 共同实现。

## 注入的上下文类型

| 类型 | 来源 | 用途 |
| --- | --- | --- |
| 会话上下文 | `.trellis/scripts/get_context.py` | 当前开发者、Git 状态、活动任务、全部活动任务、日志和包。 |
| 工作流上下文 | `.trellis/workflow.md` | 当前 Trellis 流程和下一步动作。 |
| spec 上下文 | `.trellis/spec/` + 任务 JSONL | 实施/检查期间必须遵循的 spec。 |
| 任务上下文 | `.trellis/tasks/<task>/prd.md`、`design.md`、`implement.md`、`research/` | 当前任务的需求、设计、实施计划和研究。 |
| 平台上下文 | 平台 hook/settings/Agent | 让不同 AI 工具通过各自机制读取上述文件。 |

## session-start

支持 session-start 的平台会在会话开始、清空、压缩或收到类似事件时注入 Trellis 概览。注入内容通常包括：

- 工作流摘要。
- 当前任务状态。
- 活动任务。
- spec 索引路径。
- 开发者身份和 Git 状态。

如果用户发现 AI 在新会话中不知道当前任务，首先检查平台的 session-start hook 或等价机制是否已安装并运行。

## workflow-state

workflow-state 是在每个用户轮次附近注入的轻量提示。它根据当前任务状态，从 `.trellis/workflow.md` 选择 `no_task`、`planning`、`in_progress` 或 `completed` 等块。

如果用户希望修改“AI 在某个状态下下一步应该做什么”，首先编辑 `.trellis/workflow.md` 中对应的状态块。

## 子 Agent 上下文

Implement 和 check Agent 需要任务上下文。Trellis 有两种加载模式：

1. **hook 推送**：平台 hook 在 Agent 启动前注入 jsonl 引用的文件，以及 `prd.md`、存在时的 `design.md` 和存在时的 `implement.md`。
2. **Agent 拉取**：Agent 定义指示 Agent 在启动后读取活动任务、jsonl 上下文和任务产物。

在两种模式中，任务目录里的 JSONL 文件都是 spec/research 上下文清单。任务产物按以下顺序单独读取：`prd.md` → `design.md if present` → `implement.md if present`。

## JSONL 读取规则

`implement.jsonl` 和 `check.jsonl` 每行包含一个 JSON 对象：

```jsonl
{"file": ".trellis/spec/backend/index.md", "reason": "Backend rules"}
```

读取器应跳过不含 `file` 字段的种子行。配置 JSONL 时，AI 只应包含 spec/research 文件，不要预先登记将要修改的代码文件。

## 活动任务与上下文键

活动任务状态位于 `.trellis/.runtime/sessions/`，并按会话隔离。Hook 尝试从平台事件、环境变量、对话记录路径或 `TRELLIS_CONTEXT_ID` 解析上下文键。

如果 Shell 命令看不到相同的上下文键，`task.py current --source` 可能报告没有活动任务。此时应检查平台是否把会话身份传入 Shell，而不是手工编写全局当前任务文件。

## 本地定制点

| 需求 | 编辑位置 |
| --- | --- |
| 修改 session-start 注入内容 | 平台的 `session-start` hook 或插件文件。 |
| 修改逐轮 workflow-state 规则 | `.trellis/workflow.md` 中的 `[workflow-state:STATUS]` 块。平台 workflow-state hook 会逐字解析这些块，不内嵌回退文本。 |
| 修改子 Agent 读取上下文的方式 | 平台 Agent 定义、`inject-subagent-context` hook 或 Agent 前置指令。 |
| 修改 JSONL 验证/显示 | `.trellis/scripts/common/task_context.py`。 |
| 修改活动任务解析 | `.trellis/scripts/common/active_task.py`。 |

修改上下文注入时，应验证两点：新会话能够看到正确任务；子 Agent 能够看到正确的任务产物/spec/research。
