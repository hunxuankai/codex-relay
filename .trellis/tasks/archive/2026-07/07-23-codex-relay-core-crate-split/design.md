# 技术设计

## 总体方案

把 `src-tauri` 从单 package 调整为“同根 workspace + 两个 package”：

```text
codex-relay（Tauri 应用）
  ├─ Tauri run/build/plugin 组合
  ├─ commands / AppState / tray
  ├─ autostart / file watch / self-check / safe log
  ├─ TauriAutostartBackend
  └─ 依赖 codex-relay-core

codex-relay-core（平台无关 Rust）
  ├─ error / models
  ├─ Provider、设置、事务、备份等 services
  ├─ HTTP/gateway/process/path/file 等 infrastructure
  └─ Provider、事务、路径与基础设施测试
```

workspace 根仍是 `src-tauri/Cargo.toml`，继续拥有 Tauri package、`build.rs`、`tauri.conf.json`、
`Cargo.lock` 和 `target/`。core 位于 `src-tauri/crates/codex-relay-core`，不引入新的锁文件或 target。

## Cargo workspace 与依赖边界

根 manifest 增加：

```toml
[workspace]
members = ["crates/codex-relay-core"]
resolver = "3"
```

根 package 保留：

- `tauri-build` build dependency；
- Tauri、autostart、notification、single-instance、updater plugin；
- 根 crate 自身直接使用的 `serde`、`serde_json`、`tokio`、`chrono`、`notify`、
  `tracing`、`tracing-appender`、`tracing-subscriber`、`uuid`；
- 根测试直接使用的 `serde_json`、`tempfile`、`tokio` dev dependency；
- 对 `codex-relay-core` 的本地 path dependency。

core 继承 Provider/配置边界需要的依赖：`chrono`、`reqwest`、`rustls`、`serde`、`serde_json`、
`regex`、`sha2`、`tempfile`、`thiserror`、`tokio`、`toml_edit`、`url`、`uuid` 和 Windows
`windows-sys`。core manifest 不得出现 `tauri`、`tauri-plugin-*` 或只服务于桌面生命周期的
notify/logging 依赖。

`reqwest`/Rustls feature 组合原样迁移，继续使用 `rustls-no-provider` + 显式 `ring`，不允许 feature
重构顺带改变 TLS、系统代理、HTTP/2、charset 或 JSON 能力。

## 模块所有权

### core crate

- `error.rs`：`AppError`、`CommandError`、`CommandResult<T>`。
- `models/`：全部稳定 DTO、事务/健康/设置/Provider 数据。
- `services/`：Provider、配置、密钥、偏好、设置、备份、事务和 availability 服务。
- `infrastructure/`：路径、原子文件、指纹、Provider HTTP/gateway、Codex process/runner/
  preflight/JSONL、Rustls provider，以及只能在 core 内读取错误详情的脱敏/日志格式化纯函数。
- `tests/`：`path_safety` 与 `provider_workflow`，改为直接导入 `codex_relay_core`。

### Tauri 应用 crate

- `lib.rs` / `main.rs`：Tauri builder、plugin、invoke handler 和进程生命周期。
- `commands/`：保持现有 command 签名与结果映射。
- `app_state.rs`：组合 core 服务和 Tauri runtime 状态。
- `tray.rs`：Tauri tray/window/notification 适配。
- `services/`：autostart、file watch、自检等桌面生命周期服务；它们复用 core 的模型和服务，
  但不进入 Provider 快速测试目标。
- `infrastructure/safe_log.rs`：桌面日志初始化与文件保留；脱敏/错误格式化复用 core 实现。
- `tauri_autostart_backend.rs`：实现根 crate 的 `AutostartBackend`，封装
  `tauri_plugin_autostart::ManagerExt`。

根 `lib.rs` 通过：

根 crate 直接 re-export core 的 `error`/`models`，并在本地 `services`/`infrastructure` 模块中
re-export core 子模块，同时挂载桌面生命周期模块。

保持 `crate::error`、`crate::models`、`crate::services`、`crate::infrastructure` 以及外部
`codex_relay_lib::<module>` 路径不变。应用层不复制 core 实现。

## Provider 数据流与层边界

迁移后的数据流保持：

```text
Tauri command
  → AppState 中的 core Service
    → Provider/Settings/Transaction Service
      → core Infrastructure
        → 临时目录、受管配置或显式 Provider 网络边界
  → CommandResult DTO
  → 前端
```

现有 `ProviderAvailabilityTarget` 从 `provider_service` 移到
`models::provider_availability`，由 `ProviderService` 构造，Provider HTTP、gateway 和 runner 只消费
该内部模型。这样 infrastructure 不再反向引用 service 模块。字段和 `Debug` 脱敏行为不变，类型
保持 `pub(crate)`，不成为 Tauri command DTO。

## TLS 初始化契约

core 保留幂等 `ensure_ring_crypto_provider` 内部实现，并暴露一个最小的公开初始化入口供 Tauri
`run()` 在 plugin 构造前调用。Provider HTTP 与 gateway 仍在各自 Client 构造前调用同一内部边界，
避免依赖应用启动顺序。三处调用保持上一阶段的防回归语义。

## 测试与开发入口

### 快速行为切片

保留现有 npm 命令名以减少开发者迁移成本，但改为显式选择 core package：

```powershell
npm run test:rust:lib -- provider_http
npm run test:rust:path-safety
npm run test:rust:provider-workflow
```

