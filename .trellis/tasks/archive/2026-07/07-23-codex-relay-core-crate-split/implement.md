# 实施计划

## 约束与执行模式

- 当前任务使用 Codex inline 模式，主会话直接实施和检查，不派发子 Agent。
- 每个行为切片遵循红测 → 最小绿实现 → 专项回归 → 必要重构。
- 不读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。
- 不清理默认 `src-tauri/target`；独立 target 仅用于最终受控冷基准。
- 任何产品行为、安全边界或发布形态的显著变化都先回到规划并请求用户确认。

## 有序实施清单

### 1. 结构契约红测

- [x] 扩展 `scripts/tests/rust-dev-scripts.tests.ps1`：要求快速 Rust 命令显式选择
  `-p codex-relay-core`，完整 Rust 门禁覆盖 workspace，并检查 core manifest 存在且没有直接 Tauri
  依赖。
- [x] 扩展 `scripts/tests/rust-dependency-graph.tests.ps1`：增加 core 依赖树包含 Tauri 时失败、无 Tauri
  时通过的合成场景。
- [x] 运行 `npm run test:rust-dev-guard` 与 `npm run test:rust-deps`，记录因目标结构尚不存在而失败的
  红测证据。

### 2. Provider 内部模型所有权

- [x] 在 `models::provider_availability` 定义内部 `ProviderAvailabilityTarget`，保留字段、Clone 与
  Debug 脱敏语义。
- [x] 更新 ProviderService、Provider HTTP、gateway、runner 和 availability service 的引用，删除
  service 层旧定义。
- [x] 运行 `provider_http`、`codex_gateway`、`codex_runner`、`provider_service` 和
  `provider_availability_service` 专项；确认错误码、请求形状和密钥脱敏不变。

### 3. 建立 workspace 与 core crate

- [x] 在 `src-tauri/Cargo.toml` 添加 resolver 3 workspace 和 core member；保持根 Tauri package、
  build dependency、crate-type 和版本位置不变。
- [x] 创建 `src-tauri/crates/codex-relay-core/Cargo.toml` 与 `src/lib.rs`，只声明 Provider-centered
  平台无关依赖。
- [x] 使用 `apply_patch` 迁移 error/models、Provider/事务 services 和相应 infrastructure 到 core；
  safe log、file watch、自检和 autostart 经基准验证后按桌面生命周期所有权留在根 crate。
- [x] 从 `autostart_service` 移除 Tauri 具体 backend，在根 crate 新建
  `tauri_autostart_backend.rs` 实现本地 trait。
- [x] 将 `path_safety.rs`、`provider_workflow.rs` 移到 core tests，并改用 `codex_relay_core` 导入。
- [x] 更新迁移后服务测试的 fixture 相对路径；搜索所有 `include_str!`/`include_bytes!` 防止遗漏。
- [x] 根 `lib.rs` 和本地 wrapper modules re-export core 的 `error`、`models`、`services`、
  `infrastructure`，保持现有模块路径；
  只为 Tauri `run()` 暴露最小 TLS 初始化入口。
- [x] 把依赖按实际使用拆分到根/core manifest，运行 `cargo metadata` 与 `cargo check --workspace`。

### 4. 快速入口与质量门禁转绿

- [x] 将 `test:rust:lib`、`test:rust:path-safety`、`test:rust:provider-workflow` 改为
  `-p codex-relay-core`，继续显式使用固定 `src-tauri/target` 和原 watcher pre-script。
- [x] 将 `check:rust` 的 fmt、Clippy、test 改为 workspace 范围。
- [x] 扩展真实依赖图检查：全应用保持 ring-only，core 依赖树不得出现 Tauri。
- [x] 重新运行两组 PowerShell 合成测试，确认结构红测转绿。

### 5. 行为专项与跨 crate 兼容

- [x] core：`provider_http`、`codex_gateway`、`codex_preflight`、`codex_runner`、
  `provider_availability_service`。
