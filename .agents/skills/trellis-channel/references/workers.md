# Worker 与 Agent Card

当协作 Agent 应独立执行并通过 channel 事件日志报告时，使用 Worker。Worker 是
附加到 channel 的已注册子进程（claude 或 codex）；Supervisor 将 inbox 消息
转发给它，并把它的输出转换回 channel 事件。

## 启动 Worker

```bash
trellis channel create impl-task --by dispatcher --cwd /path/to/repo
trellis channel spawn impl-task --provider codex --as codex-impl --timeout 30m

echo "Implement the schema for table X per .trellis/.../prd.md" \
  | trellis channel send impl-task --as dispatcher --to codex-impl --stdin

trellis channel wait impl-task --as dispatcher --from codex-impl --kind done --timeout 30m
```

`spawn` 会派生一个 `channel __supervisor` Worker；它发出 `spawned`、流式发送
`progress`，并应以 `done`、`error` 或 `killed` 结束。Worker 保持 inbox 空闲，
直到 `send --to <worker>` 将其唤醒；设置 `--inbox-policy
broadcastAndExplicit` 时，广播也可将其唤醒。

主要 `spawn` 参数：

- `--agent <name>`：加载 `.trellis/agents/<name>.md`，取得 provider/model/as/system prompt 默认值。
- `--provider <claude|codex>`：覆盖 Agent Card，并根据适配器注册表验证。
- `--as <name>`：channel 中的 Worker 句柄，默认为 Agent 名称。
- `--cwd <path>`：Worker 工作目录，也是 `--file`/`--jsonl` 的路径限制根目录。
- `--model <id>`：覆盖模型。
- `--resume <id>`：恢复现有 claude session / codex thread。
- `--timeout <duration>`：经过 `30s` / `2m` / `1h` 后自动终止。
- `--warn-before <duration>`：提前发出 `supervisor_warning` 的时间，默认 `5m`；`0ms` 禁用。
- `--file <path>`（可重复，支持 glob）：向 system prompt 注入文件内容。
- `--jsonl <path>`（可重复）：Trellis JSONL 清单，每行一个 `{file, reason}`。
- `--by <agent>`：`spawned` 事件的作者，默认为 `$TRELLIS_CHANNEL_AS` 或 `main`。
- `--inbox-policy <explicitOnly|broadcastAndExplicit>`：默认为 `explicitOnly`。
- `--idle-timeout <duration>`：OOM 防护的空闲 TTL，默认 `5m`；`0` 禁用。
- `--max-live-workers <n>`：启动时的存活 Worker 数量上限，默认 `6`；`0` 禁用。

成功事件 `spawned` 会记录 `pid`、`provider`、`agent`、注入的 `files` 和解析后的
`manifests`，便于后续观察者审计上下文。

## Agent Card

`--agent <name>` 解析为 `.trellis/agents/<name>.md`。Card 名称必须匹配
`[A-Za-z0-9._-]+`。Trellis 默认安装包含两张 Card：

- `.trellis/agents/check.md`：代码质量审查者。
- `.trellis/agents/implement.md`：执行实施工作的编码 Worker。

```yaml
---
name: check
description: Code quality check expert.
provider: claude
---
```

Frontmatter 字段用于填充 `spawn` 默认值（provider、model、`as`）；Markdown
正文成为 Worker 的 system prompt 角色。Card **不会**自动附加任务文件；每次
spawn 都必须显式注入上下文（见下文）。

启动命名 Agent 前，始终先检查项目 Card：

```bash
ls .trellis/agents
sed -n '1,100p' .trellis/agents/check.md
```

## 上下文注入

以下两个参数将内容注入 Worker system prompt 中由 `context-loader` 组装的
`# CONTEXT FILES` 块：

- `--file <path>`：可重复，支持 glob（`*`、`**`）。读取并连接每个匹配项。
- `--jsonl <path>`：可重复的 Trellis 清单，每行格式为
  `{"file":"<path>","reason":"<why>"}`。`reason` 会作为头部注释保留在
  每个文件内容上方。

加载器强制执行以下限制：

- 每个文件硬限制 1 MB，超出即报错。
- 单文件达到 200 KB 时向 stderr 发出警告。
- 组装后的上下文总量达到 500 KB 时向 stderr 发出警告。
- 路径遍历限制：所有解析后的路径都必须位于 `--cwd` 下。

