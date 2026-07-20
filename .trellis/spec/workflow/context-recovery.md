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

## 会话恢复与写入所有权契约

### 1. 范围与触发条件

读取当前任务可以在缺少会话标识时使用唯一 session 指针恢复上下文；任务元数据推断、文件写入和 Git 暂存属于有副作用的边界，必须证明任务由当前精确 session 所有。

### 2. 签名

```python
resolve_active_task(repo_root: Path) -> ActiveTask
_get_session_owned_task(repo_root: Path) -> str | None
```

`ActiveTask` 的写入判定字段是 `source_type`、`stale` 和 `task_path`。只有 `source_type == "session"`、`stale == False` 且 `task_path` 存在时，`_get_session_owned_task` 才返回任务路径。

### 3. 契约

| 解析结果 | 只读上下文恢复 | 读取任务元数据 | 写入或 Git 暂存 |
|---|---|---|---|
| 精确、非 stale 的 `session` | 允许 | 允许 | 允许当前任务目录 |
| `session-fallback` | 允许 | 禁止 | 禁止；只处理当前 workspace |
| stale `session` | 可报告旧指针 | 禁止 | 禁止 |
| `none` | 无任务上下文 | 禁止 | 禁止 |

### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 当前 context 文件指向存在的任务 | 返回该任务，并允许推断 package/branch 与暂存该任务 |
| 当前 context 文件已清除，只剩其他 session 指针 | 解析器可返回 `session-fallback` 供只读恢复；写入 helper 必须返回 `None` |
| 当前 session 指向已归档或不存在的任务 | 视为 stale；不得加载其 `task.json`，也不得暂存其路径 |
| 无可用指针或存在多个候选 session | 当前任务未知；只允许记录 workspace |

### 5. 良好、基线与错误用例

- 良好：当前 session 精确指向活动任务，会话日志使用该任务的 package/branch，并只暂存 workspace 与该任务。
- 基线：没有当前任务时，会话日志仍可写入 workspace，但 package/branch 不从任务推断。
- 错误：当前 session 已结束，却把唯一剩余的其他 session 任务当作自己的任务加载和提交。

### 6. 必需测试

- 精确、非 stale 的 session 返回任务路径；同一指针变 stale 后返回 `None`。
- `session-fallback` 不调用 `load_task`，不参与 package/branch 推断。
- `session-fallback` 自动提交只传入 workspace 路径，其他任务目录不进入 `safe_git_add`。
- 测试只使用 `TemporaryDirectory` 与 mocked Git，不操作真实 `.trellis/.runtime` 或 Git index。

### 7. 错误与正确做法

错误做法是在写入边界调用只返回任务路径、丢失来源信息的 `get_current_task()`：

```python
current = get_current_task(repo_root)
```

正确做法是保留解析来源，并在任何元数据读取或暂存前收窄到当前精确 session：

```python
active = resolve_active_task(repo_root)
current = (
    active.task_path
    if active.source_type == "session" and not active.stale
    else None
)
```

## 记忆边界

`trellis mem` 是缺失对话细节的补救，不是检查点替代品。搜索和上下文输出不得复制真实 API Key、Authorization Header 或完整认证文件到任务、规范或日志。
