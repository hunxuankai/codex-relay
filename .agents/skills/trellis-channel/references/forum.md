# Forum Channel

Forum channel 是持久的主题式 channel。创建 channel 时通过 `--type forum`
指定，之后类型不可更改。它不是普通聊天流；默认读取路径是
**forum 摘要 -> 单个 thread 时间线 -> 当前上下文**。

## Forum 与普通 Channel

Channel 类型在 `channel create` 时通过 `--type` 设置，之后永不改变：

- `chat`（默认）：扁平消息时间线。`channel messages` 始终呈现事件流；
  `--thread` 和 `--action` 等 forum 专属参数在此类型中会被拒绝。
- `forum`：以 thread 为中心。不带过滤条件的 `channel messages` 呈现
  thread 看板摘要，而不是原始事件。`post`、`forum`、`thread` 和
  `thread rename` 子命令只适用于 forum channel。

两种类型使用相同的作用域模型（默认为 `--scope project`；
`--scope global` 将 channel 放入跨项目存储桶）。

## 创建 Forum Channel

```bash
trellis channel create design-feedback \
  --type forum \
  --scope global \
  --description "Cross-project design feedback board." \
  --context-raw "One thread per design topic; close when resolved." \
  --by main
```

单仓库看板使用 `--scope project`，跨项目看板使用 `--scope global`。

## Thread：打开、评论、状态与摘要

Thread 位于 forum channel 内。每个 thread 由稳定的 `--thread <key>` 标识
（约定使用小写 kebab-case）。Thread 的第一个操作是 `opened`；之后所有操作
都使用同一个 `--thread` 键。

```bash
trellis channel post design-feedback opened \
  --scope global \
  --as main \
  --thread login-empty-state \
  --title "Empty state on the login screen" \
  --description "Track design feedback for the new login empty state." \
  --labels design,login \
  --context-raw "Spotted during the 0.4 release review." \
  --text-file /tmp/thread-open.md

trellis channel post design-feedback comment \
  --scope global \
  --as reviewer \
  --thread login-empty-state \
  --text-file /tmp/review.md

trellis channel post design-feedback status \
  --scope global \
  --as main \
  --thread login-empty-state \
  --status closed

trellis channel post design-feedback summary \
  --scope global \
  --as main \
  --thread login-empty-state \
  --summary "Adopted the option-B layout; ticket TRELLIS-123 owns the fix."
```

关键区别：

- `--description` 是**持久的** thread 描述，用于回答“这个 thread 讨论什么？”
  它在 `opened` 时设置，也可再次运行带 `--description` 的 `post` 来编辑。
- `--text` / `--stdin` / `--text-file` 是**事件正文**，即附加到本条时间线
  记录的评论或载荷。
- `--labels` 和 `--assignees` 是 CSV，并会**替换**当前值，而不是追加。
- `--summary` 是滚动更新的 thread 摘要。在 `status closed` 时设置摘要，
  是带上下文标记 thread 已解决的标准方式。

除 `opened` 外，每个操作都要求 `--thread`（实际使用中，`opened` 也需要该参数，
因为不存在匿名 thread）。

## 读取 Forum

```bash
trellis channel messages design-feedback --scope global
trellis channel forum design-feedback --scope global --status open
trellis channel thread design-feedback login-empty-state --scope global
trellis channel messages design-feedback --scope global --raw --thread login-empty-state
```

如果协作者说“我在 forum 中添加了评论”，先运行 `channel forum` 查看哪个
thread 发生变化，再用 `channel thread <name> <thread>` 深入查看。不要直接
临时解析 `events.jsonl`。

## 上下文

上下文条目是读取 channel 或 thread 时应始终纳入范围的持久背景。它们**不是**
时间线事件，而是单独投影并为每位读取者重放。

使用 `context` 子命令。`create` 和 `post` 上旧的 `--linked-context-file` /
`--linked-context-raw` 参数是已弃用别名，会合并到标准的 `--context-file` /
`--context-raw`。