针对任务目录启动 Check Agent 的示例：

```bash
TASK=.trellis/tasks/05-13-example
trellis channel spawn cr-example --agent check --provider codex --as check-cx \
  --file "$TASK/prd.md" \
  --file "$TASK/design.md" \
  --file "$TASK/implement.md" \
  --jsonl "$TASK/check.jsonl" \
  --cwd "$PWD" --timeout 30m
```

`spawned` 事件会同时记录字面量 `files` 数组和从 `--jsonl` 展开的所有
`manifests`，因此审计轨迹能够反映 Worker 实际看到的内容。

## 名称与路由

`--as` 有两种含义：

- `send` / `wait` / `interrupt`：发言者身份，即结果事件的作者。
- `spawn`：其他 Agent 通过 `--to` 寻址的 Worker 句柄。

多个 Worker 或 Provider 参与同一个 channel 时，使用显式名称：

```bash
trellis channel spawn cr-feature --agent check --as check-claude
trellis channel spawn cr-feature --agent check --provider codex --as check-cx

trellis channel wait cr-feature --as main \
  --from check-claude,check-cx --kind done --all --timeout 15m
```

`--all` 要求同时提供 `--from`，并阻塞到列出的每个 Worker 都产生匹配事件；
超时以退出码 **124** 结束，并向 stderr 输出 `timeout: still waiting on ...`。

## 软中断：`interrupt`

`channel interrupt` 是协作式重定向：它追加一个 `interrupt_requested` 事件
（reason 为 `"user"`）；Supervisor 收到后会在适配器支持时中断 Provider 当前
turn、发送替代指令，并追加 `interrupted` 结果事件。需要 Worker 放弃当前 turn、
立即处理新输入且不丢失 session 时使用。

```bash
echo "Stop refactoring the parser — switch to fixing the failing test in src/foo.ts" \
  | trellis channel interrupt impl-task --as dispatcher --to codex-impl --stdin
```

参数：

- `--as <agent>` **（必填）**：调用方身份。
- `--to <agent>` **（必填）**：目标 Worker。
- `--scope <project|global>`：channel 作用域。
- `--stdin` / `--text-file <path>` / `[text]`：替代指令正文。

命令首先追加 `kind: "interrupt_requested"`；下游 `wait` / `messages` 可通过
`--kind interrupt_requested,interrupted`（`messages` 每次只能指定一个 kind）
观察请求及处理结果，例如记录重定向，或让其他 Worker 等待协调者完成纠正。

对于应等到 Worker 下一个 turn 再处理的低优先级提示，发送普通定向消息：

```bash
echo "Check this when you reach the next turn." \
  | trellis channel send impl-task --as dispatcher --to codex-impl \
      --stdin
```

## 硬中断：`kill` + `--resume`

Worker 必须**立即**停止时使用 `kill`，例如循环失控、错误指令已在执行，或
适配器未响应 `interrupt`。Supervisor 按 SIGTERM -> 8 秒宽限 -> SIGKILL
逐级升级；需要 SIGKILL 时，CLI 会写入 `killed` 事件，确保事件日志如实记录。

```bash
trellis channel kill impl-task --as codex-impl
trellis channel spawn impl-task --as codex-impl --provider codex \
  --resume "$(cat ~/.trellis/channels/<bucket>/impl-task/worker.session-id)"

echo "STOP — new instructions: ..." \
  | trellis channel send impl-task --as dispatcher --to codex-impl --stdin
```

`kill` 参数：

- `--as <agent>` **（必填）**：指定 Worker 名称（位置参数 `<name>` 是 channel）。
- `--scope <project|global>`。
- `--force`：立即发送 SIGKILL，同时终止内部 Worker PID。

副作用：清理 `pid`、`worker-pid`、`config`、`spawnlock` sidecar 文件；保留
`log`、`session-id`、`thread-id`，用于取证和恢复。

当 `interrupt` 无法收敛时，kill + `--resume` 是确定可用的重定向路径。

## Worker OOM 防护

OOM 防护避免孤立/空闲 Worker 不断累积并耗尽主机资源。它在每次 `spawn` 时运行，
并对每个项目存储桶执行两项策略：

