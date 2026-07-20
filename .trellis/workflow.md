# 开发工作流

---

## 核心原则

1. **先规划再编码**：开始前先明确要做什么
2. **注入规范，而非依赖记忆**：通过 hook/Skill 注入指南，不依赖记忆回想
3. **持久化一切**：研究、决策和经验都写入文件；对话会被压缩，文件不会
4. **增量开发**：一次只处理一个任务
5. **沉淀经验**：每个任务结束后复盘，并将新知识写回规范

---

## Trellis 系统

### 开发者身份

首次使用时初始化身份：

```bash
python ./.trellis/scripts/init_developer.py <your-name>
```

该命令会创建被 Git 忽略的 `.trellis/.developer`，以及
`.trellis/workspace/<your-name>/`。

### 规范系统

`.trellis/spec/` 保存按包和层组织的编码指南。

- `.trellis/spec/<package>/<layer>/index.md`：包含**开发前检查**和**质量检查**的
  入口；实际指南位于它所指向的 `.md` 文件中。
- `.trellis/spec/guides/index.md`：跨包思考指南。

```bash
python ./.trellis/scripts/get_context.py --mode packages   # list packages / layers
```

**何时更新规范**：发现新模式/约定、需要固化缺陷预防措施、作出新技术决策。

### 任务系统

每个任务在 `.trellis/tasks/{MM-DD-name}/` 下拥有独立目录，其中保存
`task.json`、`prd.md`、可选的 `design.md`、可选的 `implement.md`、可选的
`research/`，以及供支持子 Agent 的平台使用的上下文清单
（`implement.jsonl`、`check.jsonl`）。

```bash
# Task lifecycle
python ./.trellis/scripts/task.py create "<title>" [--slug <name>] [--parent <dir>]
python ./.trellis/scripts/task.py start <name>          # set active task (session-scoped when available)
python ./.trellis/scripts/task.py current --source      # show active task and source
python ./.trellis/scripts/task.py finish                # clear active task (triggers after_finish hooks)
python ./.trellis/scripts/task.py archive <name>        # move to archive/{year-month}/
python ./.trellis/scripts/task.py list [--mine] [--status <s>]
python ./.trellis/scripts/task.py list-archive

# Code-spec context (injected into implement/check agents via JSONL).
# `implement.jsonl` / `check.jsonl` are seeded on `task create` for sub-agent-capable
# platforms; the AI curates real spec + research entries during planning when needed.
python ./.trellis/scripts/task.py add-context <name> <action> <file> <reason>
python ./.trellis/scripts/task.py list-context <name> [action]
python ./.trellis/scripts/task.py validate <name>

# Task metadata
python ./.trellis/scripts/task.py set-branch <name> <branch>
python ./.trellis/scripts/task.py set-base-branch <name> <branch>    # PR target
python ./.trellis/scripts/task.py set-scope <name> <scope>

# Hierarchy (parent/child)
python ./.trellis/scripts/task.py add-subtask <parent> <child>
python ./.trellis/scripts/task.py remove-subtask <parent> <child>

# PR creation
python ./.trellis/scripts/task.py create-pr [name] [--dry-run]
```

> 运行 `python ./.trellis/scripts/task.py --help` 查看最新的权威列表。

**当前任务机制**：`task.py create` 创建任务目录，并在会话身份可用时自动设置
每会话活动任务指针，使规划路标立即触发。`task.py start` 写入同一个指针
（已设置时保持幂等），并将 `task.json.status` 从 `planning` 改为
`in_progress`。状态存储在 `.trellis/.runtime/sessions/` 下。如果无法从 hook
输入、`TRELLIS_CONTEXT_ID` 或平台原生会话环境变量取得上下文键，则不存在活动
任务，`task.py start` 会失败并提示会话身份。`task.py finish` 删除当前会话文件，
但不改变状态。`task.py archive <task>` 写入 `status=completed`、将目录移动到
`archive/`，并删除仍指向已归档任务的运行时会话文件。

### 工作区系统

在 `.trellis/workspace/<developer>/` 下记录每个 AI 会话，用于跨会话跟踪。

- `journal-N.md`：会话日志。**每个文件最多 2000 行**；超过上限时自动创建
  新的 `journal-(N+1).md`。
- `index.md`：个人索引，记录会话总数和最近活动时间。

```bash
python ./.trellis/scripts/add_session.py --title "Title" --commit "hash" --summary "Summary"
```

### 上下文脚本

```bash
python ./.trellis/scripts/get_context.py                            # full session runtime
python ./.trellis/scripts/get_context.py --mode packages            # available packages + spec layers
python ./.trellis/scripts/get_context.py --mode phase --step <X.Y>  # detailed guide for a workflow step
```

---

