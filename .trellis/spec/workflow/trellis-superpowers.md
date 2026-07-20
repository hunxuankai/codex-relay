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

不启用 channel 或 sub-agent dispatch。Codex 在主会话中读取任务和相关规范、直接实施和检查。

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
