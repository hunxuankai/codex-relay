# 命令参考

本页是当前 `trellis channel` 子命令的权威参考，已根据
`packages/cli/src/commands/channel/` 中的源码（`index.ts` Commander 连接
以及各子命令处理器）验证。

除非另有说明，每个子命令都接受 `--scope <project|global>`；默认值为
`project`，并根据当前 cwd 解析项目存储桶。

## 顶层命令

```
trellis channel <subcommand>
```

> 多 Agent 协作运行时：通过共享事件日志启动、协调和中断 Worker Agent。

---

## 创建/列出

### `create <name>`

```bash
trellis channel create <name>
  [--scope project|global]                # default: project
  [--type chat|forum]                     # default: chat
  [--task <path>]                         # associated Trellis task dir
  [--project <slug>]
  [--labels a,b,c]
  [--description <text>]                  # stable channel description
  [--context-file <abs-path>] ...         # repeatable
  [--context-raw  <text>]      ...        # repeatable
  [--linked-context-file <abs-path>]      # [deprecated alias]
  [--linked-context-raw  <text>]          # [deprecated alias]
  [--cwd <path>]                          # recorded in create event
  [--by <agent>]                          # default: main
  [--force]                               # overwrite existing channel
  [--ephemeral]                           # hide from default list, prunable
```

行为：
- 追加 `create` 事件；`type` 不可变，之后不能在 forum 与 chat 之间切换。
- `--ephemeral` channel 默认不显示在 `channel list` 中，是
  `channel prune --ephemeral` 的清理目标。
- `--linked-context-*` 会合并到 `--context-*`；使用时发出弃用通知。

### `list`

```bash
trellis channel list
  [--scope project|global]
  [--json]
  [--project <slug>]                      # substring match on task field
  [--all]                                 # include ephemeral (suffix '*')
  [--all-projects]                        # scan every project bucket
```

行为：
- 默认作用域为当前 cwd 的项目。`--all-projects` 扫描每个存储桶。
- 美化模式按最近活动排序，输出 `NAME WORKERS EVENTS LAST KIND TYPE TASK`，
  并在页脚注明隐藏的临时 channel 数量。
- `--json` 切换为 JSON 数组。

---

## Chat 消息

### `send <name> [text]`

```bash
trellis channel send <name> [text]
  --as <agent>                            # REQUIRED — author
  [--scope project|global]
  [--to <agents,csv>]                     # default: broadcast
  [--stdin | --text-file <path>]          # body from stdin or file
  [--delivery-mode appendOnly|requireKnownWorker|requireRunningWorker]
```

行为：
- 正文优先级：位置参数 `[text]` -> `--stdin` -> `--text-file`。
- `--to` 只有一个条目时存为字符串，多个条目时存为数组；省略表示广播。
- `--delivery-mode` 选择定向投递验证：
  - `appendOnly`（近似默认值，只记录事件），
  - `requireKnownWorker`（指定目标必须有 `spawned` 事件），
  - `requireRunningWorker`（Worker 当前必须存活）。
- 在 stdout 上以单行 JSON 输出追加的事件。

