# 进度与调试

美化输出面向操作者，原始输出才是审计日志。子命令（`forum`、`thread`、
`messages`、`context`）是审计*接口*；手动搜索 `events.jsonl` 前应先使用它们。

## 美化输出与 `--raw`

`trellis channel messages <channel>` 呈现紧凑、易读的视图，包括时间戳、
身份、kind 和简短正文。它供操作者快速浏览 channel，不用于诊断。

美化输出可能且确实会截断：

- 较长的 progress 增量（`text_delta`、部分工具参数）
- 工具名称和命令行
- 多行状态字段和结构化 `detail` 数据
- 超出列宽预算的 forum thread 标题

发现异常时，例如 Worker 看似卡住、progress 行在单词中间结束、action 字段显示
`...`，请切换到 `--raw`。原始模式严格按照 `events.jsonl` 中的内容，每行输出
一个 JSON 事件，不会丢弃信息。

```bash
# Pretty (operator view)
trellis channel messages <channel> --kind done --last 10
trellis channel messages <channel> --kind error --last 10

# Raw (diagnostic view) — one JSON per line
trellis channel messages <channel> --raw --kind progress --last 20
trellis channel messages <channel> --raw --last 50
```

经验法则：绝不要根据被截断的 progress 行诊断 Worker。

### 重建流式文本

要重建模型在一个 turn 中实际流式输出的内容，请连接 progress 事件中的
`detail.text_delta`：

```bash
trellis channel messages <channel> --raw --kind progress --last 80 \
  | python -c 'import json,sys; [print((json.loads(l).get("detail") or {}).get("text_delta",""), end="") for l in sys.stdin if l.strip()]'
```

## 诊断卡住的 Worker

症状：`trellis channel list` 显示 Worker 正在运行，但 `messages` 中没有新事件，
且 `wait` 持续超时。

排查顺序：

1. **定位 channel 文件。** 如果不确定 channel 位于哪个存储桶，使用
   `list --all --all-projects`。

   ```bash
   trellis channel list --all --all-projects
   CHAN=~/.trellis/channels/<bucket>/<channel>
   ```

2. **确认 Supervisor 与 Worker PID 仍存活。**

   ```bash
   cat "$CHAN/<worker>.pid"            # supervisor PID
   cat "$CHAN/<worker>.worker-pid"     # actual CLI subprocess PID
   ps -p "$(cat "$CHAN/<worker>.pid")"
   ps -p "$(cat "$CHAN/<worker>.worker-pid")"
   ```

   如果 Supervisor PID 已消失，但 channel 仍列出 Worker，说明存在残留条目；
   使用 `trellis channel kill <name> --as <worker> --force` 清理。

3. **持续查看 Worker 日志。** Provider / MCP / 工具启动输出如果未进入 channel，
   应以此处为标准查看位置。

   ```bash
   tail -f "$CHAN/<worker>.log"
   ```

4. **检查最近的原始事件。** Worker 发出 `progress` 但没有 `message`/`done` 时，
   通常仍在流式输出，或阻塞于工具调用：

   ```bash
   trellis channel messages <channel> --raw --last 50
   ```

“存活但无输出”的常见原因：

- Provider 在首个 token 前冷启动，耗时较长但最终会继续。
- MCP Server 在启动期间阻塞，可在 Worker 日志中看到。
- Worker 正在等待已挂起子进程的工具结果。
- Prompt 过大或模型受到速率限制；检查 Worker 日志中的 Provider 端错误。

## 解读 Progress 事件

`progress` 事件表示正在进行的一项工作。其形态随 `action` 字段变化，但关键字段
始终位于 `detail` 下：

- `detail.text_delta`：模型增量输出；跨事件连接可重建流式回复。
- `detail.tool_name`、`detail.tool_input`：即将运行或正在运行的工具调用。
- `detail.status`：长时间运行操作使用的短字符串（`starting`、`running`、
  `flushing`、`done`）。
- `detail.action`：语义标签，例如 thread 心跳使用的 `status`。

Progress 事件按设计具有较多噪声。除非传入 `--include-progress`，否则 `wait`
会忽略它们。确实需要查看时，优先使用：

```bash
trellis channel messages <channel> --raw --kind progress --last 80
```

如果事件流稳定发出 progress，却始终没有以 `done`/`error`/`message` 收尾，
通常表示工具调用已挂起；请在 Worker 日志中检查子进程。

## Wait 语义速查

`channel wait` 从 `events.jsonl` 末尾开始监视，默认由以下事件唤醒：

