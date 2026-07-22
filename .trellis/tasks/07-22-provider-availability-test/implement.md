# 实施计划

## 顺序与 TDD 行为切片

1. **结果 DTO、错误矩阵与目标解析** `[completed 2026-07-23]`
   - 先写 Rust 单元测试：camelCase DTO、稳定 status/code、Debug/序列化不含密钥、合法/无效
     Provider、缺少密钥和缺少偏好模型。
   - 扩展 `ProviderService` 的只读内部目标解析，不新增前端密钥接口，不写任何受管文件。
   - 公开被测边界：`ProviderAvailabilityResult`、`ProviderService::resolve_availability_target`。

2. **API Responses 探测** `[completed 2026-07-23]`
   - 添加 `reqwest` 直接依赖，先用回环 HTTP server 写失败测试：正确 `/responses` URL、无 tools、
     Bearer 假密钥、选中模型、16 token、无重定向、无重试、显式 Relay proxy。
   - 覆盖 2xx、401、403、404、429、5xx、非 JSON、非 Responses、256 KiB 超限、挂起、取消、
     DNS/连接/TLS 分类。
   - Mock 只替代外部 Provider；请求构造、Header、正文、响应读取和分类使用真实实现。

3. **活动测试与取消协议** `[completed 2026-07-23]`
   - 先写单活动测试、重复请求拒绝、错误 request ID、取消正确请求、完成后注册项清理测试。
   - 使用 `tokio::sync` 取消信号和 RAII 清理；锁只保护注册表，不跨网络或子进程等待持有。
   - API 取消先接入同一协议，形成后续 Codex runner 的公共取消边界。

4. **Codex 参数、环境与 catalog 纯构造** `[completed 2026-07-23]`
   - 先写纯函数测试：精确版本允许列表、完整严格配置、argv 不含密钥、唯一 env key、环境白名单、
     catalog 纯文本字段、临时根必须位于系统 temp。
   - 合成 CLI 帮助/版本/JSONL fixture，只使用 `test-key-*-not-real`。
   - 将当前 `0.144.4` 参数和 catalog 契约集中为单一版本适配器；不得在多个分支复制常量。

5. **回环安全预检** `[completed 2026-07-23]`
   - 先写回环 server 测试：捕获首个 Codex Responses 请求，精确核对工具集合、Provider、模型、
     假 Authorization 和请求上限，并返回最小流式完成响应。
   - 覆盖未知/多余/缺失工具、非目标 Provider、真实密钥意外出现、请求超限和预检超时；这些
     情况都证明真实 Provider client 未被调用。
   - 不使用 `codex doctor`，不读取真实 Codex Home。

6. **Windows 进程树与有界输出** `[completed 2026-07-23]`
   - 先写 Windows 专项测试：Job Object 创建/加入失败、父进程派生子进程、关闭 Job 后整树退出、
     stdout/stderr 1 MiB 上限、超时和取消、清理前路径校验、目录锁导致清理失败。
   - 使用可注入假 executable/进程后端；不得运行用户安装的 Codex。
   - 只在 Job Object 门禁通过后允许进入真实 Provider 阶段；`kill_on_drop` 和 `taskkill` 不替代
     Job 契约。

7. **Codex JSONL 解析与兼容性服务** `[completed 2026-07-23]`
   - 先写中央 `unknown → typed event` 解码测试：正常完成、远端错误、任意工具调用、权限请求、
     Hook/MCP/plugin/web search 标记、未知事件、非 JSON、截断、退出码异常和 stderr 安全警告。
   - 基于 0.144.4 官方源码确认 JSONL 未覆盖全部内部工具类型；增加受监控回环转发层，Codex
     只持有假密钥，Rust 向真实 Provider 注入真实 Header，并在工具调用 SSE 到达 Codex 前阻断。
   - 组合版本检查 → managed requirements 检查 → 临时目录 → 回环预检 → 受监控真实运行 → 进程树
     退出 → 清理；每个失败点验证是否联系真实 Provider、公开 status/code 和密钥不泄漏。
   - 成功路径必须没有任何工具事件且临时根不存在。

8. **AppState、Tauri commands 与命令级测试** `[completed 2026-07-23]`
   - 在 `AppState` 注入 `ProviderAvailabilityService`，新增独立 command module 和 invoke handler。
   - 先写 command 测试：camelCase 参数、UUID 校验、API/Codex 结果、取消、统一 CommandResult、
     JSON 不含密钥/路径/argv/响应正文。
   - 只读测试不得调用 `begin_application_write`、事务、托盘刷新、通知或 providers-changed 事件。

