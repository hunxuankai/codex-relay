# 工作流模式

根据意图选用以下模式。多轮工作优先使用持久 channel，一次性问题使用
`channel run`。

## 模式 A：多轮需求探索

当用户说“和 codex/claude 讨论一下”“brainstorm”或“拉一个 Agent
进来一起看”时使用。

```bash
trellis channel create brainstorm-storage-layer --by main \
  --task .trellis/tasks/05-XX-storage-adapter

trellis channel spawn brainstorm-storage-layer \
  --agent architect --provider codex \
  --file .trellis/tasks/05-XX-storage-adapter/prd.md \
  --file .trellis/tasks/05-XX-storage-adapter/design.md \
  --as cx-arch --timeout 30m

trellis channel send brainstorm-storage-layer \
  --as main --to cx-arch --text-file /tmp/brainstorm-r1.md

trellis channel wait brainstorm-storage-layer \
  --as main --kind done --from cx-arch --timeout 10m
```

不要收到一个回答就停止。阅读回答、找出模糊之处、发送新的追问并重复，
直到结论可执行。

最低轮次结构：

1. 方向选择：应使用现有机制还是新机制？
2. MVP 边界：v1、v2，以及什么条件会迫使 v2 内容回到 v1。
3. 数据契约：事件、Schema、元数据、状态事实来源和兼容性。
4. CLI / UX 契约：命令名、参数、错误、默认值和歧义。
5. 跨层风险与测试：共享辅助函数、偏移点和阻塞发布的测试。

可选轮次：

- 运维：日志、调试、Worker 卡住、终止/重启和恢复。
- 迁移/发布：破坏性状态、清单、变更日志和文档站点。
- 反方审查：让协作 Agent 反驳当前方案。

每次追问都应要求给出具体文件路径、命令、Schema、被否决的方案和阻塞发布的
问题。需要作出决定时，不接受含糊其辞。

## 模式 B：Implement / Check Agent

当用户要求派发实施或审查工作时使用。

```bash
TASK=.trellis/tasks/05-12-foo
trellis channel create cr-foo --task "$TASK" --by main

trellis channel spawn cr-foo \
  --agent check \
  --jsonl "$TASK/check.jsonl" \
  --file "$TASK/prd.md" \
  --file "$TASK/design.md" \
  --file "$TASK/implement.md" \
  --cwd "$PWD" --timeout 15m

trellis channel send cr-foo --as main --to check --text-file /tmp/cr-brief.md
trellis channel wait cr-foo --as main --kind done --from check --timeout 15m
trellis channel messages cr-foo --kind message --from check --last 1 --raw
```

实施工作使用 `--agent implement`，并发送实施简报。检查工作应包含精确的
diff 范围、相关规范和已经运行的验证。

## 模式 C：并行审查者

使用一个 channel 和互不相同的 Worker 名称。

```bash
trellis channel create cr-feature --by main --ephemeral

trellis channel spawn cr-feature --agent check \
  --jsonl "$TASK/check.jsonl" --file "$TASK/prd.md" --file "$TASK/design.md" \
  --timeout 15m

trellis channel spawn cr-feature --agent check --provider codex --as check-cx \
  --jsonl "$TASK/check.jsonl" --file "$TASK/prd.md" --file "$TASK/design.md" \
  --timeout 15m

trellis channel send cr-feature --as main --to check --text-file /tmp/cr-brief.md
trellis channel send cr-feature --as main --to check-cx --text-file /tmp/cr-brief.md
trellis channel wait cr-feature --as main --kind done --from check,check-cx --all --timeout 15m
```

`--all` 表示列出的每个 Worker 都必须发出匹配事件。

## 模式 D：一次性 Worker

```bash
trellis channel run --provider codex --message "say hi in 3 words" --timeout 1m
trellis channel run --agent plan --message-file /tmp/plan-question.md --timeout 10m
```

成功时，`run` 会删除临时 channel；发生错误、超时或终止时，则保留 channel
并打印路径以供检查。

## 模式 E：Forum Channel

用于问题论坛、主题式反馈、发布待办、Agent 发现和内部变更日志。完整模型见
`forum.md`。

## 模式 F：接手现有 Thread

如果用户给出 forum/thread 名称，请自行恢复上下文：

```bash
trellis channel forum <board> --scope global
trellis channel thread <board> <thread> --scope global --raw
trellis channel context list <board> --scope global --thread <thread>
trellis channel messages <board> --scope global --raw --thread <thread>
```

输出约束摘要，而不是倾倒完整对话记录：

- 用户层面的问题
- 影响当前仓库的上下文文件
- 当前版本与未来版本需求
- 当前代码/设计是否满足需求
- 下一步操作或要追加的评论
