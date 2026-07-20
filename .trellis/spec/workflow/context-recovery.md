# 上下文检查点与恢复

## 强制任务文件

复杂任务维护：

```text
task.json
prd.md
design.md
implement.md
implement.jsonl
check.jsonl
research/
```

`implement.md` 必须持续包含：

```markdown
## 当前进度
## 已完成
## 关键决策
## 验证证据
## 下一步
## 尚未解决的问题
```

## 必须更新的时机

- 每个实施步骤完成后；
- 每次 Git 提交后；
- 关键需求、设计或边界改变后；
- 运行耗时较长的测试、构建或外部工具前；
- 对话已经较长、可能压缩时；
- 暂停工作或结束当前回合前。

## 恢复顺序

1. 定位当前任务：

   ```powershell
   python .trellis/scripts/task.py current --source
   ```

2. 读取 `task.json`、`prd.md`、`design.md`、`implement.md`。
3. 验证 `implement.jsonl` 和 `check.jsonl` 可解析且引用文件存在。
4. 只加载当前阶段需要的 `.trellis/spec/`。
5. 检查点仍缺细节时再使用：

   ```powershell
   trellis mem search "<关键词>"
   trellis mem context <session-id>
   ```

6. 把恢复的新结论立即写回任务文件，不能继续只依赖聊天。

## 会话指针

Trellis 0.6.7 的当前任务指针位于被忽略的 `.trellis/.runtime/sessions/`，由平台会话标识或 `TRELLIS_CONTEXT_ID` 选择。它是本机运行状态，不提交；共享进度必须写进任务文件。

## 记忆边界

`trellis mem` 是缺失对话细节的补救，不是检查点替代品。搜索和上下文输出不得复制真实 API Key、Authorization Header 或完整认证文件到任务、规范或日志。
