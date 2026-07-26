# Trellis 与 Superpowers 边界

## 唯一生命周期

由 AI 按请求规模、复杂度、风险和持久化需要判断是否创建 Trellis 任务：

- 一次性答复、简单只读查询，以及范围清楚、低风险、局部且可在当前会话完成并验证的
  小改动，可以直接处理。
- 跨模块或多阶段、需求或方案复杂、涉及安全/配置迁移/发布/卸载/数据保留、需要
  持久化 PRD/设计/进度/验证证据、可能跨会话，或用户明确要求时，必须创建任务。
- 判断需要任务时直接创建并简短告知用户，无需另行征求创建许可。已有相关任务时
  继续使用；不得把不相关工作混入活动任务。

一旦创建任务，其完整生命周期只由 Trellis `tdd` 工作流负责：

```text
create → PRD → design → implement → start
→ red/green/refactor → check → update spec → commit → finish → archive
```

Trellis 负责任务指针、规划材料、研究、上下文选择、实施检查点、质量检查、规范更新和开发日志。

## Codex 模式

`.trellis/config.yaml` 必须保持：

```yaml
codex:
  dispatch_mode: inline
```

`inline` 只规定核心 Implement/Check 由主会话直接执行，不等于禁止所有辅助子 Agent。
主 Agent 仍拥有用户沟通、任务状态、TDD 顺序、最终修复与验证、规范更新、提交和归档。

满足以下条件时，主 Agent 应考虑派发受控辅助子 Agent：

- 存在两个或更多可以独立完成的读多写少分支，例如规范/代码探索、测试或日志分析、
  文档核验和只读审查；
- 派发能够显著减少主线程的中间输出污染，或能并行取得相互独立的证据；
- Prompt 首行包含 `Active task: <task path>`，依赖的用户决定已经写入任务材料，
  Agent 能从 PRD、设计、计划、JSONL、规范或研究文件恢复上下文；
- Agent 默认只读，或只写当前任务 `research/` 下的独立文件；不得并发修改重叠文件。

辅助 Agent 的结论不能替代主 Agent 的核验。核心代码实施、问题修复和 Trellis check
继续 Inline；写入型 Implement/Check 子 Agent、Channel Worker 或并行写入只有在用户明确
要求且另有隔离与验证方案时才启用。

子 Agent 是否继承父会话历史属于宿主运行时实现细节。项目长期契约只依赖持久化任务
材料和显式活动任务路径，不把某个固定 `fork_turns` 值视为跨版本保证。

### 派发示例

正确：一个 Agent 只读检查安全边界，另一个 Agent 分析测试日志；两者收到相同的活动任务
路径，返回带文件证据的报告，主 Agent 逐项验证后再修复。

正确：Research Agent 只把一个独立主题写入当前任务 `research/<topic>.md`，不修改代码、
规范、平台配置或 Git。

错误：Inline 模式把核心实现或 Trellis check 整体交给子 Agent，或让多个 Agent 同时修改
同一批文件，然后直接依据摘要宣布完成。

## 保留的 Superpowers

- `using-superpowers`：发现适用能力。
- `systematic-debugging`：异常、测试失败或重复修复前查根因。
- `receiving-code-review`：验证审查意见的技术正确性。
- `verification-before-completion`：完成声明必须基于新鲜证据。

## Trellis 任务中不重复使用

- brainstorming
- writing-plans
- test-driven-development
- executing-plans
- subagent-driven-development
- finishing-a-development-branch

这些阶段由 Trellis `tdd` 工作流统一负责，避免重复 PRD、设计、实施计划、TDD 状态和收尾菜单。完成前验证只约束证据真实性，不创建第二套任务状态。

## 任务门禁

- 复杂任务在 `task.py start` 前必须有 `prd.md`、`design.md` 和 `implement.md`。
- 自动创建任务或进入规划不自动等于批准实施；必须确认用户已明确要求或批准实施。
- 工作开始、每个阶段和暂停前更新 `implement.md`。
- 提交前执行 Trellis check、完整验证和 spec 更新判断。