<!--
  WORKFLOW-STATE 路标契约（编辑下方 tag 块前先阅读）

  下方 ## Phase Index 章节内嵌的 [workflow-state:STATUS] 块，是所有受支持
  AI 平台 UserPromptSubmit hook 所读取的逐轮 `<workflow-state>` 路标的唯一
  事实来源。inject-workflow-state.py（Python 平台）和
  inject-workflow-state.js（OpenCode 插件）只负责解析；v0.5.0-rc.0 之后，
  脚本中不再内置后备字典。

  STATUS 字符集：[A-Za-z0-9_-]+。Hook 找不到 tag 时，会降级为通用的
  “Refer to workflow.md for current step.”提示行；它有意保持可见，让用户
  能发现并修复损坏的 workflow.md。

  不变量（test/regression.test.ts）：
    每个标记为 `[required · once]` 的工作流说明步骤，都必须在所属 Phase 的
    [workflow-state:*] 块中有匹配的强制执行行。路标是唯一的逐轮 channel；
    如果其中没有提到必需步骤，AI 会静默跳过它（Phase 1 规划门禁和
    Phase 3.4 提交都曾因这一缺口被跳过）。

  TAG ↔ PHASE 作用域：
    [workflow-state:no_task]      → 没有活动任务；Phase 1 之前
    [workflow-state:planning]     → 整个 Phase 1（status='planning'）
    [workflow-state:planning-inline] → Phase 1 的 Codex inline 变体
    [workflow-state:in_progress]  → Phase 2 + Phase 3.2-3.4
                                    （从 task.py start 到 task.py archive，
                                    status 始终保持 'in_progress'）
    [workflow-state:in_progress-inline] → Phase 2/3 的 Codex inline 变体
    [workflow-state:completed]    → 当前无法触发：cmd_archive 在同一次调用中
                                    改变 status 并移动目录，解析器因此失去指针
                                    （保留该块，供未来显式的
                                    in_progress→completed 转换使用）

  编辑检查清单：
    - 修改 [workflow-state:STATUS] 块时，同时检查对应 Phase 中标记为
      `[required · once]` 的说明步骤是否同步
    - 编辑后运行 `trellis update`，将新正文推送到下游用户项目
      （块级受管替换）
    - 完整运行时契约：
      .trellis/spec/cli/backend/workflow-state-contract.md
-->

## Phase Index

```
Phase 1：规划 → 分类、取得任务创建同意，然后编写规划材料
Phase 2：实施 → 仅在任务状态为 in_progress 后实施；每个行为切片执行一个红色测试 → 绿色实现 → 重构
Phase 3：完成 → 验证、更新规范、提交并收尾
```

### 请求分类

- 简单对话或小任务：只询问本轮是否应创建 Trellis 任务。如果用户拒绝，本次
  会话跳过 Trellis。
- 复杂任务：询问是否可以创建 Trellis 任务并进入规划。如果用户拒绝，不要进行
  大范围 inline 实施；应解释、澄清范围或建议更小的拆分。
- 用户同意创建任务不等于同意开始实施。仍然必须先完成规划。

### 规划材料

- `prd.md`：需求、约束和验收标准。不要在其中放置技术设计或执行清单。
- `design.md`：复杂任务的技术设计，包括边界、契约、数据流、权衡、兼容性、
  发布/回滚形态。
- `implement.md`：复杂任务的实施计划，包括有序清单、验证命令、审查门禁和
  回滚点。
- `implement.jsonl` / `check.jsonl`：子 Agent 上下文使用的规范和研究清单；
  不能替代 `implement.md`。
- 轻量任务可以只有 PRD。复杂任务在 `task.py start` 前必须有 `prd.md`、
  `design.md` 和 `implement.md`。

### 父/子任务树

当一个用户请求包含多个可独立验证的交付物时，使用父任务。父任务拥有源需求集、
任务映射、跨子任务验收标准和最终集成审查；除非它也有直接工作，否则通常不应
作为实施目标。

对可独立规划、实施、检查和归档的交付物使用子任务。父/子结构不是依赖系统；
如果一个子任务必须等待另一个，请将顺序写入子任务的 `prd.md` / `implement.md`，
并保持每个子任务的验收标准可测试。

使用 `task.py create "<title>" --slug <name> --parent <parent-dir>` 创建新子任务。
使用 `task.py add-subtask <parent> <child>` 关联现有任务，使用
`task.py remove-subtask <parent> <child>` 解除错误关联。

<!-- 逐轮路标：没有活动任务时显示（Phase 1 之前） -->

[workflow-state:no_task]
没有活动任务。先对当前轮次分类，并在创建任何 Trellis 任务前征得任务创建同意。
简单对话/小任务：只询问本轮是否应创建 Trellis 任务。如果用户拒绝，本次会话跳过 Trellis。
复杂任务：询问用户是否可以创建 Trellis 任务并进入规划阶段。如果用户拒绝，应解释、澄清范围或建议更小的拆分。
[/workflow-state:no_task]

