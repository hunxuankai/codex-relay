# 修改本地任务生命周期

任务生命周期包括创建、启动、上下文配置、完成、归档、父/子任务和生命周期 hook。默认定制目标是 `.trellis/tasks/`、`.trellis/config.yaml` 和 `.trellis/scripts/`。

## 编辑前先读取这些文件

1. `.trellis/workflow.md`
2. `.trellis/config.yaml`
3. `.trellis/scripts/task.py`
4. `.trellis/scripts/common/task_store.py`
5. `.trellis/scripts/common/task_utils.py`
6. 当前任务的 `.trellis/tasks/<task>/task.json`

## 常见需求与编辑位置

| 需求 | 编辑位置 |
| --- | --- |
| 创建任务后自动同步外部系统 | `.trellis/config.yaml` 中的 `hooks.after_create`。 |
| 启动任务后自动更新状态 | `.trellis/config.yaml` 中的 `hooks.after_start`。 |
| 完成任务后运行脚本 | `.trellis/config.yaml` 中的 `hooks.after_finish`。 |
| 归档后清理外部资源 | `.trellis/config.yaml` 中的 `hooks.after_archive`。 |
| 修改默认任务字段 | `.trellis/scripts/common/task_store.py`。 |
| 修改任务解析/搜索 | `.trellis/scripts/common/task_utils.py`。 |
| 修改活动任务行为 | `.trellis/scripts/common/active_task.py`。 |

## 生命周期 hook

`.trellis/config.yaml` 支持：

```yaml
hooks:
  after_create:
    - "python .trellis/scripts/hooks/my_sync.py create"
  after_start:
    - "python .trellis/scripts/hooks/my_sync.py start"
  after_finish:
    - "python .trellis/scripts/hooks/my_sync.py finish"
  after_archive:
    - "python .trellis/scripts/hooks/my_sync.py archive"
```

Hook 命令会收到指向当前任务 `task.json` 的 `TASK_JSON_PATH` 环境变量。Hook 失败通常应给出警告，但不阻塞主要任务操作。

## 修改任务字段

如果用户希望添加项目本地字段，优先将其放在 `task.json` 的 `meta` 下，以免破坏现有脚本对标准字段的假设。

示例：

```json
"meta": {
  "linearIssue": "ENG-123",
  "risk": "high"
}
```

如果确实需要修改标准字段，请检查每个读取 `task.json` 的本地脚本。

## 修改活动任务

活动任务是存储在 `.trellis/.runtime/sessions/` 中的会话级状态。不要退回到全局 `.current-task` 模型。如果用户希望修改活动任务行为，请编辑：

- `.trellis/scripts/common/active_task.py`
- 平台 hook 或 Shell 会话桥接
- `.trellis/workflow.md` 中的活动任务说明

### `task.py create` 设置活动指针

`.trellis/scripts/common/task_store.py` 中的 `cmd_create` 在写入新任务目录后立即尽力调用 `set_active_task`。具体行为如下：

- 当调用 Shell 带有会话身份（`TRELLIS_CONTEXT_ID` 环境变量，或 `resolve_context_key` 能识别的任意平台专属会话环境变量，参见 `active_task.py:_ENV_SESSION_KEYS`）时，`.trellis/.runtime/sessions/<context_key>.json` 中的会话指针会改为指向新任务。任务写入 `status=planning`，并在紧接着的下一次 `UserPromptSubmit` 触发 `[workflow-state:planning]`。
- 当会话身份不可用时（在 AI 会话外直接调用 CLI，或平台未向 Shell 传播身份），仍会创建任务目录并写入 `status=planning`，但不会改动活动指针。用户返回 AI 会话后，可以通过 `task.py start <dir>` 关联该任务。

这样一来，`[workflow-state:planning]` 会成为 `task.py create` 之后需求探索和 JSONL 整理期间的实时路标。R7 之前的行为会让路标一直停留在 `no_task`，直到运行 `task.py start`，因此规划块实际上是无法触发的文本。

如果 fork `task.py` 以添加新的创建路径（例如绕过 `cmd_create` 的外部导入），请审计该路径是否也调用 `set_active_task`。没有该调用，创建出的任务不会显示为活动状态。完整的状态写入方表位于 `.trellis/spec/cli/backend/workflow-state-contract.md`。

## 修改步骤

1. 使用 `python ./.trellis/scripts/task.py current --source` 确认当前任务。
2. 读取当前任务的 `task.json`，确认状态和字段。
3. 配置需求先编辑 `.trellis/config.yaml`。
4. 脚本行为需求再编辑 `.trellis/scripts/`。
5. 如果 AI 流程发生变化，同步 `.trellis/workflow.md`。

## 禁止事项

- 不要直接编辑 `.trellis/.runtime/sessions/` 来“修复”业务状态。
- 不要在脚本中硬编码项目私有字段；优先使用 `meta`。
- 不要默认要求用户 fork Trellis CLI。