- `message`
- `done`
- `error`
- `killed`
- `progress`（仅当传入 `--include-progress`）

常用过滤方式：

```bash
trellis channel wait T --as main --from check --kind done --timeout 15m
trellis channel wait T --as main --from check,check-cx --kind done --all --timeout 15m
trellis channel wait T --as worker --kind interrupt_requested --timeout 1h
trellis channel wait T --as main --thread release-note --action status --timeout 10m
```

退出码：`0` 表示匹配，`124` 表示超时，`1`/`2` 表示错误。`wait --all` 超时时，
stderr 会列出仍未响应的 Worker。

## 审计 `events.jsonl`：使用子命令，不要使用 `grep`

每个 channel 都将完整历史持久化到 `$CHAN/events.jsonl`。调试时很容易直接对
该文件使用 `tail` / `grep` / `jq`，但不要形成习惯，且对 forum channel
**绝不能**这样做。

优先使用子命令的原因：

- `messages` 已使用过滤条件（`--kind`、`--from`、`--last`、`--thread`、
  `--action`）重放文件，并提供 `--raw` 获取精确 JSON。通常无需另写单行命令。
- `wait` 以 EOF 语义读取同一文件；用 `tail -f | jq` 重新实现会在高负载下
  丢失事件，并在轮转时打乱顺序。
- `forum`、`thread` 和 `context list` 会使用内置 reducer 投影当前状态，
  手写过滤无法可靠复现这些规则。

### Forum Channel：绝不要直接解析 `events.jsonl`

Forum channel 将多个逻辑 thread 复用到单个 `events.jsonl`。每个事件都带有
`thread` 和 `action` 字段，forum 子命令知道如何归并它们。手动解析文件会：

- 混合不同 thread，使单个 thread 看起来不连贯。
- 遗漏会改变后续事件解释方式的 thread 生命周期事件（open / status / close）。
- 忽略 Worker inbox cursor，从而“看到” Worker 已消费的事件并误判为待处理。

改用理解 forum 语义的视图：

```bash
# List logical threads inside the forum channel
trellis channel forum <channel>

# Inspect one thread end-to-end
trellis channel thread <channel> <thread>

# Replay messages for a thread (supports --raw, --kind, --last)
trellis channel messages <channel> --thread <thread> --raw --last 100

# Channel/thread durable context (not a worker inbox projection)
trellis channel context list <channel> --thread <thread> --raw
```

CLI 当前不提供“Worker 尚有哪些事件待处理”的 inbox 投影命令。如果确实要诊断
待处理状态，应先用 `messages --to <worker> --raw` 检查定向事件；只有怀疑 CLI
本身有问题时，才直接读取 `events.jsonl`，例如确认事件是否真正持久化，或在
调试 Supervisor 时与 `<worker>.inbox-cursor` 对照。

## 常见故障

| 症状 | 原因 | 处理方式 |
|---|---|---|
| `trellis: command not found` | CLI 未全局安装 | `npm install -g @mindfoldhq/trellis` |
| `wait` 立即退出 | 过滤条件错误或身份冲突 | 使用不同的 `--as`，检查原始消息 |
| zsh 对消息文本报错 | Shell 解释了标点 | 使用 `--stdin` 或 `--text-file` |
| progress 行被截断 | 美化输出截断 | 使用 `messages --raw --kind progress` |
| Worker 始终不发言 | Provider 启动 / prompt / MCP 延迟 | 检查 `<worker>.log`、`ps` 和原始事件 |
| 在另一个 cwd 中找不到 channel | 项目存储桶不匹配 | `cd` 到项目、使用 `--scope global` 或 `list --all-projects` |
| 列表中有残留 Worker | Supervisor 退出但未清理 | `trellis channel kill <name> --as <worker> --force` |
| forum thread 看起来混乱 | 直接解析了 `events.jsonl` | 使用 `forum`、`thread`、`messages --thread` |

## 存储布局

```text
~/.trellis/channels/
└── <bucket>/
    └── <channel-name>/
        ├── events.jsonl
        ├── <channel>.lock
        ├── <worker>.log
        ├── <worker>.pid
        ├── <worker>.worker-pid
        ├── <worker>.config
        ├── <worker>.session-id
        ├── <worker>.thread-id
        ├── <worker>.inbox-cursor
        └── <worker>.spawnlock
```

Agent 通常应使用 CLI，而不是直接读取文件。只有 CLI 视图不足以调试时，才直接
读取文件；即便如此，也绝不能直接读取 forum channel 的 `events.jsonl`。
