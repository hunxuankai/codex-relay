# 当前 Rust crate 边界研究

## 研究问题

确定怎样拆分 `codex-relay-core`，才能让典型 Provider 行为切片不再编译或链接 Tauri 应用 crate，
同时保持现有 command、DTO、错误码、网络、安全和事务行为不变。

## 输入证据

- 上一阶段归档材料：
  - `.trellis/tasks/archive/2026-07/07-23-rust-build-feedback-optimization/prd.md`
  - `.trellis/tasks/archive/2026-07/07-23-rust-build-feedback-optimization/design.md`
  - `.trellis/tasks/archive/2026-07/07-23-rust-build-feedback-optimization/implement.md`
- 工作提交 `15b39a3`：增加安全 no-watch/固定 target 入口、收敛 Rustls `ring`、修复 Provider
  preflight 稳定性。关键依赖和共享 TLS 安装边界位于 `src-tauri/Cargo.toml`、
  `src-tauri/src/infrastructure/rustls_provider.rs`、`provider_http.rs` 和 `codex_gateway.rs`。
- 规范提交 `635c6ca`：固定日常 Rust 构建反馈和 Provider preflight 传输/协议契约。
- 当前相关规范：backend 的 Rust/服务/错误/Provider 测试规范，testing 的 TDD/构建反馈/验证规范，
  security 的路径密钥/事务/保留规范，以及 workflow 的 Trellis 生命周期与文档归属规范。

## 当前结构事实

### Tauri 耦合面

