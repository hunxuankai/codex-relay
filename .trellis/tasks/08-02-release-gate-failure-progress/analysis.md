# 缺陷复盘：成功发布门禁被进程收尾误判且失败进度丢失

## 1. 根因类别

- **主要类别：E（隐式假设）**——`SafeProcessRunner` 假设主进程退出后超过 5 秒仍有 Job 后代，
  就等价于“进程树未能安全终止”。代码虽然调用 `job.terminate()`，却不验证结果并无条件返回
  `ProcessTreeTermination`，从而抹掉已经取得的成功退出码。
- **次要类别：B（跨层契约）**——`ProcessLocalVerificationBackend` 除取消外把全部 `ProcessError`
  折叠为一个 `Failed`，service/orchestrator 只保留 `Option<i32>`，控制台无法区分超时、输出上限、
  进程树终止或输出读取失败。
- **次要类别：D（测试覆盖缺口）**——已有测试覆盖取消时终止后代和第一条发布结构测试，但没有覆盖
  “父进程成功退出、后代可安全清理”以及生产 backend 执行完整 `npm run check` 的组合边界。
- **次要类别：B（状态契约）**——session 只持久化终态 `failed`，时间线又把 failed 直接投影为所有步骤
  waiting；实时 `StepFailed` 没有持久化，重启后无法恢复现场。

## 2. 为什么先前修复没有覆盖

1. 上一次本地门禁诊断修复解决了 PowerShell 中文编码和命令 ID/可选退出码传播，但为了不虚构底层
   原因，把所有进程错误统一描述为“没有可用退出码”；这保留真实性，却没有建立类型化分类。
2. 生产过滤环境回归只运行第一条发布结构测试，未经过输出更长、进程层级更多的完整 `npm run check`。
3. 上一次“固定发布进度”改动处理的是左右滚动布局；`ReleaseTimeline` 的终态 reducer 没有进入该范围，
   因而 `failed → waiting` 的旧逻辑仍然存在。
4. 直接运行 `npm run check` 能证明脚本成功，但不能证明同一命令穿过 Job Object 后仍被判定成功。

## 3. 预防机制

| 优先级 | 机制 | 具体行动 | 状态 |
|---|---|---|---|
| P0 | 架构 | 安全终止并验证剩余后代成功后保留主进程退出码 | 已完成 |
| P0 | 类型 | 用 `LocalVerificationFailure` 穷尽区分退出码与进程分类 | 已完成 |
| P0 | 状态 | `ReleaseStateStore::fail` 原子保存失败前阶段、stepId 和 code | 已完成 |
| P0 | 测试 | 增加真实父进程成功退出/后代清理测试和完整 backend 慢速探针 | 已完成 |
| P1 | 前端 | 时间线优先消费持久化 failure，加载 session 时失效旧事件通道 | 已完成 |
| P1 | 规范 | 更新发布代码规范和跨层思考检查项 | 已完成 |

## 4. 系统性扩展

- **相似问题**：Git、GitHub CLI 与其它 `SafeProcessRunner` 消费者同样受益于“安全清理成功不等于
  命令失败”的修复；它们的严格超时、取消和真正终止失败仍保持不变。
- **设计改进**：终态枚举只能说明结果，不能承担失败现场。需要跨重启展示时，应持久化紧凑、类型化、
  无秘密的检查点，而不是复制完整事件日志或从 `failed` 猜测。
- **流程改进**：长命令既要有直接执行证据，也要有穿过生产 runner/过滤环境的边界证据；两者不能互相替代。
- **诊断安全**：Windows 进程树诊断应依赖 Job Object、父进程返回的子 PID 和有界测试夹具；不得用
  退出后可能 PID 复用的全系统父子轮询去终止进程。

## 5. 知识沉淀

- [x] 更新 `.trellis/spec/release/publishing.md`。
- [x] 更新 `.trellis/spec/guides/cross-layer-thinking-guide.md`。
- [x] 增加 core、service、orchestrator、state、application、Vue 与 composable 回归测试。
- [x] 保留默认 ignored 的生产完整门禁探针，避免默认套件递归。
- [x] 仓库不存在需同步的 `src/templates/markdown/spec` 对应模板，本次无需模板同步。