- [x] core：`provider_service`、`transaction_service`、`config_service`、`settings_service`。
- [x] core integration：`path_safety` 与 `provider_workflow`。
- [x] Tauri 根 crate：commands、AppState、tray、autostart adapter 测试；确认 command DTO/错误码路径不变。
- [x] 运行 `cargo tree -p codex-relay-core`，确认没有 `tauri`/`tauri-plugin-*`；运行全图检查确认
  `aws-lc-sys` 不存在且 Rustls `ring` 存在。

### 6. 文档与长期规范

- [x] 更新 README 目录结构、core 快速测试和 workspace 完整门禁说明。
- [x] 更新 backend `rust-guidelines.md` / `service-boundaries.md` 的模块路径与 Tauri adapter 边界。
- [x] 更新 testing `rust-build-feedback.md` 的 core package、workspace 检查和 20 秒重建基准契约。
- [x] 核对“关于”页面：本任务不改变产品定位、数据/安全、更新或卸载行为，不做无关修改。

### 7. 完整验证与基准

- [x] 运行 `npm run test:rust-dev-guard`、`npm run test:rust-deps`、`npm run check:rust:deps`。
- [x] 运行专项 core 测试与根 Tauri 测试。
- [x] 运行完整 `npm run check`，记录测试数量、编译/链接耗时和任何首次失败。
- [x] 运行 `npm run build:frontend`。
- [x] 运行安全审计：Git 状态/跟踪文件、密钥前缀和 Authorization/Bearer 人工复核、路径数据、
  `git diff --check`。
- [x] 在固定默认 target 上连续至少 3 次触发 core 源码时间戳重建并运行 Provider HTTP 专项，验证
  每次完整反馈是否低于 20 秒。
- [x] 记录缓存命中、core 重建、根应用重建（如有必要）、core 冷构建、完整门禁和产物大小。
- [x] 独立冷 target 只在绝对路径/系统 temp/前缀三重校验后创建与删除；删除后确认不存在。

### 8. 完成流程

- [x] 根据实施中形成的稳定契约判断并完成 spec 更新，运行 Trellis check。
- [x] 审阅 `git diff`、`git diff --check` 和暂存差异；提交代码/文档，提交信息使用简体中文。
  工作提交为 `5393ceb`，规范与任务材料提交为 `eaee269`。
- [ ] 运行 Trellis 收尾流程，归档任务并记录会话。
- [ ] 确认没有 push。

## 验证命令

```powershell
npm run test:rust-dev-guard
npm run test:rust-deps
npm run check:rust:deps
npm run test:rust:lib -- provider_http
npm run test:rust:lib -- codex_gateway
npm run test:rust:lib -- codex_preflight
npm run test:rust:lib -- codex_runner
npm run test:rust:lib -- provider_availability_service
npm run test:rust:lib -- provider_service
npm run test:rust:lib -- transaction_service
npm run test:rust:path-safety
npm run test:rust:provider-workflow
cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay --lib
cargo tree --manifest-path src-tauri/Cargo.toml -p codex-relay-core -e normal,build
npm run check
npm run build:frontend
git diff --check
```

## 风险文件与回滚点

- `src-tauri/Cargo.toml` / `Cargo.lock`：workspace 默认成员或依赖归属错误可能漏测 core；完整门禁必须
  显式 `--workspace`。失败时回滚 workspace 切片，不修改上一阶段 TLS feature。
- `src-tauri/src/lib.rs`：re-export 或 Tauri adapter 路径错误会破坏 command 构建；根 lib 专项先于
  全量检查。
- `src-tauri/crates/codex-relay-core/src/services/transaction_service.rs`：只移动和改相对 fixture 路径，
  不重写事务逻辑；任何行为失败立即对照移动前字节和测试。
- Provider HTTP/gateway/process：类型路径迁移不得改变代理、TLS、超时、重试、SSE 或错误分类。
- `scripts/check-rust-dependency-graph.ps1`：新增 core 边界检查不能削弱 ring/aws-lc 现有检查。
- 冷基准清理：任何路径校验失败立即停止，不执行递归删除。

