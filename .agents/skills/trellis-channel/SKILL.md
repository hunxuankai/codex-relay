---
name: trellis-channel
description: 使用 Trellis channel 进行实时多代理协作、派生 Worker、跨代理审查、进度检查、论坛频道管理和频道日志调试。
---

# trellis-channel

`trellis channel` 是本地多代理协作运行时。当代理需要通过持久事件日志交流、需要将 Worker 派生为对等进程、需要中断/调试正在运行的 Worker，或需要把反馈记录到持久的 `--type forum` 频道时使用。

典型用户信号：“和 codex/claude 讨论”“与另一个代理进行需求探索”“派生 implement/check Worker”“让代理审查”“开一个 issue 看板 / changelog 论坛”“看看这个 thread”“channel 卡住了 / 没输出”“progress 被截断”“这个 channel 命令怎么写”。

此 Skill 是索引。只加载当前工作所需的参考文件，不要预先加载全部文件。

## 首选命令

```bash
trellis --version
trellis channel --help
trellis channel list --all
trellis channel list --scope global --all
```

如果用户指定了频道或 thread，先检查它，再询问背景：

```bash
trellis channel forum <board> --scope global
trellis channel thread <board> <thread> --scope global
trellis channel context list <board> --scope global --thread <thread>
```

## 按用户意图路由

| 用户意图 | 读取 |
|---|---|
| “和 codex/claude 讨论一下”“与另一个代理进行需求探索” | `references/workflows.md` |
| “派一个 implement/check agent”“让 agent review”“派生 Worker” | 先读 `references/workflows.md`，再读 `references/workers.md` |
| “开 issue 区 / topic 群 / changelog / board”“创建论坛” | `references/forum.md` |
| “看看这个 thread / linked context”“检查 thread” | `references/forum.md` |
| “channel 卡住了 / 没输出 / progress 被截断”“Worker 停滞” | `references/progress-debugging.md` |
| “具体命令怎么写”“X 接受哪些 flag” | `references/command-reference.md` |

## 核心规则

- 新论坛频道使用 `--type forum`。`thread` 是论坛频道内的一条事项。
- 使用 `--context-file` / `--context-raw` 和 `trellis channel context add/delete/list`。`--linked-context-*` 是已弃用术语。
- 长消息使用 `--stdin` 或 `--text-file`。不要把很长的中英文混合文本放入位置式 Shell 参数。
- 美化后的 `messages` 输出是操作面板，可能截断进度。审计时使用 `--raw`。
- `--as` 根据命令表示发言者或 Worker 句柄。涉及多个代理或会话时，使用明确、稳定的名称。
- `--scope project`（默认）操作当前工作目录的项目桶；`--scope global` 操作共享的 `__global__` 桶。应有意识地选择范围；除非传入 `--scope global`，否则项目列表看不到全局看板。
- 需求探索应进行多轮压力测试。一次回答加一次确认属于审查，不是需求探索。
- **派发器等待模式**：使用 `--kind done` / `--kind turn_finished`（Supervisor 生成的系统事件）作为完成信号。当前 CLI 不存在 `--tag`；不要发明 `phase_done` / `question` 等自定义完成信号。需要在 turn 中途终止 Worker 时，使用专用的 `trellis channel interrupt` 命令，并观察 `interrupt_requested` / `interrupted` 事件。参见 `references/command-reference.md` 的“tag 与 kind”。
- 论坛频道使用事件溯源。不要先解析 `events.jsonl`；应使用 `forum`、`thread`、`messages --thread` 和 `context list`。
- `@mindfoldhq/trellis-core` 负责可复用的频道/thread 状态、事件追加、序号分配、上下文/标题投影、Reducer 和任务辅助函数。CLI 负责 flag、终端渲染、提示、Worker 生命周期和进程退出。

## 参考文件

- `references/workflows.md`——权威协作模式 A–F（对等需求探索、派生审查、派发并等待、论坛 issue 收集、中断并重定向、单次运行）。
- `references/forum.md`——论坛频道、上下文、标题、重命名、changelog 论坛、thread 筛选。
- `references/workers.md`——派生、Agent Card、上下文注入（`--file` / `--jsonl`）、中断、终止语义。
- `references/progress-debugging.md`——progress/raw 检查、停滞 Worker 诊断、OOM 防护、退出码。
- `references/command-reference.md`——当前 CLI 命令参考（所有子命令、所有 flag、输出约定、scope/type 模型）。

## 不适用场景

- 只需一个 Markdown 文件和提示词即可完成的静态审查。
- 用自我记录替代正常工具调用。
- 长期记忆检索。可执行问题使用持久论坛频道，会话/历史搜索使用 `trellis mem`（`trellis-session-insight` Skill）。
