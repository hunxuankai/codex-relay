# 本地任务系统

Trellis 任务系统完全存储在用户项目的 `.trellis/tasks/` 下。每个任务都是一个目录，包含需求、上下文、研究、状态和关系信息。

## 任务目录结构

```text
.trellis/tasks/
├── 04-28-example-task/
│   ├── task.json
│   ├── prd.md
│   ├── design.md
│   ├── implement.md
│   ├── implement.jsonl
│   ├── check.jsonl
│   └── research/
└── archive/
    └── 2026-04/
```

| 文件 | 用途 |
| --- | --- |
| `task.json` | 任务元数据：状态、负责人、优先级、分支、父/子任务及类似字段。 |
| `prd.md` | 需求、约束和验收标准。轻量任务可以只有 PRD。 |
| `design.md` | 复杂任务的技术设计：边界、契约、数据流、兼容性和权衡。 |
| `implement.md` | 复杂任务的执行计划：有序清单、验证命令、审查门禁和回滚点。 |
| `implement.jsonl` | implement 代理必须先读取的规范/研究文件列表。 |
| `check.jsonl` | check 代理必须先读取的规范/研究文件列表。 |
| `research/` | 研究产物。复杂发现不应只存在于聊天中。 |

## `task.json`

`task.json` 记录任务状态和元数据。常见字段：

| 字段 | 含义 |
| --- | --- |
| `id` / `name` / `title` | 任务标识和标题。 |
| `status` | `planning`、`in_progress`、`review` 或 `completed` 等状态。 |
| `priority` | `P0`、`P1`、`P2`、`P3`。 |
| `creator` / `assignee` | 创建者和负责人。 |
| `package` | Monorepo 中的目标包；可以为空。 |
| `branch` / `base_branch` | 工作分支和 PR 目标分支。 |
| `children` / `parent` | 父/子任务关系。 |
| `commit` / `pr_url` | 完成后的提交和 PR 信息。 |
| `meta` | 扩展字段。 |

## 父/子任务树

父/子任务关系用于组织工作。父任务把相关交付物归入同一组源需求；它不是依赖调度器，也不替代子任务自身的规划材料。

当请求包含多个可独立验证的交付物时，使用父任务。父任务负责：

- 源需求和面向用户的范围。
- 子任务映射及其职责边界。
- 跨子任务验收标准和最终集成审查。

对可以独立完成规划、实施、检查和归档的交付物使用子任务。如果一个子任务依赖另一个，应把依赖写入子任务的 `prd.md` / `implement.md`；不要依赖树中位置暗示顺序。

使用以下命令创建新子任务：

```bash
python ./.trellis/scripts/task.py create "<child title>" --slug <child-slug> --parent <parent-dir>
```

使用以下命令链接或取消链接现有任务：

```bash
python ./.trellis/scripts/task.py add-subtask <parent-dir> <child-dir>
python ./.trellis/scripts/task.py remove-subtask <parent-dir> <child-dir>
```

父任务上的 `children` 是历史列表。子任务归档时，Trellis 会在父任务中保留其名称，使已完成子任务移入 `archive/` 后，类似 `[2/3 done]` 的进度仍有意义。

AI 不应把阶段编号视为任务状态。任务进度主要由 `status`、材料是否存在（`prd.md`，以及可选的 `design.md` / `implement.md`）、子代理模式是否已配置 JSONL 上下文，以及 `workflow.md` 中的阶段说明决定。

## 活动任务

用户看到的是“当前任务”，但 Trellis 按会话存储活动任务状态。

```text
.trellis/.runtime/sessions/<context-key>.json
```

`task.py start` 把任务路径写入当前会话的运行时会话文件。`task.py current --source` 显示当前任务及其来源。不同 AI 窗口可以指向不同任务，而不会互相覆盖。

如果平台或 Shell 环境没有稳定会话身份，`task.py start` 可能无法设置活动任务。AI 应读取错误、检查平台钩子/会话环境，不要回退到共享全局指针。

## JSONL 上下文

`implement.jsonl` 和 `check.jsonl` 是供子代理优先读取的上下文清单。它们不替代 `implement.md`；后者是人类可读的执行计划。

格式：

```jsonl
{"file": ".trellis/spec/cli/backend/index.md", "reason": "Backend conventions"}
{"file": ".trellis/tasks/04-28-example/research/api.md", "reason": "API research"}
```

规则：

- 包含规范和研究文件。
- 不要包含即将修改的代码文件。
- 不要把聊天中的临时结论作为唯一上下文。
- 种子行没有 `file` 字段，只用于提示 AI 填写真实记录。

## 常用命令

```bash
python ./.trellis/scripts/task.py create "<title>" --slug <slug>
python ./.trellis/scripts/task.py start <task>
python ./.trellis/scripts/task.py current --source
python ./.trellis/scripts/task.py add-context <task> implement <file> <reason>
python ./.trellis/scripts/task.py validate <task>
python ./.trellis/scripts/task.py finish
python ./.trellis/scripts/task.py archive <task>
```

修改任务系统时，AI 应优先使用脚本命令维护结构。只有脚本无法覆盖需求时，才直接编辑 JSON/Markdown。

## 本地定制点

| 需求 | 编辑位置 |
| --- | --- |
| 修改默认任务模板 | `.trellis/scripts/common/task_store.py` 和任务创建说明。 |
| 修改状态语义 | `.trellis/workflow.md`、workflow-state 钩子逻辑和任务使用约定。 |
| 添加任务生命周期动作 | `.trellis/config.yaml` 中的 `hooks.after_*`。 |
| 修改上下文规则 | `.trellis/workflow.md` 中的规划材料指南及相关平台代理/钩子指令。 |
| 修改归档策略 | `.trellis/scripts/common/task_store.py` / `task_utils.py`。 |

这些都是用户项目中的本地文件。除非用户希望贡献上游，否则不要默认编辑 Trellis CLI 源码。