全仓 `rg` 显示，直接引用 `tauri` 或 Tauri plugin 的 Rust 文件只有：

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/*.rs`
- `src-tauri/src/tray.rs`
- `src-tauri/src/services/autostart_service.rs` 中的 `TauriAutostartBackend`

`models/`、Provider/事务/设置服务和多数基础设施模块本身不依赖 Tauri。自检、文件 watcher、
开机启动和日志虽可在语法上脱离 Tauri，但只由桌面应用生命周期消费；是否纳入 core 必须由实际
Provider 重建基准决定，而不能只按“是否 import Tauri”机械分类。

### 源码与测试分布

当前根 lib 测试共 172 项，按顶层模块分布：

| 模块 | 测试数 |
|---|---:|
| `app_state` | 1 |
| `commands` | 9 |
| `error` | 1 |
| `infrastructure` | 54 |
| `models` | 10 |
| `services` | 86 |
| `tray` | 11 |

初始“全部平台无关逻辑进入 core”的边界可迁入约 151 项测试，根 crate 仅保留约 21 项 Tauri
组合与适配测试。实施基准证明该边界仍过宽后，最终将桌面生命周期模块留在根 crate：core 为
134 项单元测试，根 crate 为 39 项单元测试，另有 core 的 3 项 path safety 和 1 项 provider
workflow 集成测试。拆分前根测试可执行文件约 17.8 MiB，PDB 约 177.8 MiB。

### 依赖闭包

在 `x86_64-pc-windows-msvc` 上使用 `cargo metadata --filter-platform` 分析当前解析图：

- 当前 Windows 元数据共 335 个包。
- 初始“全部平台无关”core 候选的非 dev 闭包约 157 个包。
- Tauri 与五个应用 plugin 的非 dev 闭包约 312 个包。
- Tauri 路径相对 core 候选额外引入约 174 个包。

该统计包含 feature 合并后的当前解析图，不能直接等同编译秒数，但足以证明把 Provider 测试从
Tauri 路径切出会显著缩小需要解析、编译和链接的依赖面。

最终 Provider-centered core 在 Windows 上的非 dev 闭包为 133 个包；移出 notify、tracing
appender/subscriber 及桌面生命周期源码，同时保留 core 内部错误脱敏所需 regex 后，core 单元
测试 exe 约 12.9 MiB、PDB 约 102.0 MiB。

### 当前反馈基准

- 上一阶段：默认 target 缓存命中 4.12 秒；仅触发根 crate 重建 26.81 秒；独立冷构建
  422.67 秒、2.68 GiB；完整 `npm run check` 352 秒。
- 本任务研究阶段新鲜缓存命中：`npm run test:rust:lib -- provider_http` 为 6/6，通过；Cargo
  测试执行 1.06 秒，含 watcher 门禁和 npm 启动的墙钟时间 9.58 秒。
- 缓存命中已低于 20 秒，但源码变更后的 26.81 秒根 crate 重建仍超过门禁；拆分必须以重复的
  core 源码时间戳重建测量，而不能只报告缓存命中。
- 研究时一次 `cargo tree --no-dev` 因当前 Cargo 不支持该参数而失败；随后改用
  `cargo tree -e normal,build --depth 1` 成功。失败不能作为依赖图证据。

## 备选方案

### 方案 A：Provider-centered core + Tauri/桌面生命周期应用 crate（推荐）

在 `src-tauri` 内建立 Cargo workspace，新增 `crates/codex-relay-core`。把 error、全部 models、
Provider/设置/事务/备份 services，以及路径/原子文件/Provider 网络/Codex runner infrastructure
迁入 core；根 crate 保留 `run`、commands、AppState、tray，以及 autostart、file watch、自检和
日志等桌面生命周期模块，并通过 module re-export 保持现有 Rust 路径。

优点：边界由实际 Provider 反馈目标决定，不复制 Provider/事务逻辑；134 项 core 测试和两项 core
integration targets 无需 Tauri。缺点：根 crate 仍拥有少量不直接 import Tauri 的运行时模块，
边界需要按“领域/生命周期所有权”解释，而不是单纯按依赖名称解释。

### 方案 B：只抽 Provider HTTP/gateway 与少量模型

只迁移 `provider_http`、`codex_gateway`、Provider availability DTO 和最少依赖。

优点：首轮文件变动较小。缺点：这些基础设施当前依赖 `ProviderAvailabilityTarget`，而目标解析又
依赖 Provider/settings/secret 服务；要么复制类型和装配逻辑，要么引入大量 trait/feature，最终
仍会逐步迁移同一批服务。它只能优化少数单元测试，不能让完整 Provider 行为切片独立。

### 方案 C：立即拆成 domain/network/transaction/desktop 多 crate

把模型、事务、Provider 网络和 Tauri desktop 各自拆成 crate。

优点：理论上获得更细粒度的增量编译。缺点：当前代码规模和一个明确的 20 秒目标不足以证明
多 crate 的 API、版本和 feature 管理成本；跨 crate 可见性变化更大，设计/回滚风险明显高于本阶段。

## 决策

采用方案 A。目录选择 `src-tauri/crates/codex-relay-core`：Cargo workspace 与现有 Tauri manifest、
锁文件和固定 `src-tauri/target` 保持同一根，不需要把仓库根改成虚拟 workspace，也不改变 Tauri
CLI 的 manifest 发现和发布产物路径。

同时把 `ProviderAvailabilityTarget` 从 `provider_service` 移到 Provider availability 模型，消除
infrastructure 反向依赖 service 的现有层级泄漏；该类型仍为 core crate 内部契约，不改变 command
或前端 DTO。

实施中的第一版把全部平台无关模块放入 core，强制 Provider 源码重建的 Cargo 时间仍为 24.19 秒，
完整 npm 入口为 29.85 秒，未达标。把日志生命周期、file watch、自检和 autostart 服务按桌面
生命周期所有权移回根 crate，并把错误脱敏纯函数留在 core 以避免公开原始错误详情后，最终连续
三次完整 Provider 快速入口为 10.49、11.38、11.30 秒；对应 Cargo 编译/链接为 6.15、7.68、
7.06 秒，证明最终边界稳定低于 20 秒。

## 目标结构

```text
src-tauri/Cargo.toml                         workspace + Tauri 应用 package
src-tauri/src/
  lib.rs                                    Tauri 组合入口 + core 模块 re-export
  app_state.rs
  commands/
  tray.rs
  services/                                 autostart / file watch / self-check
  infrastructure/safe_log.rs                桌面日志生命周期
  tauri_autostart_backend.rs                Tauri plugin adapter
src-tauri/crates/codex-relay-core/
  Cargo.toml                                无 Tauri 依赖
  src/
    lib.rs
    error.rs
    models/
    services/                               Provider / 设置 / 事务 / 备份
    infrastructure/                         路径 / 文件 / Provider 网络 / Codex runner
  tests/
    path_safety.rs
    provider_workflow.rs
```

## 兼容与风险结论

- 根 crate re-export `error`、`models`、`services`、`infrastructure`，现有 app 内部路径和外部测试
  路径保持稳定。
- `TauriAutostartBackend` 实现根 crate `AutostartBackend` trait；Autostart 错误码和服务测试不变。
- core 使用普通 `rlib`，根 crate 继续保留 `lib`/`cdylib`/`staticlib` 和 Tauri build script。
- `cargo test`、Clippy 和 fmt 必须显式使用 workspace 范围，否则 core 单元/集成测试会被漏掉。
- 快速脚本必须使用 `-p codex-relay-core`；依赖图门禁必须额外拒绝 core 依赖树中的任何 Tauri 包。
- 文件迁移只改变源码位置；所有 Provider/事务/路径测试继续使用相同 fixture 和临时目录。
- fixture 相对路径必须随新目录深度统一更新，且仍只引用仓库中的 `test-key-*-not-real` 文件。

## 尚需用户决定的事项

无。研究未发现需要改变产品行为、安全边界、发布格式或扩大到多 crate 分层的理由，符合用户已
授予的直接实施范围。
