# 技术设计：Trellis 混合子代理策略

## 1. 设计目标

把“谁拥有任务生命周期”和“谁可以承担辅助认知工作”拆成两个独立维度：

- 主 Agent/Trellis 始终拥有任务状态、用户决策、TDD 顺序、最终验证、规范更新、提交和归档。
- 辅助子代理只处理边界清晰的研究、探索、测试/日志分析或只读审查；它们的结果必须
  通过任务文件或结构化摘要回到主 Agent。
- `codex.dispatch_mode: inline` 继续作为核心 Implement/Check 的默认模式，避免一次策略
  调整把所有任务切换为写入型子代理。

## 2. 策略分层

| 工作类型 | 默认执行者 | 是否允许辅助子代理 | 写入边界 |
| --- | --- | --- | --- |
| 需求、设计、TDD 切片选择 | 主 Agent | 否 | 任务材料 |
| 核心实现与共享文件修改 | 主 Agent Inline | 仅在独立子任务且串行时可申请 | 明确文件集合 |
| 代码库探索、规范/文档核验 | 主 Agent 或 Research/Explorer | 是 | 只读或当前任务 `research/` |
| 测试日志、失败分类、静态审查 | 主 Agent 或只读 Reviewer | 是 | 默认只读 |
| 最终修复、全范围验证、提交/归档 | 主 Agent | 否 | 代码、规范、Git |
| 多轮对等协作 | 主 Agent + Channel | 仅在确有持久协作需求时 | Channel 角色卡约束 |

## 3. 上下文契约

辅助 Agent 不依赖某个 Codex 版本的对话继承实现。派发时：

1. Prompt 首行写入 `Active task: <task path>`。
2. Agent 从任务路径读取 `prd.md`，再读取存在时的 `design.md`、`implement.md`，以及
   任务 JSONL 指向的规范/研究文件。
3. 主 Agent 的临时聊天决策如果影响行为，必须先写入任务材料或研究文件，再派发依赖
   该决策的 Agent。
4. 返回结果包含文件路径、证据、未解决风险和建议动作；不能只返回无法复核的结论。

`fork_turns` 仅视为宿主运行时实现细节，不写入项目长期契约。无论宿主是否继承历史，
任务文件都必须足以恢复工作。

## 4. 并发与安全边界

- 不允许两个写入型 Agent 同时修改重叠文件或同一任务材料。
- 只读研究可以并行；写入 `research/` 时每个主题使用独立文件名。
- 子 Agent 不得修改 `AGENTS.md`、`.trellis/workflow.md`、`.trellis/spec/`、平台配置、
  用户级 Codex/Relay 目录或 Git 历史，除非主 Agent 明确把它作为独立交付物派发。
- 任何涉及真实认证文件、密钥、受管配置事务或默认用户路径的实验必须使用安全临时目录
  和脱敏 fixture；不能因子代理隔离而放宽项目安全规则。

## 5. 兼容性与回滚

- 不新增 `dispatch_mode` 枚举，不改变现有 `inline` / `sub-agent` 工作流解析逻辑。
- 更新说明文字和提示块时保留现有平台标记，确保 `get_context.py --mode phase` 的路由
  输出不变。
- 若后续试点证明写入型子代理稳定，可另建任务将 `dispatch_mode` 或任务级覆盖策略调整
  为串行 Sub-agent；本任务不预先启用该切换。
- 如发现策略文字导致误派发，回滚本任务涉及的规则/提示文件即可，不影响应用运行时配置。

## 6. 影响文件

- `AGENTS.md`
- `.trellis/spec/workflow/trellis-superpowers.md`
- `.trellis/workflow.md`
- `.trellis/config.yaml`
- `.trellis/scripts/common/workflow_phase.py`
- `.codex/hooks/inject-workflow-state.py`
- 必要时同步 `.codex/agents/trellis-*.toml` 中的上下文/递归说明

