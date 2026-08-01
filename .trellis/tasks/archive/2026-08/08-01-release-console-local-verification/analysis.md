# 缺陷复盘：发布控制台本地门禁误失败与诊断折叠

## 1. 根因类别

- **主要类别：E（隐式假设）**——`src/release-request.test.ts` 假设 PowerShell 中文异常在所有 Windows 子进程环境中都按 UTF-8 解码；发布控制台过滤环境改变了该假设，中文变为乱码，真实拒绝行为被测试误判。
- **次要类别：D（测试覆盖缺口）**——结构测试直接执行能够通过，但缺少通过生产 `ProcessLocalVerificationBackend`、生产环境过滤和真实 npm shim 的永久回归，无法在开发阶段发现组合环境差异。
- **次要类别：B（跨层契约）**——local verification service 已知道失败命令，但 orchestrator 将其折叠为无字段错误，application 又只收到 `error.code()`，最终界面只能显示 `releasePipeline` 与通用文案。

## 2. 为什么先前路径没有解决问题

1. **直接执行四个结构测试**：证明测试代码与候选内容本身可通过，但没有覆盖发布控制台的过滤环境，因此不能解释真实一键发布失败。
2. **检查 npm shim / Job Object**：真实 `npm.cmd --version` 经同一 process backend 正常，排除了命令 shim 和基础启动链；继续修改启动逻辑只会偏离根因。
3. **恢复 `WT_SESSION` 实验**：没有改变失败，说明问题不是单个终端环境变量，而是测试把本地化输出当机器契约。
4. **既有通用错误事件**：即使底层已经知道失败命令，跨层折叠仍隐藏了证据，使控制台只能提示“查看对应阶段证据”，增加了诊断成本。

## 3. 预防机制

| 优先级 | 机制 | 具体行动 | 状态 |
|---|---|---|---|
| P0 | 机器契约 | PowerShell 秘密检测输出 ASCII 稳定码 `RELEASE_NOTES_SECRET_DETECTED`，Vitest 不再依赖中文 | 已完成 |
| P0 | 类型/架构 | `LocalVerificationError` 与 `ReleaseOrchestratorError` 保留命令 ID 和 `Option<i32>` 退出码 | 已完成 |
| P0 | 集成测试 | 通过真实 process backend、环境过滤和 npm shim 执行生产第一条发布结构测试 | 已完成 |
| P0 | 真实性 | 无退出码时只说明“命令未能完成且没有可用退出码”，不猜测启动、超时等类别 | 已完成 |
| P1 | 规范 | `.trellis/spec/release/publishing.md` 增加完整本地门禁编码与错误证据契约 | 已完成 |
| P1 | 思考指南 | 跨层检查新增本地化子进程文本、字段传播和过滤环境回归检查项 | 已完成 |

## 4. 系统性扩展

- **相似问题**：未来修改其它 PowerShell/原生命令测试时，应搜索是否有对中文 stdout/stderr 的唯一断言；可读文本可保留，但机器契约应使用稳定 code 或结构化输出。
- **设计改进**：只在确有用户价值时扩展具体进程失败类别；当前 `Option<i32>` 能安全区分“非零退出”与“没有退出码”，不应为了显示更具体文案而回传原始 stderr。
- **流程改进**：发布控制台涉及环境过滤、shim、Job Object 或工作目录时，至少保留一个穿过生产基础设施边界的回归；直接执行和 mock backend 都不能替代该证据。
- **范围控制**：不实施全局 Windows 编码重构，也不把所有 PowerShell 错误一次性编号；只为被自动化消费的错误建立稳定契约。

## 5. 知识沉淀

- [x] 更新 `.trellis/spec/release/publishing.md`。
- [x] 更新 `.trellis/spec/guides/cross-layer-thinking-guide.md`。
- [x] 添加 service、orchestrator、application 和过滤环境回归测试。
- [x] 在任务材料保留 RED/GREEN、完整检查与打包证据。
- [x] 仓库不存在 `src/templates/markdown/spec` 或其它对应模板副本，无需同步模板。