> **注意：**`send` **没有** `--tag` 或 `--kind` 参数。参见下文
> [`tag-vs-kind`](#tag-vs-kind--how-event-shape-is-actually-controlled)。

### `messages <name>`

```bash
trellis channel messages <name>
  [--scope project|global]
  [--raw]                                 # one JSON event per line
  [--follow]                              # stream new events
  [--last <N>]                            # last N matching events
  [--since <seq>]                         # seq > N
  [--kind <kind>]                         # one of CHANNEL_EVENT_KINDS
  [--from <csv>]                          # author filter
  [--to <target>]                         # routing target filter
  [--thread <key>]                        # forum-only
  [--action <thread-action>]              # forum-only
  [--no-progress]                         # hide progress events
```

行为：
- 自动检测 forum channel；没有过滤条件时呈现 thread 看板，而不是事件流。
  `--thread` / `--action` 只适用于 forum，在 chat channel 上会报错。
- 根据 `CHANNEL_EVENT_KINDS` 验证 `--kind`；这里只接受单值，不接受 CSV，
  CSV 是 `wait` 的行为。

### `wait <name>`

```bash
trellis channel wait <name>
  --as <agent>                            # REQUIRED — self for filter ctx
  [--scope project|global]
  [--timeout <Ns|Nm|Nh|Nms>]              # parsed by parseDuration
  [--from <a,b>]                          # author CSV
  [--kind <k1,k2>]                        # CSV, OR semantics
  [--thread <key>]                        # forum filter
  [--action <thread-action>]              # forum filter
  [--to <target>]                         # default: own agent (broadcast + me)
  [--include-progress]                    # also wake on progress events
  [--all]                                 # require every --from to match
```

行为：
- 以 JSON 流式输出匹配事件，每行一个。
- 默认 `--to` 过滤目标为调用方自身 Agent；广播事件仍然匹配，即广播 +
  显式发给自己的事件。
- `--all` 要求同时提供 `--from`，并阻塞到列出的每个 Agent 都产生匹配事件。
- **超时以 124 退出**；使用 `--all` 时向 stderr 输出
  `timeout: still waiting on ...`。

---

<a id="tag-vs-kind--how-event-shape-is-actually-controlled"></a>

## tag-vs-kind：事件形态的实际控制方式

v0.6.0 channel CLI 中任何位置都**没有 `--tag` 参数**；`--kind` 也不是
任何 `--tag` 参数的旧版别名。

当前源码中的具体模型：

- `--kind` 是唯一的事件类型过滤器，并受 Trellis 发出的白名单约束
  （`packages/core/src/channel/internal/store/events.ts` 中的
  `CHANNEL_EVENT_KINDS`）：
  - `create`, `join`, `leave`, `message`, `thread`, `context`, `channel`,
    `spawned`, `killed`, `respawned`, `progress`, `done`, `error`,
    `waiting`, `awake`, `undeliverable`, `interrupt_requested`,
    `turn_started`, `turn_finished`, `interrupted`, `supervisor_warning`
  - 传入其他值会抛出 `Invalid --kind '<x>'. Must be one of: …`。
- `--kind` 位于 `wait`（CSV，OR 语义）和 `messages`（单值）上。`send` 和
  `run` 不能发出自定义 kind；每次 `send` 都写入 `message` 事件。
- 在 turn 中途终止 Worker **不是** tag，而是由专用的 `channel interrupt`
  命令完成。该命令产生 `interrupt_requested` / `interrupted` 事件，并在
  Provider 层中断 Worker。

派发方等待 Worker 时的实用规则：

- “Worker 完成一个 turn”使用 `--kind done,turn_finished`；这些系统事件由
  Supervisor 自动发出，不要依赖 Worker LLM 记得发送任何自定义信号。
- 只有确实需要在 turn 中途终止时，才使用 `trellis channel interrupt` 命令。
- **不要**发明用户侧 tag 作为完成信号。不存在 `--tag` 过滤器；Worker 将
  自定义字符串写进最终消息，只会成为 `message` 事件中的文本，`wait` 无法
  据此匹配。

长正文始终通过 stdin 或文件传入：

```bash
trellis channel send T --as A --stdin < /tmp/message.md
trellis channel send T --as A --text-file /tmp/message.md
```

---

## 中断

### `interrupt <name> [text]`

```bash
trellis channel interrupt <name> [text]
  --as <agent>                            # REQUIRED — caller
  --to <agent>                            # REQUIRED — target worker
  [--scope project|global]
  [--stdin | --text-file <path>]
```

行为：
- 追加带有 `reason: "user"` 和替代指令正文的 `interrupt_requested` 事件；
  Supervisor 在支持时执行 Provider 层中断（Claude `/interrupt`、Codex turn
  取消），随后追加 `interrupted` 结果事件。
- 在 stdout 上输出本次追加的 `interrupt_requested` 事件 JSON。

---

## Worker

### `spawn <name>`

```bash
trellis channel spawn <name>
  [--scope project|global]
  [--agent <agent-name>]                  # loads .trellis/agents/<name>.md
  [--provider claude|codex]               # overrides agent file
  [--as <worker-name>]                    # default: agent name
  [--cwd <path>]
  [--model <id>]
  [--resume <id>]                         # session/thread id resume
  [--timeout <Ns|Nm|Nh>]                  # auto-kill after duration
  [--warn-before <Ns|Nm|Nh>]              # supervisor_warning lead time
                                          # default 5m, 0ms disables
  [--file <path>] ...                     # glob, repeatable; inject content
  [--jsonl <path>] ...                    # Trellis manifest, repeatable
  [--by <agent>]                          # spawn-event author
                                          # default: TRELLIS_CHANNEL_AS env or 'main'
  [--inbox-policy explicitOnly|broadcastAndExplicit]
                                          # default explicitOnly
  [--idle-timeout <Ns|Nm|Nh>]             # OOM-guard idle TTL
                                          # default 5m, 0 disables
  [--max-live-workers <n>]                # spawn-time live-worker budget
                                          # default 6, 0 disables
```

行为：
- 根据适配器注册表（`packages/cli/src/commands/channel/adapters/`）验证
  Provider；当前支持 `claude`、`codex`。
- Worker 保持 inbox 空闲，直到第一次收到 `send --to <worker>`。
- 记录包含 `pid`、`provider`、`agent`、`files`、`manifests` 的
  `spawned` 事件。
- OOM 防护优先级：CLI 参数 -> 环境变量
  （`TRELLIS_CHANNEL_WORKER_IDLE_TIMEOUT`、
  `TRELLIS_CHANNEL_MAX_LIVE_WORKERS`）->
  `.trellis/config.yaml#channel.worker_guard` -> 内置默认值。

### `run [name]`

```bash
trellis channel run [name?]
  [--agent <name>]
  [--provider claude|codex]
  [--as <worker-name>]
  [--cwd <path>]
  [--model <id>]
  [--file <path>] ...                     # repeatable, glob
  [--jsonl <path>] ...                    # repeatable
  [--message <text> | --message-file <path> | --stdin]
  [--timeout <Ns|Nm|Nh>]                  # default 5m
```

行为：
- 一次性执行。省略 `name` 时自动生成 `run-<hex>`。
- 创建临时 channel（`createMode=run`）、启动单个 Worker、发送 prompt、等待
  `done`、将最终 Assistant 文本打印到 stdout，并在成功时删除 channel。
  失败时保留 channel 供检查，退出码为 1。

> `run` **没有** `--tag` 参数。完成状态通过 Supervisor 发出的 `done` 事件
> 检测。

### `kill <name>`

```bash
trellis channel kill <name>
  --as <agent>                            # REQUIRED — worker agent name
  [--scope project|global]
  [--force]                               # SIGKILL immediately
```

行为：
- 默认路径按 SIGTERM -> 8 秒宽限 -> SIGKILL 逐级升级；需要 SIGKILL 时，
  CLI 写入 `killed` 事件，确保日志如实记录。
- 清理 `pid`、`worker-pid`、`config`、`spawnlock` sidecar 文件；保留
  `log`、`session-id`、`thread-id` 用于取证/恢复。

### `rm <name>`

```bash
trellis channel rm <name>
  [--scope project|global]
```

行为：
- 终止所有存活 Worker，然后删除整个 channel 目录。
- 输出 `Removed channel '<name>'`。

### `prune`

```bash
trellis channel prune
  [--scope project|global]                # omitted: scan every project
  [--all | --empty | --idle <Ns|Nm|Nh|Nd> | --ephemeral]   # mutually exclusive
  [--yes]                                 # actually delete (default: dry-run)
  [--dry-run]                             # default true; redundant with default
  [--keep <names,csv>]                    # exclusion list
```

行为：
- 过滤参数互斥；同时使用会报错。
- 默认执行 dry-run；`--yes` 切换为真正删除。
- 不带 `--scope` 时扫描**每个**项目存储桶，这是有意设计的全仓清理；使用
  `--scope project|global` 时只扫描对应存储桶。
- 无论使用何种过滤条件，始终跳过有存活 Worker 的 channel。
- 输出：每个候选项一行 `name  last-ts  (reason)`，最后输出汇总。

---

## Forum Channel

### `post <name> <action>`

```bash
trellis channel post <name> <action>
  --as <agent>                            # REQUIRED
  [--scope project|global]
  [--thread <key>]                        # required except action=opened
  [--title <text>]
  [--text <text> | --stdin | --text-file <path>]
  [--description <text>]                  # stable thread description
  [--status <status>]
  [--labels a,b]                          # REPLACES thread labels
  [--assignees a,b]                       # REPLACES assignees
  [--summary <text>]
  [--context-file <abs-path>] ...
  [--context-raw  <text>]      ...
  [--linked-context-file <abs-path>]      # [deprecated alias]
  [--linked-context-raw  <text>]          # [deprecated alias]
```

行为：
- CLI 接口上的 `<action>` 是自由格式；约定值包括 `opened`、`comment`、
  `status`、`labels`、`assignees`、`summary`、`processed`。
- `action=rename` 会被拒绝；改用 `thread rename`。
- `--labels` / `--assignees` 采用替换语义，而不是追加。
- 在 stdout 上输出追加的事件 JSON。

### `forum <name>`

```bash
trellis channel forum <name>
  [--scope project|global]
  [--status <status>]
  [--raw]
```

行为：
- 列出 reducer 归并后的 thread 状态。`--status` 按当前 thread 状态过滤；
  `--raw` 为每个 thread 输出一个 JSON。

### `thread <name> <thread>` / `thread rename`

```bash
trellis channel thread <name> <thread-key>
  [--scope project|global]
  [--raw]

trellis channel thread rename <name> <old-thread> <new-thread>
  --as <agent>                            # REQUIRED
  [--scope project|global]
```

行为：
- `thread <name> <key>` 显示一个 thread 的时间线：先输出
  `<thread> [<status>] <title>` 头部，再输出 description / labels /
  assignees / summary / timeline 行。`--raw` 切换为原始事件。
- `thread rename` 是唯一的修改操作；`post --action rename` 会被拒绝。

---

## 上下文/标题

### `context add` / `context delete` / `context list`

```bash
trellis channel context add <name>
  [--as <agent>]                          # default: main
  [--scope project|global]
  [--thread <key>]                        # thread-level instead of channel-level
  [--file <abs-path>] ...                 # repeatable
  [--raw <text>]      ...                 # repeatable
                                          # at least one of --file or --raw

trellis channel context delete <name>
  [--as <agent>]                          # default: main
  [--scope project|global]
  [--thread <key>]
  [--file <abs-path>] ...
  [--raw <text>]      ...

trellis channel context list <name>
  [--scope project|global]
  [--thread <key>]
  [--raw]                                 # one JSON entry per line
```

行为：
- `add` / `delete` 追加 `context` 事件并输出事件 JSON。
- `list` 投影当前上下文条目；美化输出为 `file <path>` /
  `raw <truncated text>` 行，为空时输出 `(no context)`。

### `title set <name>` / `title clear <name>`

```bash
trellis channel title set <name>
  --title <text>                          # REQUIRED
  [--as <agent>]                          # default: main
  [--scope project|global]

trellis channel title clear <name>
  [--as <agent>]                          # default: main
  [--scope project|global]
```

行为：
- 追加 `kind: "channel", action: "title"` 事件，为 channel 投影稳定的显示
  标题。输出事件 JSON。

---

## 隐藏/内部命令

| 命令 | 用途 |
|---|---|
| `channel __supervisor <channel> <worker> <config>` | `spawn` 调用的派生入口。不要直接调用。 |
| `channel __parse-trace <adapter> <file>` | 开发辅助命令：通过匹配的适配器重放已记录的 stream-json / wire trace，并打印生成的 channel 事件。根据 Provider 注册表验证适配器。 |

---

## 事件模型

`CHANNEL_EVENT_KINDS`（由 `parseChannelKind` 强制执行的白名单）：

`create`, `join`, `leave`, `message`, `thread`, `context`, `channel`,
`spawned`, `killed`, `respawned`, `progress`, `done`, `error`, `waiting`,
`awake`, `undeliverable`, `interrupt_requested`, `turn_started`,
`turn_finished`, `interrupted`, `supervisor_warning`.

`MEANINGFUL_EVENT_KINDS`（未显式提供 `--kind` 时，`wait` / `messages`
使用的默认可见子集）：

`create`, `join`, `leave`, `message`, `thread`, `context`, `channel`,
`spawned`, `killed`, `respawned`, `done`, `error`.

不属于 `MEANINGFUL_EVENT_KINDS` 的 kind（例如 `progress`、`waiting`、
`awake`、`supervisor_warning`、`turn_*` / `interrupt*` 集合）仍会流经存储；通过
`--kind` 或 `--include-progress` 选择查看。

Forum channel 采用事件溯源；使用 CLI reducer（`forum`、`thread`、
`context list`）进行状态投影。

---

## 输出约定

- **修改操作**（`send`、`interrupt`、`post`、`context add/delete`、
  `title set/clear`、`thread rename`）在 **stdout** 上以单行 JSON 输出追加的事件。
- **流式读取**（`wait`、`messages --follow`）在 stdout 上每行输出一个 JSON 事件。
- **美化读取**（`list`、`messages`、`forum`、`thread`、`context list`）
  输出带颜色和填充的表格/时间线。
- **`run`** 只在 stdout 上输出最终 Assistant 文本，便于调用方使用管道；
  诊断说明写入 stderr。
- **错误**通过 `chalk.red("Error:")` 写入 stderr，并以 `exit 1` 退出。
- **`wait` 超时**专门以 **124** 退出。