- **空闲 TTL**：清理最后活动时间早于配置阈值的 Worker，默认 `5m`；`0` 禁用。
- **存活 Worker 数量上限**：同一项目存储桶中已有超过 N 个 Worker 存活时，
  拒绝新的 spawn，默认 `6`；`0` 禁用。

优先级从高到低为：

1. CLI 参数：`spawn` 上的 `--idle-timeout`、`--max-live-workers`。
2. 环境变量：`TRELLIS_CHANNEL_WORKER_IDLE_TIMEOUT`、
   `TRELLIS_CHANNEL_MAX_LIVE_WORKERS`.
3. `.trellis/config.yaml` 中的 `channel.worker_guard`。
4. 内置默认值（`5m`、`6`）。

清理通知会在 spawn 时写入 stderr，让操作者看到哪些空闲 Worker 被清理，以及
新 spawn 为何被拒绝。该防护不会区别对待临时 Worker / `channel run` Worker；
它们遵循相同的空闲 TTL 和数量上限。

要审计当前状态，通过 `channel list` 的 `WORKERS` 列查看 Worker，并检查
`~/.trellis/channels/<bucket>/<channel>/` 下每个 channel 的 `pid` /
`worker-pid` sidecar 文件。

## Worker Inbox API

Inbox 是唤醒 Worker 的 channel 接口。路由由两个选项控制：

- **Inbox 策略**（`spawn --inbox-policy`）：
  - `explicitOnly`（默认）：Worker 只由 `send --to <worker>` 或
    `interrupt --to <worker>` 唤醒。
  - `broadcastAndExplicit`：广播也可唤醒，即不带 `--to` 的 `send`。
- **投递模式**（`send --delivery-mode`）：
  - `appendOnly`：无论 Worker 状态如何都追加事件。
  - `requireKnownWorker`：`--to` 指定的 Worker 从未启动过时失败。
  - `requireRunningWorker`：指定的 Worker 当前不存活时失败。

调用方预期协作者正在运行时，更严格的投递模式可防止消息静默丢失。

与 inbox 相关的子命令：

- `send <channel> [text]`：追加 `message` 事件。
  - `--as <agent>` **（必填）**：作者。
  - `--to <agents>`：CSV；一个目标存为字符串，多个目标存为数组；省略时广播。
  - `--stdin` / `--text-file <path>` / `[text]`：正文来源。
  - `--delivery-mode <appendOnly|requireKnownWorker|requireRunningWorker>`.
- `interrupt <channel> [text]`：软中断重定向，见上文。
- `wait <channel>`：阻塞到匹配事件到达。
  - `--as <agent>` **（必填）**：过滤上下文中的 `self`。
  - `--from <agents>`：作者 CSV。
  - `--kind <kind[,kind...]>`：CSV，采用 OR 语义；支持
    `interrupt_requested`、`interrupted`、`done`、`progress` 等。
  - `--to <target>`：默认为自身 Agent，匹配广播和显式发给自己的事件。
  - `--include-progress`：也由 progress 事件唤醒。
  - `--all`：要求每个 `--from` Agent 都匹配，超时以 **124** 退出。
  - `--timeout <duration>`：`30s` / `2m` / `1h` / `1000ms`。
- `messages <channel>`：查看、过滤或跟随事件流。
  - `--follow` 持续跟随；`--kind` / `--from` / `--to` 过滤；`--raw`
    每行输出一个 JSON；`--no-progress` 隐藏 progress 噪声。

典型的派发方循环：

```bash
# 1. Wake the worker.
echo "Run the failing test and report." \
  | trellis channel send impl-task --as dispatcher --to codex-impl --stdin \
      --delivery-mode requireRunningWorker

# 2. Block until it finishes.
trellis channel wait impl-task --as dispatcher \
  --from codex-impl --kind done,error --timeout 30m

# 3. Read the final answer.
trellis channel messages impl-task --from codex-impl --last 1 --raw
```

所有发出事件的子命令（`send`、`interrupt`、`post`、`context add` / `delete`、
`title set` / `clear`、`thread rename`）都会在 stdout 上以单行 JSON 打印
追加的事件，便于为 inbox 层编写脚本。
