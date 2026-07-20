# 修改本地上下文加载

上下文加载决定 AI 何时读取工作流、任务、规范、研究、工作区和 Git 状态。当用户表示“AI 不知道当前任务”“Agent 没有读取规范”或“上下文太多/太少”时，阅读本页。

## 编辑前先读取这些文件

1. `.trellis/workflow.md`
2. `.trellis/scripts/get_context.py`
3. `.trellis/scripts/common/session_context.py`
4. `.trellis/scripts/common/task_context.py`
5. `.trellis/scripts/common/active_task.py`
6. 当前平台的 hook 或 Agent 文件
7. 当前任务的 `implement.jsonl` / `check.jsonl`

## 上下文来源

| 来源 | 用途 |
| --- | --- |
| `.trellis/workflow.md` | 工作流和下一步操作提示。 |
| `.trellis/tasks/<task>/prd.md` | 当前任务需求。 |
| `.trellis/tasks/<task>/design.md` | 复杂任务的技术设计。 |
| `.trellis/tasks/<task>/implement.md` | 复杂任务的实施计划。 |
| `.trellis/tasks/<task>/implement.jsonl` | 实施前需要读取的规范/研究资料。 |
| `.trellis/tasks/<task>/check.jsonl` | 检查时需要读取的规范/研究资料。 |
| `.trellis/spec/` | 项目规范。 |
| `.trellis/workspace/` | 会话记录。 |
| Git 状态 | 当前工作树改动。 |

## 常见需求与编辑位置

| 需求 | 编辑位置 |
| --- | --- |
| 在新会话中注入更多/更少信息 | `session_context.py` 或平台的 `session-start` hook。 |
| 修改每次用户输入时的提示 | `.trellis/workflow.md` 中的 `[workflow-state:STATUS]` 块。`inject-workflow-state` hook 只负责解析，并按原文读取该块。 |
| Agent 未读取规范 | 任务 JSONL、Agent 前置指令、`inject-subagent-context` hook。 |
| 活动任务丢失 | `active_task.py` 与平台会话身份传播。 |
| 修改 JSONL 验证规则 | `task_context.py`。 |

## JSONL 规则

`implement.jsonl` / `check.jsonl` 是关键的上下文加载接口：

```jsonl
{"file": ".trellis/spec/backend/index.md", "reason": "Backend conventions"}
{"file": ".trellis/tasks/04-28-x/research/api.md", "reason": "API research"}
```

其中只应包含规范/研究文件。不要把即将修改的代码文件放入这些清单；Agent 会在实施期间自行读取代码文件。

## 修改会话上下文

如果用户希望每个新会话都能看到更多项目状态，请编辑：

- `.trellis/scripts/common/session_context.py`
- 对应平台的 `session-start` hook

上下文不能无限增长。优先注入索引和路径，让 AI 按需读取详细文件。

## 修改子 Agent 上下文

首先确定平台使用哪种模式：

- hook 推送：编辑 `inject-subagent-context` hook。
- Agent 拉取：编辑对应 `trellis-implement` / `trellis-check` Agent 文件中的读取步骤。

两种模式都必须确保 Agent 最终读取：

1. 活动任务
2. 对应的 JSONL
3. JSONL 引用的规范/研究文件
4. `prd.md`
5. 存在时读取 `design.md`
6. 存在时读取 `implement.md`

## 故障排查顺序

```bash
python ./.trellis/scripts/task.py current --source
python ./.trellis/scripts/task.py list-context <task>
python ./.trellis/scripts/task.py validate <task>
python ./.trellis/scripts/get_context.py --mode packages
```

编辑 hook/Agent 前，先确认任务和 JSONL 正确。
