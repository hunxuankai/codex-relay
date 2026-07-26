# 评估并放宽 Trellis 子代理派发策略

## 背景与已确认事实

- 当前仓库选择 `codex.dispatch_mode: inline`，并要求 Trellis `tdd` 独占任务生命周期。
- 工作流已经存在 Codex `sub-agent` 分支；Implement Agent 会通过 `Active task`、
  `implement.jsonl`、`prd.md`、`design.md` 和 `implement.md` 自行加载任务上下文。
- 现有配置、Hook 和阶段解析器把 `fork_turns="none"` 写成 Inline 的固定技术理由，
  但 Agent 定义同时记录了 `fork_turns="all"` 的递归协作死锁防护，表述存在版本耦合和
  语义不一致。
- 当前 Codex 官方手册建议将探索、测试、日志分析和总结等读多写少工作交给子代理，
  同时警告并行写代码会增加冲突和协调成本。
- 用户已批准继续把前一轮审视结果落实为项目工作流调整；本任务不修改应用业务代码，
  不读取或修改真实用户级 Codex/Relay 配置，也不触碰现有未跟踪的 `bash.exe.stackdump`。

## 目标

将项目规则从“全面禁止子代理”调整为可审计的混合策略：主 Agent 继续拥有核心
Trellis 生命周期和最终责任，受控子代理可承担边界清晰的研究、测试分析和只读审查，
并使持久化文档不再依赖某个特定 Codex 上下文继承实现。

## 需求

- **R1 生命周期所有权不变**：任务创建、规划材料、用户决策、TDD 顺序、最终验证、
  规范更新、提交和归档仍由主 Agent/Trellis 负责。
- **R2 Inline 默认不变**：`codex.dispatch_mode` 继续为 `inline`；本任务不把所有任务
  全局切换到 `sub-agent`，也不在当前任务中实际派发 Implement/Check 子代理。
- **R3 允许受控辅助代理**：项目规则明确允许在满足独立性、范围、权限和写入边界时，
  使用研究、探索、测试/日志分析或只读审查子代理；需要共享写入时必须串行并由主 Agent
  复核，不允许多个代理并发修改重叠文件。
- **R4 上下文契约稳定**：辅助代理 Prompt 必须携带活动任务路径，子代理优先从任务材料
  和规范文件加载上下文；文档不得把 `fork_turns="none"` 或 `all` 描述为跨版本不变的
  Codex 契约。
- **R5 平台文件一致**：同步更新 `AGENTS.md`、工作流规范、Codex 状态注入/阶段解析器
  的说明，以及必要的本地 Agent 角色卡；保留现有 `sub-agent` 工作流分支的兼容性。
- **R6 安全与证据**：不修改真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`，
  不引入密钥或完整认证内容；验证结果必须区分静态检查、脚本测试和人工策略审查。

## 验收标准

- [x] 项目长期规则不再将“禁止任何子代理”作为唯一策略，而是区分生命周期代理与受控辅助代理。
- [x] `codex.dispatch_mode: inline` 仍为默认值，`get_context.py --mode phase` 对 Codex
      仍选择 Inline 路由；现有 Implement/Check 子代理路径未被破坏。
- [x] 受控辅助代理的触发条件、读写边界、并发限制、任务上下文加载顺序和主 Agent 复核
      责任均有明确、可执行的文字契约。
- [x] 配置、Hook、阶段解析器和 Agent 角色卡不再以互相矛盾的固定 `fork_turns` 假设
      解释默认模式。
- [x] 相关 Python 脚本测试、TOML 解析、工作流上下文输出和 `git diff --check` 通过。
- [x] 未读取、写入或删除真实用户级 Codex/Relay 数据；`bash.exe.stackdump` 未被纳入本任务。

## 范围外

- 不修改应用 Rust/Vue 业务代码、Provider 行为或发布配置。
- 不修改全局 Codex 配置、真实用户级会话、认证文件或 Relay 数据目录。
- 不在本任务中运行真实子代理试点；试点应在规则落地后另建低风险任务并记录对比证据。
- 不引入新的 Channel Worker 编排系统或并行写入机制。

## 验证约束

- 所有路径和配置检查使用仓库文件或安全临时目录。
- 需要验证 Codex 行为时，使用临时 `CODEX_HOME`/`CODEX_SQLITE_HOME` 和脱敏 fixture；
  不以默认用户配置目录作为测试前提。
- 任务结束前保留未跟踪 `bash.exe.stackdump` 的真实状态，不声称工作树完全干净。

## 说明

- `prd.md` 只记录需求、约束和验收标准；技术设计写入 `design.md`，执行顺序写入
  `implement.md`。