### 添加上下文

```bash
# Channel-level context (whole forum)
trellis channel context add design-feedback \
  --scope global \
  --raw "Upstream feedback board; please link tasks before opening threads."

# Thread-level context (one thread)
trellis channel context add design-feedback \
  --scope global \
  --thread login-empty-state \
  --file "$PWD/.trellis/tasks/05-13-login-redesign/design.md"
```

- `--thread <key>` 用于在 channel 级和 thread 级上下文之间切换。
- `--file` 路径**必须是绝对路径**；相对路径会被拒绝。
- `--raw` 是纯文本内联内容。
- 两个参数都可重复；`add` / `delete` 至少需要其中一个。
- `--as <agent>` 记录作者身份，默认为 `main`。

### 列出上下文

```bash
trellis channel context list design-feedback --scope global
trellis channel context list design-feedback --scope global --thread login-empty-state --raw
```

`list` 上的 `--raw` 每行输出一个 JSON 条目，便于管道处理；不使用该参数时，
得到人类可读的 `file <path>` / `raw <truncated text>` 列表。存储为空时输出
`(no context)`。

### 删除上下文

```bash
trellis channel context delete design-feedback \
  --scope global \
  --thread login-empty-state \
  --raw "stale note"
```

删除依据是**值**而不是 ID：传入与添加时相同的 `--file` 或 `--raw` 值。
重复使用参数可在一次调用中删除多个条目。

### 读取顺序

读取 thread 时按以下顺序自上而下处理：

1. Thread `description`（持久的“讨论主题是什么”）。
2. 上下文条目（channel 级 + thread 级）。
3. 时间线（`opened`、`comment`、`status`、`summary`）。

如果上下文文件缺失或无法读取，请明确说明，并继续处理其余数据；不要捏造内容。

## 标题投影

`title` 为 channel 投影稳定的显示标题，而不重命名存储地址。传给每个命令的
channel `name` 保持不变。

```bash
trellis channel title set design-feedback \
  --scope global \
  --title "Design feedback board"

trellis channel title clear design-feedback --scope global
```

- `title set` 要求提供 `--title`。
- `--as <agent>` 记录作者身份，默认为 `main`。
- 这属于展示层变更。工具和脚本继续使用原始 channel 名称。

## 重命名 Thread

如果创建 thread 时使用了错误的键（拼写错误、slug 约定错误等），
`thread rename` 是修正路径。Thread 不支持硬删除，重命名是受支持的纠正操作。

```bash
trellis channel thread rename design-feedback old-key new-key \
  --scope global \
  --as main
```

- **必须**提供 `--as <agent>`。
- `post <name> rename` 会被拒绝；必须使用 `thread rename`。

## 删除约束

不要把删除单条评论或硬删除 thread 设计成正常工作流。Forum thread 是只追加的
协作历史。需要纠正状态时，请使用：

- `post ... status` 将 thread 标记为 closed、blocked 等状态。
- `post ... summary` 记录解决结果。
- `post ... --labels` 重新设置标签（替换整个集合）。
- `thread rename` 修正错误的 thread 键。

## 内部变更日志模式

全局 forum channel 的常见用途是内部发布/运行时变更日志。每项重要变更使用
一个 thread，可以让历史保持可搜索：

```bash
trellis channel create release-notes \
  --type forum \
  --scope global \
  --description "Internal release and runtime changelog." \
  --context-raw "One thread per notable change; close when shipped." \
  --by main

trellis channel post release-notes opened \
  --scope global \
  --as main \
  --thread release-2026-q1 \
  --title "Channel threads and forum UX in 0.6" \
  --description "Forum channel UX shipped in the 0.6 line." \
  --labels channel,release \
  --text-file /tmp/release-notes.md
```

使用稳定且有描述性的 thread 键（例如 `release-2026-q1`、
`runtime-event-schema-change`），方便后续读取者按名称查找。