9. **TypeScript 类型与 typed service** `[completed 2026-07-23]`
   - 先写 `src/services/tauri.test.ts` 失败测试，确认三个 command 名、参数名、结果解包和安全错误。
   - 新建独立 availability 类型文件；只有 `src/services/tauri.ts` 导入 `invoke`。

10. **可用性 composable** `[completed 2026-07-23]`
    - 先写行为测试：API/Codex 结果独立、单测试 busy、重复操作阻断、取消、晚响应丢弃、指纹变化
      清空结果、错误只保留稳定 code/message。
    - 实现 `useProviderAvailability`，返回 readonly 状态与显式动作；不把结果并入 Provider DTO 或
      localStorage。

11. **Provider 可用性面板与详情编排** `[completed 2026-07-23]`
    - 实施前按当前 Element Plus 2.14.3 官方文档/类型核对 `ElButton`、`ElTag` 和现有
      `ConfirmDialog` 契约。
    - 先写组件/视图测试：默认 API 主操作、高级独立入口、费用说明、确认后才启动 Codex、取消、
      两类结果、禁用原因、aria-label、Provider 变更失效和窄窗口布局类。
    - 新建聚焦 `ProviderAvailabilityPanel.vue`；`ProvidersView` 只组合两个 composable、确认对话框
      和统一交互禁用状态，不内联结果解析。

12. **产品契约、关于页与安全说明** `[in_progress]`
    - 更新 `.trellis/spec/project/product-contract.md`、README、AboutView 及测试，说明只有用户显式
      测试才访问模型网络、API 最小请求与 Codex 正常回合的 token 差异、结果不持久化。
    - 核对设置页代理说明：Relay proxy 适用于更新与 API 测试，不改变普通 Codex CLI；高级测试
      使用受限继承的 CLI 网络环境。

13. **隔离回归、复盘与规范更新**
    - 扩展 `path_safety`：默认 `.codex`/CodexRelay 哨兵前后递归快照不变，测试只使用
      `AppPaths::for_test`、回环网络、假 executable 和假密钥。
    - 运行高置信度密钥扫描，复核 `OPENAI_API_KEY`、Authorization、Bearer、`env_key` 命中。
    - 若出现重复修复或安全门禁缺陷，使用 `trellis-break-loop`；完成检查后用
      `trellis-update-spec` 沉淀 Provider 测试、外部进程和有界网络响应契约。

## 风险文件与回滚点

- 预计新增/修改：`models/provider_availability.rs`、`services/provider_availability_service.rs`、
  Codex runner/Windows Job infrastructure、provider command、`AppState`、`lib.rs`、Cargo 依赖、
  availability TypeScript 类型、typed service、composable、面板、ProvidersView、AboutView、README
  及对应测试。
- API 探测依赖和 URL/响应解析可独立交付；Codex 高级入口只有全部安全切片绿色后才接入 UI。
  Job Object、managed config 检查、工具预检、输出上限或清理任一失败时，回滚高级入口但保留
  已通过验证的 API 测试，不实施降级方案。
- 新功能不写受管文件，不应修改 `TransactionService`、备份格式、文件监控清单或 Provider
  指纹结构；若实施中需要这些变化，返回规划重新审查。
- 添加 `reqwest`、runtime `tempfile`、Windows API feature 后先跑 Cargo check，避免到最后才发现
  MSRV、feature 或 linker 问题。
- 现有 `useProviders` 不承担新测试状态，避免把 CRUD refresh/busy 与长网络请求耦合；统一禁用只
  在视图组合层派生。
- 真实 CLI 自动化验证受仓库路径红线限制；不得为了“端到端”声明运行位于真实 `.codex` 下的
  安装包或读取真实认证。缺少安全外置 CLI 时保留未验证项。

## `task.py start` 前检查

- [ ] 用户审查并批准 `prd.md`、`design.md`、`implement.md`。
- [ ] PRD 没有开放问题，API 默认 + Codex 高级入口、手动取消和精确版本门禁均已确认。
- [ ] `codex.dispatch_mode` 仍为 inline，不创建 implement/check JSONL 或子 Agent 流程。
- [ ] 实施前运行 `trellis-before-dev`，按索引重新加载 project/backend/frontend/security/testing
  详细规范和 Vue 必读参考。