### Phase 1：规划
- 1.0 创建任务 `[required · once]`（仅在取得任务创建同意后）
- 1.1 需求探索 `[required · repeatable]`（`prd.md`；复杂任务还需要 `design.md` + `implement.md`）
- 1.2 研究 `[optional · repeatable]`
- 1.3 配置上下文 `[required · once]`：Claude Code、Cursor、OpenCode、Codex、Kiro、Gemini、Qoder、CodeBuddy、Copilot、Droid、Pi（仅子 Agent 派发平台；inline 平台跳过）
- 1.4 激活任务 `[required · once]`（先通过审查门禁，再运行 `task.py start`；status → in_progress）
- 1.5 完成条件

<!-- 逐轮路标：整个 Phase 1 期间显示（status='planning'） -->

[workflow-state:planning]
加载 `trellis-brainstorm`，停留在规划阶段。
轻量任务：`prd.md` 可以足够。复杂任务：完成 `prd.md`、`design.md` 和 `implement.md`；在 `task.py start` 前请求审查。
TDD 规划门禁：在 `task.py start` 前记录可观察行为切片、被测公开接口和 mock 边界。
多交付物范围：考虑使用父任务和可独立验证的子任务；依赖关系必须写入子任务材料，不能由任务树位置暗示。
子 Agent 模式：启动前将 `implement.jsonl` 和 `check.jsonl` 整理为规范/研究清单。
[/workflow-state:planning]

<!-- 逐轮路标：codex.dispatch_mode=inline 时在整个 Phase 1 期间显示。
     这是 Codex 专属、需选择启用的 [workflow-state:planning] 替代块。主 Agent
     在 Phase 2 直接编辑代码，因此跳过 JSONL 整理；inline 工作流加载
     `trellis-before-dev`，而不是向子 Agent 注入 JSONL。 -->

[workflow-state:planning-inline]
加载 `trellis-brainstorm`，停留在规划阶段。
轻量任务：`prd.md` 可以足够。复杂任务：完成 `prd.md`、`design.md` 和 `implement.md`；在 `task.py start` 前请求审查。
TDD 规划门禁：在 `task.py start` 前记录可观察行为切片、被测公开接口和 mock 边界。
多交付物范围：考虑使用父任务和可独立验证的子任务；依赖关系必须写入子任务材料，不能由任务树位置暗示。
Inline 模式：跳过 JSONL 整理；Phase 2 通过 `trellis-before-dev` 读取材料/规范。
[/workflow-state:planning-inline]

### Phase 2：实施
- 2.1 实施 `[required · repeatable]`
- 2.2 质量检查 `[required · repeatable]`
- 2.3 回滚 `[on demand]`

<!-- 逐轮路标：status='in_progress' 时显示。
     作用域：整个 Phase 2 + Phase 3.2-3.4（从 task.py start 到
     task.py archive，status 始终保持 'in_progress'；只有 archive 会改变它）。
     因此正文必须覆盖从实施到提交的每个必需步骤，包括 Phase 3.3 规范更新和
     Phase 3.4 提交。 -->

子 Agent 派发协议适用于所有平台和所有子 Agent，包括 class-2 Codex/Copilot/Gemini/Qoder 与 `trellis-research`：每个派发 prompt 都先以 `Active task: <task path from task.py current>` 开头，再写角色专属指令。

[workflow-state:in_progress]
流程：选择一个行为 -> 红色测试 -> 绿色实现 -> 保持绿色时重构 -> `trellis-check` -> `trellis-update-spec` -> 提交（Phase 3.4）-> `/trellis:finish-work`。
主会话默认派发 implement/check 子 Agent。子 Agent 自我豁免：如果已经作为 `trellis-implement` 运行，不要再启动 `trellis-implement` 或 `trellis-check`；如果已经作为 `trellis-check` 运行，不要再启动 `trellis-check` 或 `trellis-implement`。只有主会话可以派发。
派发 prompt 以 `Active task: <task path from task.py current>` 开头。上下文读取顺序：JSONL 条目 -> `prd.md` -> 存在时的 `design.md` -> 存在时的 `implement.md`。
[/workflow-state:in_progress]

<!-- 逐轮路标：codex.dispatch_mode=inline 且 status='in_progress' 时显示。
     这是 Codex 专属、需选择启用的 [workflow-state:in_progress] 替代块。
     主会话直接编辑代码，而不是派发子 Agent。 -->

[workflow-state:in_progress-inline]
流程：`trellis-before-dev` -> 选择一个行为 -> 红色测试 -> 绿色实现 -> 保持绿色时重构 -> `trellis-check` -> 验证 -> `trellis-update-spec` -> 提交（Phase 3.4）-> `/trellis:finish-work`。
Inline 模式下不要派发 implement/check 子 Agent。
上下文读取顺序：`prd.md` -> 存在时的 `design.md` -> 存在时的 `implement.md`，再加上 Skill 加载的相关规范/研究材料。
[/workflow-state:in_progress-inline]

