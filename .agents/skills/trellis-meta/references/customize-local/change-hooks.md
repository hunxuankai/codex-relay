# 修改本地 Hook

Hook 是连接平台与 Trellis 的自动化层。当用户希望修改“何时注入上下文”“Shell 命令如何继承会话”或“Agent 启动前读取哪些文件”时，通常应编辑 hook。

## 首先读取这些文件

1. 目标平台的 settings/config，例如 `.claude/settings.json`、`.codex/hooks.json`、`.cursor/hooks.json`、`.trae/hooks.json`
2. 目标平台的 hook 目录
3. `.trellis/scripts/common/active_task.py`
4. `.trellis/scripts/common/session_context.py`
5. `.trellis/workflow.md`

## 常见 Hook 类型

| Hook | 用途 |
| --- | --- |
| session-start | 会话开始、清空或压缩时注入 Trellis 概览。 |
| workflow-state | 每次用户输入时注入状态提示。 |
| sub-agent context | Agent 启动前注入 PRD/spec/research。 |
| shell session bridge | 让 Shell 中的 `task.py` 命令看到同一个会话身份。 |

## 修改步骤

1. 在 settings/config 中找到 hook 注册。
2. 确认注册的脚本路径存在。
3. 读取 hook 脚本，识别输入、输出以及所调用的 `.trellis/scripts/`。
4. 修改 hook 行为。
5. 如果 hook 依赖工作流内容，同步 `.trellis/workflow.md`。

## 示例：修改新会话注入内容

首先找到 session-start hook：

```text
.claude/settings.json
.claude/hooks/session-start.py
```

如果 hook 最终调用 `.trellis/scripts/get_context.py` 或 `session_context.py`，编辑本地脚本通常比在 hook 中硬编码内容更稳健。

## 示例：Agent 没有读取 JSONL

首先确认：

```bash
python ./.trellis/scripts/task.py current --source
python ./.trellis/scripts/task.py validate <task>
```

如果任务和 JSONL 正确，判断平台使用 hook 推送还是 Agent 拉取。hook 推送型应编辑 `inject-subagent-context`；Agent 拉取型应编辑 Agent 文件。

## 注意事项

- Settings 负责注册，hook 脚本负责行为；应同时检查两者。
- 不同平台支持不同的 hook 事件。不要直接复制其他平台的 settings。
- Hook 应读取项目本地 `.trellis/`；不应依赖 Trellis 上游源码路径。
- Hook 失败时应产生可见错误，避免 AI 静默丢失上下文。
