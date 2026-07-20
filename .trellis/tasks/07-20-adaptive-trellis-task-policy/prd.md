# 按规模与复杂度决定 Trellis 任务

## 目标

让 AI 根据请求的规模、复杂度、风险和持久化需要，自主判断是否创建 Trellis
任务；不再为了每个请求逐次征求任务创建许可，同时保留复杂和高风险工作的
Trellis 生命周期门禁。

## 已确认决定

- 用户明确授权 AI 自主决定是否创建 Trellis 任务，并要求把该规则长期记录下来。
- 本次改动只调整“是否创建任务”的判断与告知方式，不放宽任务创建后的规划、
  TDD、检查、验证、规范更新和收尾要求。
- 当前活动的中文化审计任务与本请求范围无关，必须保持原状；本规则变更使用独立
  轻量任务。

## 需求

1. 一次性答复、简单只读查询，以及范围清楚、低风险、局部且可在当前会话内完成和
   验证的小改动，可以不创建 Trellis 任务。
2. 出现以下任一情形时应创建任务：跨模块或多阶段工作；需求或方案复杂；涉及
   安全、配置迁移、发布、卸载、数据保留等高风险边界；需要持久化 PRD、设计、
   实施进度或验证证据；可能跨会话继续；用户明确要求使用 Trellis。
3. AI 判断需要任务时可以直接创建并简短告知用户，不再逐次询问是否允许创建。
4. 已有活动任务与请求范围一致时继续使用；范围不一致时不得把新工作混入旧任务，
   应按同一判断标准新建任务或直接处理。
5. `AGENTS.md`、`.trellis/workflow.md`、工作流规范及直接参与任务分类的本地 Skill
   必须表达一致规则；任务创建后的既有生命周期门禁保持不变。
6. 文档与纯工作流变更使用结构验证和失败预演，不虚构产品单元测试。

## 行为切片

- 公开入口：没有活动任务时的项目指令与 `[workflow-state:no_task]` 路标。
- 输入：一个新的用户请求及其规模、复杂度、风险、持续时间和证据需求。
- 预期结果：AI 自主选择“直接处理”或“创建 Trellis 任务”；选择创建时告知用户，
  但不请求任务创建许可。
- 验证边界：读取项目内 Markdown 并断言分类标准一致；不访问真实用户 Codex 或
  Codex Relay 数据目录，不 mock 产品运行时。

## 验收标准

- [x] `AGENTS.md` 明确授予 AI 任务创建判断权并列出必须建任务的风险边界。
- [x] `.trellis/workflow.md` 的请求分类、`no_task` 路标、Phase 1 目标和创建步骤不再
  要求逐次征得任务创建同意。
- [x] `.trellis/spec/workflow/` 与任务分类 Skill 不再保留冲突的“非平凡必建”或
  “用户先同意创建”表述。
- [x] 旧规则失败预演在实施前因仍存在许可要求而失败，实施后的同一结构检查通过。
- [x] 工作流状态块仍可解析，任务校验、`git diff --check` 和与风险相称的项目检查
  有本轮真实证据。
- [x] 不修改产品运行时代码，不读取或写入真实用户配置与密钥。

## 验证证据

- 初始任务创建策略探针退出 1，报告 5 个新规则缺失和 6 个旧许可规则命中；完成
  修改后用同一探针重跑退出 0。
- 活动任务隔离补充探针首次退出 1，确认 `planning`、`planning-inline`、
  `in_progress`、`in_progress-inline` 四个路标都缺少无关请求隔离规则；修改后
  重跑报告 `4/4` 通过。
- `python ./.trellis/scripts/get_context.py --mode phase --step 1.0` 退出 0，并输出
  “直接创建任务并简短告知用户，不另行征求任务创建许可”的新步骤正文。
- workflow-state 标签检查退出 0：7 组开始/结束标签全部配对，其中包含文档示例
  `my-status`。
- `quick_validate.py` 首次按 Windows 默认 GBK 读取中文 Skill 时抛出
  `UnicodeDecodeError`；确认根因为验证器未指定编码后，使用 `python -X utf8`
  分别校验 `trellis-start` 和 `trellis-brainstorm`，均返回 `Skill is valid!`。
- `python ./.trellis/scripts/task.py validate 07-20-adaptive-trellis-task-policy`
  退出 0；inline 轻量任务缺少 `implement.jsonl` / `check.jsonl` 按设计跳过。
- 最终 `npm run check` 退出 0：Vue TypeScript 检查通过，Vitest 15 个测试文件、
  65 项测试通过；Rust fmt、Clippy 通过，Rust 共 110 项测试通过。
- `git diff --check` 退出 0；仅出现 Git 关于未来按工作区设置转换 LF/CRLF 的提示。
- `git status --short` 显示产品运行时代码没有改动；另一个未跟踪的
  `07-20-audit-chinese-docs-and-commits` 任务目录保持在本任务范围外。

## 范围外事项

- 不改变 Trellis 任务创建后的 PRD、TDD、检查、提交和归档生命周期。
- 不增加第二套规划、TDD、子 Agent 派发或分支收尾流程。
- 不修改 Trellis CLI 上游源码、全局 npm 安装或 `node_modules`。