- [ ] 初始红测只使用回环网络、假 executable、临时目录和 `test-key-*-not-real`。

## 当前进度

第 13 个隔离回归、复盘与规范更新切片已完成，进入提交前最终审查。

## 已完成

- 新增 `ProviderAvailabilityPanel.vue`，以 props/emits 展示 API 与 Codex 两类独立操作、结果、状态、耗时、取消态和禁用原因。
- `ProvidersView.vue` 接入 `useProviderAvailability`，API 测试直接启动，Codex 测试经过中性 `ConfirmDialog`；Provider CRUD/偏好操作与测试共享 busy 状态，选择入口仍可用。
- Provider 指纹变化会调用 `invalidateAll()`；测试结果不进入 Provider DTO 或持久化存储。
- 核对 Element Plus 2.14.3 官方 Button/Tag 文档及本地类型，确认 `plain`、`loading`、`disabled`、`native-type`、`type`、`effect` 和 `size` 契约。
- 产品契约、README、AboutView 及 AboutView 测试已同步两类测试的联网触发、token/延迟差异、会话内结果和不修改 `config.toml`/`auth.json` 边界。
- CLI 缺失错误已按 TDD 增加回归：先观察到 `CODEX_CLI_MISSING` 断言失败，再将 `ExecutableUnavailable` 与 `UnsupportedVersion` 分开映射；CLI 缺失不再伪装成版本漂移。
- Windows Job Object 进程计数竞态已修复；取消/超时会查询 Job PID 并等待已观测句柄退出，path-safety 新增 API 回环哨兵测试，`provider_workflow` fixture 补齐模型偏好。
- 新增 backend `provider-availability-testing.md`，并在错误日志、路径密钥安全和前端状态规范中记录七章节的跨层契约、错误矩阵、测试断言点和错误/正确示例。
- 追加只读边界回归：Provider 目标解析不创建缺失的 `providers.json`，API 测试不创建缺失的 `settings.json`；测试运行不会因读取代理或密钥而改写应用数据。

## 关键决策

- 高级确认固定说明本机 Codex、一次正常回合、较高 token 消耗及不修改 `config.toml`/`auth.json`。
- 面板不调用 Tauri；所有动作经视图转发到 availability composable。

## 验证证据

- `npx vitest run src/components/ProviderAvailabilityPanel.test.ts`：4 tests passed。
- `npx vitest run src/views/ProvidersView.test.ts`：10 tests passed。
- `npx vitest run src/components/ProviderAvailabilityPanel.test.ts src/views/ProvidersView.test.ts src/composables/useProviderAvailability.test.ts src/services/tauri.test.ts`：26 tests passed。
- `npm run typecheck`：通过。
- `npm run check`（2026-07-23）：Trellis 8 tests、前端 30 files/139 tests、Rust 170 unit + 3 path-safety + 1 provider-workflow tests 全部通过；Clippy `-D warnings` 和 fmt 通过。
- 追加回归专项：CLI 缺失分类、只读 `providers.json`/`settings.json`、Responses URL 尾斜杠规范化均完成红—绿验证。
- `npm run test:trellis`：8 tests passed；高置信度密钥扫描未发现真实 key 前缀，唯一 Bearer 命中为明确的 `test-key-*-not-real` fixture。

## 下一步

- 运行最终 `npm run build:frontend`、`git diff --check`、路径/跟踪文件审计并提交改动。
- 提交后进入 Trellis 3.5 收尾；不执行真实用户目录下的 Codex 人工端到端验证。

## 尚未解决的问题

- 尚未提交本轮改动；尚未执行安全外置 Codex CLI 的人工端到端验证。

## 验证命令

每个行为切片先运行专项失败测试，再实现、重构并保持绿色。完成前至少运行：

```powershell
npm run typecheck
npm run test
npm run check:frontend
npm run check:rust
npm run check
npm run build:frontend
git diff --check
git status --short --ignored
git ls-files
```

Rust 专项至少覆盖新 service/runner、command、`provider_workflow` 和 `path_safety`；前端专项至少
覆盖 typed service、composable、面板和 ProvidersView。若没有使用安全外置 Codex CLI 完成人工
端到端测试，最终报告不得声称真实 Codex 运行已验证。
