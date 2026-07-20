# `trellis mem` CLI 参考

五个子命令的完整 flag 参考。将本文档视为权威来源；`trellis mem help` 在运行时输出相同内容，因此这里的任何漂移都属于缺陷。

## 子命令

| 命令                   | 用途                                                                                                                   |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `list`                 | 列出会话。未指定子命令时的默认命令。                                                                                   |
| `search <keyword>`     | 查找内容匹配关键词的会话。                                                                                             |
| `context <session-id>` | 深入查看一个会话：命中度最高的 N 个轮次及周边上下文。配合 `--grep` 锚定关键词。                                        |
| `extract <session-id>` | 导出清理后的对话。结合 `--phase` / `--grep` 切片。                                                                     |
| `projects`             | 列出活动项目的 `cwd` 值和会话数量。用它发现传给其他子命令的 `--cwd`。                                                  |

## Flag（在有意义的子命令中适用）

| Flag                                          | 子命令            | 含义                                                                                                                                                       |
| --------------------------------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--platform claude\|codex\|opencode\|pi\|all` | 全部              | 默认 `all`。OpenCode 适配器在 `0.6.0-beta.*` 中仍是桩实现，见下方“注意事项”。                                                                                |
| `--since YYYY-MM-DD`                          | list / search     | 包含边界的起始日期。                                                                                                                                       |
| `--until YYYY-MM-DD`                          | list / search     | 包含边界的结束日期。                                                                                                                                       |
| `--global`                                    | list / search     | 包含本机所有项目的会话。默认只包含当前项目 `cwd`。                                                                                                         |
| `--cwd <path>`                                | list / search     | 强制指定项目 cwd，不从当前位置推断。                                                                                                                       |
| `--limit N`                                   | list / search     | 限制输出行数。默认 `50`。                                                                                                                                  |
| `--grep KW`                                   | extract / context | 按关键词筛选轮次。多个空格分隔 token 使用 AND 匹配。                                                                                                       |
| `--phase brainstorm\|implement\|all`          | extract           | 按 Trellis 任务边界切片。`brainstorm` = `[task.py create, task.py start)`；`implement` = 需求探索窗口外的轮次。默认 `all`。                                  |
| `--turns N`                                   | context           | 返回的命中轮次数。默认 `3`。                                                                                                                               |
| `--around N`                                  | context           | 每个命中包含的周边轮次数。默认 `1`。                                                                                                                       |
| `--max-chars N`                               | context           | 总字符预算。默认 `6000`（约 1500 token）。                                                                                                                 |
| `--include-children`                          | search / context  | 把 OpenCode 子代理会话合并到父会话。                                                                                                                       |
| `--json`                                      | 全部              | 输出机器可解析 JSON，而不是人类可读输出。                                                                                                                  |

## 常用单行命令

```bash
# What past sessions discussed "deadlock" anywhere on this machine?
trellis mem search "deadlock" --global --limit 20

# Inside a specific session, surface the top 5 turns that mention "lock contention"
# plus 2 turns of surrounding context.
trellis mem context 5842592d --grep "lock contention" --turns 5 --around 2

# Recover the brainstorm window for a session — useful when continuing a task
# the user started a week ago.
trellis mem extract 5842592d --phase brainstorm

# List every project this machine has Trellis sessions for, with counts.
trellis mem projects
```

## 输出形式

- **默认人类可读输出**（不带 `--json`）：按终端宽度换行，高亮会话 ID，并显示轮次标记。适合直接阅读，但粘贴到 Markdown 文件中较杂乱。
- **`--json`**：稳定 Schema，可安全解析和处理。把 `mem` 输出传给后续步骤时（例如汇总为“经验”章节），优先使用 `--json`。

## 注意事项

- **OpenCode 适配器在 `0.6.0-beta.*` 中仍是桩实现。** 当 `--platform` 解析为 OpenCode（或 `all` 会包含 OpenCode）时，`mem` 输出一行“读取器不可用”提示，然后继续处理其他平台。适配器发布前，不要在回复中承诺覆盖 OpenCode。
- **`--phase` 切片依赖会话记录的 Bash 调用中出现 `task.py create` / `task.py start`。** 如果用户在不同终端、已记录 AI 循环之外运行 `task.py`，会话就没有阶段边界。`--phase all` 是安全兜底。
- **`mem` 直接索引平台 JSONL 文件。** 如果用户已清理 Claude / Codex / Pi 会话存储，`mem` 无法恢复磁盘上已不存在的内容。
- **`mem` 只读。** 不进行远端同步，也不编辑平台 JSONL。基于 `mem` 发现进行的任何写入，都是你后续调用可用编辑工具的行为。

## 需要更多信息时

在用户 Shell 中运行 `trellis mem help`。运行时帮助是权威来源，在快速迭代的 Beta 版本中可能领先于本文档。