## 启动审查清单

- [x] PRD 已收敛，需求和验收标准可测试，没有阻塞规划的开放问题。
- [x] design 明确 crate 所有权、依赖方向、兼容、测试、基准和回滚。
- [x] implement 为每个切片列出红绿证据和完整验证命令。
- [x] 用户已明确授权完成规划和启动审查后直接实施。
- [x] Codex inline 模式，不派发子 Agent 或建立重复工作流。
- [x] 实施前加载 `trellis-before-dev` 和 Phase 2.1 细则。

## 当前进度

- 结构红测、workspace/core 迁移、边界收窄、专项行为验证和文档/规范更新已完成。
- 初始全平台无关 core 的强制重建为 Cargo 24.19 秒、完整入口 29.85 秒，未达标；将桌面生命周期
  模块移回根 crate，并把错误脱敏纯函数留在 core 后，最终连续三次完整 Provider 快速入口为
  10.49、11.38、11.30 秒，对应 Cargo 6.15、7.68、7.06 秒。
- core 单元 134/134、path safety 3/3、provider workflow 1/1、根 Tauri 单元 39/39 已通过。
- 最终 `npm run check`、前端构建、安全审计和受控冷构建基准已完成；工作与规范已提交，下一步
  按 Trellis 收尾流程归档任务并记录会话。

## 验证证据

- 结构红测：`test:rust-dev-guard` 因快速命令尚未包含 `-p codex-relay-core` 退出 1；
  `test:rust-deps` 因生产脚本尚不支持 core 依赖图输入而期望 4、实际 1。两项均因目标结构缺失
  的预期原因失败。
- 结构绿测：`rust-dev-scripts: 12 tests passed`、`rust-dependency-graph: 5 tests passed`；真实依赖图
  确认 ring provider 存在、`aws-lc-sys` 不存在、core 不依赖 Tauri。
- 首次 workspace `cargo check --workspace --all-targets` 暴露 Tauri `generate_context!` 要求根 package
  直接依赖 `serde_json`；恢复普通依赖后同命令退出 0。该失败未被后续成功隐去。
- 类型/行为专项：Provider availability 11、Provider HTTP 6、gateway 3、preflight 6、runner 8、
  Provider service 18、transaction 9 均通过；最终 core 全量 134/134、path safety 3/3、provider
  workflow 1/1、根 Tauri 39/39。
- 初始过宽 core 的强制源码重建：Cargo 24.19 秒、完整 npm 入口 29.85 秒，未达 20 秒。
- 最终 Provider 快速反馈：连续三次更新时间戳后运行完整 npm 入口为 10.49、11.38、11.30 秒；
  Cargo 编译/链接为 6.15、7.68、7.06 秒。缓存命中 `--no-run` 为 0.95 秒。
- 最终 core 单元测试产物：exe 13,542,400 bytes，PDB 106,958,848 bytes；拆分前根测试产物约
  18,691,072 bytes / 186,404,864 bytes。
- 最终受控 core 冷构建：114.07 秒、1502 个文件、928,222,098 bytes，退出 0；相对上一阶段单体
  422.67 秒、2.68 GiB 显著下降。临时 target 经系统 temp、前缀和非根目录校验后删除，残留为 0。
- 最终 `npm run check` 退出 0，总耗时 266.81 秒：Trellis 8/8、前端 139/139、core 134/134、
  根 Tauri 39/39、path safety 3/3、provider workflow 1/1，Clippy/Doc tests 通过；相对 352 秒基准
  减少 85.19 秒（约 24.2%）。
- `npm run build:frontend` 退出 0，1717 modules，Vite 构建 18.34 秒、完整入口 21.38 秒；保留了
  Rollup 对第三方 `#__PURE__` 注释位置的两条非失败 warning。
- 最终安全审计：高置信度 `sk-` 文件 0；Git 仅跟踪 `dev-data/.gitkeep`；core Tauri 命中 0；
  公开原始 internal detail 入口 0；冷 target 残留 0；`git diff --check` 退出 0。