### Phase 3：完成
- 3.2 调试复盘 `[on demand]`
- 3.3 规范更新 `[required · once]`
- 3.4 提交改动 `[required · once]`
- 3.5 收尾提醒

> 注意：步骤 3.1 已并入 2.2（最后一轮全范围检查）和 3.4（提交前导检查）。保留编号稳定，避免破坏外部引用。

<!-- 逐轮路标：status='completed' 时显示。
     当前在正常流程中无法触发：cmd_archive 在同一次调用中写入
     status='completed' 并将任务目录移动到 archive/，活动任务解析器因此失去
     指针，hook 永远不会对已归档任务触发。保留该块，供未来重新设计状态转换
     （例如显式的 in_progress→completed 命令）。通过与其他活动块相同的规范
     channel 进行编辑。 -->

[workflow-state:completed]
代码已提交。运行 `/trellis:finish-work`；如果工作区仍有改动，先返回 Phase 3.4。
[/workflow-state:completed]

### 规则

1. 识别当前所处 Phase，再从该 Phase 的下一步继续
2. 在每个 Phase 内按顺序执行；不能跳过 `[required]` 步骤
3. Phase 可以回滚，例如实施时发现 PRD 缺陷 -> 返回规划修复，再重新进入实施
4. 如果输出已存在，则跳过标记为 `[once]` 的步骤，不要重复运行
5. 根据材料是否存在判断下一步；缺少 `design.md` / `implement.md` 对轻量任务有效，对复杂任务则表示规划不完整

### 活动任务路由

活动任务中的用户请求匹配以下意图时，先进行路由，再按需加载详细 Phase 步骤。

[Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

- 规划或需求不明确 -> `trellis-brainstorm`。
- `in_progress` 实施/检查 -> 派发 `trellis-implement` / `trellis-check`。
- 重复调试 -> `trellis-break-loop`；规范更新 -> `trellis-update-spec`。

[/Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

[codex-inline, Kilo, Antigravity, Devin]

- 规划或需求不明确 -> `trellis-brainstorm`。
- 编辑前 -> `trellis-before-dev`；编辑后 -> `trellis-check`。
- 重复调试 -> `trellis-break-loop`；规范更新 -> `trellis-update-spec`。

[/codex-inline, Kilo, Antigravity, Devin]

### 防护规则

- 同意创建任务不等于同意实施；材料审查后运行 `task.py start`，才能开始实施。
- 轻量任务可以只有 PRD；复杂任务需要 `design.md` + `implement.md`。
- 规划必须持久化到任务材料；报告完成前必须运行检查。

### 加载步骤细则

在每个步骤运行以下命令获取详细指南：

```bash
python ./.trellis/scripts/get_context.py --mode phase --step <step>
# e.g. python ./.trellis/scripts/get_context.py --mode phase --step 1.1
```

---

## Phase 1: Plan

目标：对请求分类，在需要任务时取得任务创建同意，并生成实施前所需的规划材料。

#### 1.0 创建任务 `[required · once]`

只有取得任务创建同意后才能创建任务目录。该命令将 status 设为 `planning`、
写入 `task.json`、创建默认 `prd.md`，并在会话身份可用时自动将新任务设为目标：

```bash
python ./.trellis/scripts/task.py create "<task title>" --slug <name>
```

`--slug` 只包含人类可读名称。**不要**包含 `MM-DD-` 日期前缀；
`task.py create` 会自动添加该前缀。

对于任务树，先创建父任务，再用 `--parent <parent-dir>` 创建每个子任务。不要仅因
存在子任务就启动父任务；应启动拥有下一个可独立验证交付物的子任务。

该命令成功后，逐轮路标会自动切换为 `[workflow-state:planning]`，要求 AI
停留在规划阶段。

此处只运行 `create`，不要同时运行 `start`。`start` 会将 status 改为
`in_progress`，导致规划材料尚未审查，路标就切换到实施阶段。将 `start` 留到
步骤 1.4。

当 `python ./.trellis/scripts/task.py current --source` 已指向任务时跳过本步骤。

#### 1.1 需求探索 `[required · repeatable]`

加载 `trellis-brainstorm` Skill，并按其指南与用户交互式探索需求。对于此 TDD
工作流，需求探索必须在实施开始前产出可观察行为。

Brainstorm Skill 会指导你：
- 每次只问一个问题
- 优先研究，而不是询问用户
- 优先提供选项，而不是提出开放式问题
- 每次收到用户回答后立即更新 `prd.md`
- 交付物可独立验证时，将大范围拆分为父任务和子任务
- 让 `prd.md` 聚焦需求和验收标准
- 记录行为切片：公开接口、输入/操作、预期结果，以及需要或不应 mock 的边界
- 对复杂任务，在实施开始前生成 `design.md` 和 `implement.md`

考虑父/子任务拆分时：
- 一个请求包含多个可独立验证的交付物时，使用父任务。
- 父任务拥有源需求、子任务映射、跨子任务验收标准和最终集成审查。
- 子任务拥有可独立规划、实施、检查和归档的实际交付物。
- 父/子结构不是依赖系统。如果子任务 B 依赖子任务 A，将顺序写入子任务 B 的
  `prd.md` / `implement.md`。
- 启动拥有下一个交付物的子任务。除非父任务本身有直接实施工作，否则不要启动父任务。

需求变化时返回本步骤并修订相关材料。至少第一个行为切片具体到足以编写失败
测试后，才能开始实施。

#### 1.2 研究 `[optional · repeatable]`

研究可以在需求探索期间的任何时刻进行，且不限于本地代码。可以使用任何可用
工具（MCP Server、Skill、Web 搜索等）查询外部信息，包括第三方库文档、行业
实践和 API 参考等。

[Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

启动 Research 子 Agent：

- **Agent 类型**：`trellis-research`
- **任务说明**：研究 <具体问题>
- **关键要求**：研究输出**必须**持久化到 `{TASK_DIR}/research/`

[/Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

[codex-inline, Kilo, Antigravity, Devin]

直接在主会话中研究，并将发现写入 `{TASK_DIR}/research/`。对于 `codex-inline`，
这可以避开 `fork_turns="none"` 隔离；该隔离会阻止 `trellis-research` 子 Agent
解析活动任务路径。

[/codex-inline, Kilo, Antigravity, Devin]

**研究材料约定**：
- 每个研究主题一个文件，例如 `research/auth-library-comparison.md`
- 在文件中记录第三方库使用示例、API 参考和版本约束
- 记下发现的相关规范文件路径，供之后引用

需求探索和研究可以自由交错；暂停对话研究技术问题，然后返回与用户讨论。

**关键原则**：研究输出必须写入文件，不能只留在聊天中。对话会被压缩，文件不会。

#### 1.3 配置上下文 `[required · once]`

[Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

整理 `implement.jsonl` 和 `check.jsonl`，使 Phase 2 子 Agent 获得正确的
规范/研究上下文。`task create` 已用一行自描述的 `_example` 初始化这些文件；
本步骤负责填入真实条目。

**位置**：`{TASK_DIR}/implement.jsonl` 和 `{TASK_DIR}/check.jsonl`（已经存在）。

**格式**：每行一个 JSON 对象，格式为 `{"file": "<path>", "reason": "<why>"}`。
路径相对于仓库根目录。

**应放入的内容**：
- **测试规范**：与本任务相关的单元/集成测试约定和 mock 策略文件
- **规范文件**：`.trellis/spec/<package>/<layer>/index.md`，以及与本任务相关的
  具体指南文件（`error-handling.md`、`conventions.md` 等）
- **研究文件**：子 Agent 需要查阅的 `{TASK_DIR}/research/*.md`

**不应放入的内容**：
- 代码文件（`src/**`、`packages/**/*.ts` 等）：子 Agent 会在实施期间自行读取，
  不在此预注册
- 即将修改的文件：原因相同

**两个文件的职责划分**：
- `implement.jsonl` -> Implement 子 Agent 正确编写代码所需的规范 + 研究
- `check.jsonl` -> Check 子 Agent 所需的规范（质量指南、检查约定；需要时包含相同研究）

这些清单不能替代 `implement.md`。`implement.md` 是复杂任务的人类可读实施计划；
JSONL 文件只列出要注入或加载的上下文文件。

**如何发现相关规范**：

```bash
python ./.trellis/scripts/get_context.py --mode packages
```

该命令列出每个包及其规范层和路径。选择与本任务领域匹配的条目。

**如何追加条目**：

可以在编辑器中直接编辑 JSONL 文件，也可以使用：

```bash
python ./.trellis/scripts/task.py add-context "$TASK_DIR" implement "<path>" "<reason>"
python ./.trellis/scripts/task.py add-context "$TASK_DIR" check "<path>" "<reason>"
```

存在真实条目后删除初始化的 `_example` 行。此操作可选；消费者会自动跳过该行。

就绪门禁：在 `task.py start` 前，`implement.jsonl` 和 `check.jsonl` 都必须
至少包含一个真实的 `{"file": "...", "reason": "..."}` 条目。只有初始化
`_example` 行并不代表就绪。

只有两个文件都已包含整理后的真实条目时，才跳过本步骤。

[/Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

[codex-inline, Kilo, Antigravity, Devin]

跳过本步骤。Phase 2 由 `trellis-before-dev` Skill 直接加载上下文。

[/codex-inline, Kilo, Antigravity, Devin]

#### 1.4 激活任务 `[required · once]`

材料审查后，将任务 status 改为 `in_progress`：

```bash
python ./.trellis/scripts/task.py start <task-dir>
```

轻量任务可以只有 `prd.md`。复杂任务启动前必须存在并审查 `prd.md`、
`design.md` 和 `implement.md`。在子 Agent 派发平台上，`implement.jsonl` 和
`check.jsonl` 启动前都必须包含整理后的真实条目。为保持兼容性，运行时消费者
允许清单缺失或只有初始化行，但这种容忍不代表规划已就绪。

该命令成功后，路标自动切换为 `[workflow-state:in_progress]`，随后继续执行
Phase 2 / 3。

如果 `task.py start` 报告会话身份错误（无法从 hook 输入、
`TRELLIS_CONTEXT_ID` 或平台原生会话环境变量取得上下文键），按错误提示设置
会话身份后重试。

#### 1.5 完成条件

| 条件 | 必需 |
|------|:---:|
| `prd.md` 存在 | ✅ |
| 用户确认任务应进入实施阶段 | ✅ |
| 已运行 `task.py start`（status = in_progress） | ✅ |
| `research/` 有研究材料（复杂任务） | 建议 |
| `design.md` 存在（复杂任务） | ✅ |
| `implement.md` 存在（复杂任务） | ✅ |

[Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

| `implement.jsonl` 和 `check.jsonl` 各包含至少一个整理后的真实条目（初始化行不计） | ✅ |

[/Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

---

## Phase 2: Execute

目标：将已审查的规划材料转化为通过质量检查的代码。

#### 2.1 实施 `[required · repeatable]`

一次只运行一个行为切片。不要先编写所有测试，也不要在看到失败测试前实施多个行为。

对每个行为：

1. 从 `prd.md` 或 `implement.md` 选择下一个行为。
2. 识别要测试的公开接口。优先选择能够表达该行为的最小用户边界或模块边界。
3. 编写一个描述预期行为的失败测试。实施开始前，测试必须因正确原因失败。
4. 实施使测试通过的最小代码路径。
5. 运行专项测试。如果失败，修复实现或测试契约，然后重新运行。
6. 变绿后，只在代码确有需要时重构。每次重构后重新运行专项测试。
7. 在 `implement.md` 或任务注记中将行为标记为完成，再进入下一个行为。

测试规则：

- 测试公开行为，而不是私有方法或内部调用顺序。
- 只 mock 系统边界：网络、时间、随机性、文件系统、子进程或外部服务。
- 对边界协作者优先使用依赖注入。
- 保持测试可读，使其成为可执行需求。

[Claude Code, Cursor, OpenCode, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

启动 Implement 子 Agent：

- **Agent 类型**：`trellis-implement`
- **任务说明**：根据已审查的任务材料一次实施一个行为切片：通过公开接口编写红色测试、完成绿色实现、只在保持绿色时重构；最后运行专项测试以及项目 lint 和 type-check
- **派发 prompt 防护**：告知启动的 Agent 已经是 `trellis-implement` 子 Agent，必须直接实施，不得再启动 `trellis-implement` / `trellis-check`

平台 hook/插件自动处理：
- 读取 `implement.jsonl`，并将引用的规范/研究文件注入 Agent prompt
- 注入 `prd.md`，以及存在时的 `design.md` 和 `implement.md`

[/Claude Code, Cursor, OpenCode, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

[codex-sub-agent]

启动 Implement 子 Agent：

- **Agent 类型**：`trellis-implement`
- **任务说明**：根据已审查的任务材料一次实施一个行为切片：通过公开接口编写红色测试、完成绿色实现、只在保持绿色时重构；最后运行专项测试以及项目 lint 和 type-check
- **派发 prompt 防护**：prompt **必须**以 `Active task: <task path>` 开头，然后明确说明启动的 Agent 已经是 `trellis-implement`，必须直接实施，不得再启动 `trellis-implement` / `trellis-check`

Codex 子 Agent 定义自动处理上下文加载要求：
- 使用 `task.py current --source` 解析活动任务，再读取 `prd.md`，以及存在时的 `design.md` 和 `implement.md`
- 读取 `implement.jsonl`，并要求 Agent 在编码前加载每个引用的规范/研究文件

[/codex-sub-agent]

[Kiro]

启动 Implement 子 Agent：

- **Agent 类型**：`trellis-implement`
- **任务说明**：根据已审查的任务材料一次实施一个行为切片：通过公开接口编写红色测试、完成绿色实现、只在保持绿色时重构；最后运行专项测试以及项目 lint 和 type-check
- **派发 prompt 防护**：告知启动的 Agent 已经是 `trellis-implement` 子 Agent，必须直接实施，不得再启动 `trellis-implement` / `trellis-check`

平台前置指令自动处理上下文加载要求：
- 读取 `implement.jsonl`，并将引用的规范/研究文件注入 Agent prompt
- 注入 `prd.md`，以及存在时的 `design.md` 和 `implement.md`

[/Kiro]

[codex-inline, Kilo, Antigravity, Devin]

1. 加载 `trellis-before-dev` Skill，读取项目指南
2. 读取 `{TASK_DIR}/prd.md`，再读取存在时的 `design.md` 和 `implement.md`
3. 查阅 `{TASK_DIR}/research/` 下的材料
4. 运行上述行为切片循环，直到任务材料中的验收行为变绿
5. 运行项目 lint 和 type-check

[/codex-inline, Kilo, Antigravity, Devin]

#### 2.2 质量检查 `[required · repeatable]`

[Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

启动 Check 子 Agent：

- **Agent 类型**：`trellis-check`
- **任务说明**：依据规范和任务材料审查所有代码改动；直接修复发现的问题；确保 lint 和 type-check 通过
- **派发 prompt 防护**：告知启动的 Agent 已经是 `trellis-check` 子 Agent，必须直接审查/修复，不得再启动 `trellis-check` / `trellis-implement`

Check Agent 的工作：
- 依据规范审查代码改动
- 依据 `prd.md`、存在时的 `design.md` 和存在时的 `implement.md` 审查代码改动
- 验证每个已完成行为都有测试，且测试在没有实现时失败，并通过公开接口通过
- 验证 mock 仅限系统边界，不针对内部实现细节
- 自动修复发现的问题
- 运行专项测试、lint 和 typecheck 进行验证

[/Claude Code, Cursor, OpenCode, codex-sub-agent, Kiro, Gemini, Qoder, CodeBuddy, Copilot, Droid, Pi]

[codex-inline, Kilo, Antigravity, Devin]

加载 `trellis-check` Skill，并按其指南验证代码：
- 规范符合性
- 行为测试通过公开接口通过
- 移除或禁用实现时，新测试因正确原因失败
- Mock 仅限系统边界
- lint / type-check / 测试
- 跨层一致性（改动跨层时）

如果发现问题 -> 修复 -> 重新检查，直到变绿。

[/codex-inline, Kilo, Antigravity, Devin]

**最终检查（Phase 3.4 提交前）**：任务最后一次 2.2 必须覆盖全范围，不能只检查
最新实施批次。使用 `python ./.trellis/scripts/get_context.py --mode packages`
列出所有受影响的包，再加载每个包规范索引中的质量检查章节。这样可以发现迭代
中途局部 2.2 无法发现的跨层/多包问题。

#### 2.3 回滚 `[on demand]`

- `check` 发现 PRD 缺陷 -> 返回 Phase 1，修复 `prd.md`，然后重新执行 2.1
- 实施出错 -> 回退当前行为切片，从失败测试开始重新执行 2.1
- 需要更多研究 -> 研究（与 Phase 1.2 相同），将发现写入 `research/`

---

## Phase 3: Finish

目标：确保代码质量、沉淀经验并记录工作。

#### 3.2 调试复盘 `[on demand]`

如果本任务涉及重复调试（同一问题修复多次），加载 `trellis-break-loop` Skill：
- 对根因分类
- 解释此前修复为何失败
- 提出预防措施

目标是沉淀调试经验，避免同类问题再次发生。

#### 3.3 规范更新 `[required · once]`

加载 `trellis-update-spec` Skill，审查本任务是否产生了值得记录的新知识：
- 新发现的模式或约定
- 遇到的易错点
- 新技术决策

据此更新 `.trellis/spec/` 下的文档。即使结论是“无需更新”，也要完成判断过程。

#### 3.4 提交改动 `[required · once]`

**规范同步前导检查**：起草提交前先问：本任务是否修复了缺陷，或发现了应写入
`.trellis/spec/` 的非显然知识，从而避免未来的自己（或 AI）重复犯错？如果是，
先返回 Phase 3.3；规范改动应进入同一任务的提交批次，不能成为被遗忘的后续事项。

AI 驱动本任务代码改动的批量提交，使 `/finish-work` 随后能在干净工作区运行。
目标：**先**生成工作提交，再生成记账提交（归档 + 日志），两者绝不交错。

**逐步操作**：

1. **检查未提交状态**：
   ```bash
   git status --porcelain
   ```
   记录每个未提交路径。如果工作树干净，跳到 3.5。

2. **从近期历史了解提交风格**，使起草的消息保持一致：
   ```bash
   git log --oneline -5
   ```
   记录前缀约定（`feat:` / `fix:` / `chore:` / `docs:` 等）和长度风格。提交主题
   与正文默认使用简体中文；Conventional Commits 的 `type(scope):` 可保留英文标识符。

3. **将未提交文件分为两组**：
   - **AI 在当前任务中编辑**：当前会话通过编辑工具修改的文件，或当前 Trellis
     任务材料明确记录并归属本任务的跨会话改动。你知道改了什么以及原因。
   - **无法识别**：未编辑且当前任务材料未记录的改动，可能是用户手动改动、
     之前任务遗留的 WIP 或无关工作。不要静默包含这些文件。

4. **起草提交计划**。将 AI 编辑的文件分为逻辑提交，每个一致的变更单元一个
   提交，而不是每个文件一个提交。每个条目包含 `<中文提交消息>` + 文件列表。
   在底部单独列出无法识别的文件。

5. **只展示一次计划，并请求一次性确认**。格式：
   ```
   建议的提交（按顺序）：
     1. <中文提交消息>
        - <file>
        - <file>
     2. <中文提交消息>
        - <file>

   无法识别的未提交文件（不包含在任何提交中，请确认纳入或排除）：
     - <file>
     - <file>

   回复“ok”/“行”执行。回复修改意见，或回复“我自己来”/“manual”中止。
   ```

6. **确认后**：按顺序为每个批次运行 `git add <files>` +
   `git commit -m "<msg>"`。不要 amend，不要 push。

7. **拒绝后**（用户回复“不行”/“我自己来”/“manual”或反对计划）：停止。
   不要尝试第二份计划。用户会手动提交；用户确认后跳到 3.5。

**规则**：
- 任何位置都不得使用 `git commit --amend`；采用三阶段三提交流程
  （工作提交 -> 归档提交 -> 日志提交）。
- 本步骤绝不 push 到远程。
- 如果用户接受文件分组但希望修改消息措辞，编辑消息并再确认一次；如果用户拒绝
  分组，则退出到手动模式。
- 批量计划只使用一个 prompt；不要逐个提交询问。

#### 3.5 收尾提醒

完成上述步骤后，提醒用户可以运行 `/finish-work` 收尾（归档任务并记录会话）。

---

## 定制 Trellis（用于 Fork）

本节面向希望修改 Trellis 工作流本身的开发者。所有定制都通过编辑本文件完成；
脚本只负责解析。

### 修改步骤含义

编辑上方 Phase 1 / 2 / 3 章节中对应步骤的说明正文。关键不变量：
- 没有活动任务时必须先分类，并在创建 Trellis 任务前征得任务创建同意。
- 规划必须区分只有 PRD 的轻量任务，以及启动前要求 `prd.md`、`design.md` 和
  `implement.md` 的复杂任务。
- 每条必需执行路径都必须确保在 `/trellis:finish-work` 前能够到达 Phase 3.4
  提交提醒。

所有 tag 块都位于上方 `## Phase Index` 章节中，紧跟在各 Phase 摘要之后：

| 作用域 | 对应 tag |
|---|---|
| 没有活动任务（Phase 1 之前） | `[workflow-state:no_task]`（位于 Phase Index ASCII 图之后） |
| 整个 Phase 1（任务已创建 -> 准备实施） | `[workflow-state:planning]`（位于 Phase 1 摘要之后） |
| Codex inline Phase 1 | `[workflow-state:planning-inline]` |
| Phase 2 + Phase 3.2–3.4（实施 + 检查 + 收尾） | `[workflow-state:in_progress]`（位于 Phase 2 摘要之后） |
| Codex inline Phase 2 + Phase 3.2–3.4 | `[workflow-state:in_progress-inline]` |
| Phase 3.5 之后（已归档） | `[workflow-state:completed]`（位于 Phase 3 摘要之后；**当前无法触发**） |

### 修改逐轮 Prompt 文本

直接编辑对应 `[workflow-state:STATUS]` 块的正文。编辑后，如果你是模板维护者，
运行 `trellis update`；如果是在定制自己的项目，重启 AI 会话。无需修改脚本。

### 添加自定义状态

添加新块：

```
[workflow-state:my-status]
你的逐轮 prompt 文本
[/workflow-state:my-status]
```

约束：
- STATUS 字符集：`[A-Za-z0-9_-]+`，允许下划线和连字符，例如 `in-review`、
  `blocked-by-team`
- 生命周期 hook 必须将 `task.json.status` 写为自定义值，否则永远不会读取该 tag
- 生命周期 hook 位于 `task.json.hooks.after_*`，并绑定到
  `after_create / after_start / after_finish / after_archive` 之一

### 添加生命周期 Hook

向 `task.json` 添加 `hooks` 字段：

```json
{
  "hooks": {
    "after_finish": [
      "your-script-or-command-here"
    ]
  }
}
```

支持的事件：`after_create / after_start / after_finish / after_archive`。注意，
`after_finish` 不等于状态变化，它只清除活动任务指针；“任务已完成”通知应使用
`after_archive`。

### 完整契约

工作流状态机的运行时契约、所有 status 写入方的位置、伪状态
（`no_task` / `stale_<source_type>`）、hook 可达性矩阵和其他深入细节见：

- `.trellis/spec/cli/backend/workflow-state-contract.md`：运行时契约 + 写入方表 + 测试不变量
- `.trellis/scripts/inject-workflow-state.py`：实际解析器（只读取 workflow.md，不含内嵌文本）