底层 Cargo 命令均包含：

```text
--manifest-path src-tauri/Cargo.toml
--target-dir src-tauri/target
-p codex-relay-core
```

watcher 门禁和固定 target 契约不变。根 Tauri 测试不混入 Provider 红绿循环。

### 完整门禁

`check:rust` 改为 workspace 范围：

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

这样完整门禁同时覆盖 core unit/integration tests 和根 Tauri 组合测试。不能依赖“根 crate 依赖
core”间接证明 core 的 `#[cfg(test)]` 测试已执行。

### 结构与依赖回归

扩展现有 PowerShell 合成测试和依赖图门禁：

- 快速脚本必须包含 `-p codex-relay-core`；
- 完整 Rust 门禁必须包含 `--workspace`/`--all`；
- workspace 必须声明 core member；
- core manifest 和真实 `cargo tree -p codex-relay-core` 不得出现 Tauri 包；
- 全应用依赖图继续拒绝 `aws-lc-sys`，并要求 Rustls `ring`。

## TDD 迁移策略

1. 先扩展脚本/依赖图合成测试，令其因 core 不存在、快速入口仍指向根 crate 而失败。
2. 在单 crate 内先把 `ProviderAvailabilityTarget` 移到模型所有者，运行现有 Provider HTTP/gateway/
   availability tests 证明行为不变。
3. 创建 workspace/core manifest 与入口，迁移 Provider-centered 模块、抽出 Tauri autostart
   adapter、更新 fixture 相对路径和根 re-export。
4. 调整 Cargo 依赖归属、快速入口和 workspace 完整门禁，使结构红测转绿。
5. 运行 Provider、事务、路径和 Tauri command 专项；保持绿色后再做格式和文档重构。

该顺序把“层级模型所有权”和“物理 crate 迁移”分开，故障时可定位到类型边界或 Cargo/文件布局，
不通过一次大范围编译失败掩盖行为回归。

实施中先验证了“全部平台无关模块进入 core”的较宽边界；强制重建仍为 24.19 秒。随后依据新鲜
基准把日志生命周期、file watch、自检和 autostart 留在根 crate；错误脱敏纯函数仍在 core 内部，
避免为日志公开原始错误详情。最终连续三次完整 Provider 快速入口为 10.49、11.38、11.30 秒。
该修订不改变公开行为，只减少 core 的无关源码、测试和依赖闭包。

## 安全与行为不变量

- 所有文件测试继续使用 `tempfile`、`AppPaths::for_test` 或成对 Relay 覆盖；不增加任何读取生产
  默认路径的代码。
- fixture 路径只因源码目录深度变化而调整，fixture 内容和假密钥保持不变。
- `TransactionService` 的共享锁、指纹、备份、临时文件、解析、原子替换、写后验证和回滚逻辑
  原文件迁移，不做语义重写。
- `config.toml` 仍由 `toml_edit` 局部修改；Provider ID、DTO 字段、camelCase、错误码、公开消息、
  Debug 脱敏和日志脱敏不变。
- Provider API/Codex gateway 的代理、超时、重定向、SSE、工具阻断、错误分类和不重试契约不变。
- Tauri command 宏、名称、参数和返回值不变；根 crate re-export 避免前端或 Rust 集成调用路径漂移。

## 基准设计

### 稳定反馈门禁

在默认 `src-tauri/target` 上：

1. 确认无普通 Tauri watcher。
2. 更新时间戳但不修改内容：`provider_http.rs` 或 `provider_availability_service.rs`。
3. 连续至少 3 次运行 `npm run test:rust:lib -- provider_http`，分别记录整条入口墙钟、Cargo 编译/
   链接时间和测试时间。
4. 每次都必须低于 20 秒才称“稳定低于 20 秒”；单次缓存命中不能替代。

同时记录 core 测试 exe/PDB 大小，与拆分前根测试 exe 约 17.8 MiB、PDB 约 177.8 MiB 比较。

### 冷构建

在 `%TEMP%` 下创建名称固定前缀 + GUID 的独立目录：

- 先解析绝对路径并确认位于系统 temp、名称前缀匹配且不是 temp 根；否则停止。
- 运行 core `cargo test -p codex-relay-core --lib --no-run`，记录耗时、文件数和目录大小。
- 如风险/时间允许，再用另一个隔离 target 记录完整 workspace/Tauri 冷构建，明确它不是日常反馈目标。
- finally 中再次做相同路径校验后删除；删除后验证目录不存在。

默认 `src-tauri/target` 绝不清理。

## 兼容、发布与回滚

- Tauri manifest、config、主 binary 名、crate-type、build script、NSIS/updater 配置和发布目录不变。
- Cargo.lock 继续只有 `src-tauri/Cargo.lock`；新增内部 package 条目属于预期变化。
- 若 core 专项未达 20 秒，保留正确拆分和真实基准，但不得通过移除测试、关闭 debug 或绕过 watcher
  门禁伪造结果；在任务材料记录剩余热点和下一步。
- 回滚点：
  1. `ProviderAvailabilityTarget` 模型移动可独立回滚。
  2. workspace/文件迁移可整体回滚到单 package，安全入口与 TLS 收敛提交不受影响。
  3. 脚本和规范只在 core 结构专项通过后更新；若结构回滚，一并恢复为根 crate 命令。
- 不 push；按 Trellis 完整检查、规范更新、提交、归档和会话记录结束。
