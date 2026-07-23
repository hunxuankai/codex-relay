# Rust 开发编译反馈契约

## Scenario：日常 Rust 行为切片

### 1. 范围/触发条件

- 触发条件：开发者或 AI 修改 `src-tauri` Rust 源码并运行专项单元/集成测试，或启动完整 Tauri
  开发应用。
- 目标是避免同一源码变化同时触发 Tauri watcher 与手动 Cargo、避免随机 target 冷编译，并防止
  重复 TLS provider 扩大依赖图。
- 快速入口只用于行为切片；提交前仍必须运行完整 `npm run check`。

### 2. 签名

安全开发入口：

```powershell
npm run dev:safe
npm run dev:safe:no-watch
npm run dev:frontend
```

快速 Rust 测试入口：

```powershell
npm run test:rust:lib -- <filter>
npm run test:rust:path-safety
npm run test:rust:provider-workflow
```

基础设施脚本：

```powershell
scripts/prepare-dev-data.ps1 [-PrepareOnly] [-NoRustWatch] [-DryRun]
scripts/check-rust-dev-environment.ps1
scripts/check-rust-dependency-graph.ps1
```

`-DryRun` 只用于验证安全路径与最终 Tauri 参数，不启动 Node、Cargo 或应用。

### 3. 契约

- `dev:safe:no-watch` 必须复用 `prepare-dev-data.ps1`，先设置
  `CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR`，再执行
  `npm.cmd run dev -- --no-watch`；不得创建不带双覆盖的普通 no-watch 入口。
- 快速 Rust npm script 必须先运行 watcher 门禁，并显式使用：

  ```text
  --manifest-path src-tauri/Cargo.toml
  --target-dir src-tauri/target
  ```

- watcher 门禁只读 `node.exe` 的 Tauri CLI 命令行；不得终止、暂停或重启任何进程。
- 普通 `tauri dev` 视为冲突；带 `--no-watch` 或没有 Tauri dev 时允许测试。
- 直接 `reqwest` 使用 `rustls-no-provider`，项目显式启用并在客户端构造前安装 Rustls `ring`；
  依赖图不得包含 `aws-lc-sys`。
- `check:rust` 必须先运行依赖图检查，再运行 fmt、Clippy 和完整 Rust tests。

### 4. 验证与错误矩阵

| 条件 | 结果 |
|---|---|
| 检测到普通 `tauri dev` watcher | 快速测试在 Cargo 前退出 2，并提示安全 no-watch/前端入口 |
| Tauri dev 带 `--no-watch` | 门禁通过 |
| 没有 Tauri dev | 门禁通过 |
| 依赖图出现 `aws-lc-sys` | `check:rust:deps` 失败 |
| 依赖图缺少 Rustls `ring` | `check:rust:deps` 失败 |
| `ring` 未在 reqwest Client 构造前安装 | 专项测试必须捕获 panic/构造失败，不得忽略 |
| 最终构建目标被运行进程锁定 | 如实报告失败；关闭应用后重试，不把该次计为成功 |

### 5. 良好/基线/错误用例

- 良好：Rust TDD 使用 `dev:safe:no-watch` 加 `test:rust:lib -- provider_http`，只构建一个稳定
  target，完成后再运行完整检查。
- 良好：纯 Vue 修改只运行 `dev:frontend`，不启动 Rust watcher。
- 基线：没有开发应用时直接运行快速 Rust 测试，门禁静默通过。
- 错误：每个任务创建带随机后缀的 `CARGO_TARGET_DIR`，把缓存命中变成完整冷编译。
- 错误：保持 `tauri dev` watcher 运行，同时在另一 target 执行 Cargo 测试。
- 错误：为缩短当前任务删除整个 `target`；这只会把成本推迟到下一次构建。

### 6. 必需测试及断言点

- `test:rust-dev-guard`：合成普通 watcher、`--no-watch`、无进程三种命令行；断言退出码和行动建议。
- `test:rust-dev-guard`：dry-run 断言安全双路径和 `npm.cmd run dev -- --no-watch`，不启动真实应用。
- `test:rust-deps`：合成依赖图断言 aws-lc 失败、缺 ring 失败、ring-only 通过。
- `check:rust:deps`：真实 Cargo feature 图不含 `aws-lc-sys` 且含 ring。
- Provider HTTP/gateway tests：断言 Client 可构造，代理、认证、JSON、SSE、重定向与错误分类不变。
- 完成前运行 path safety、provider workflow 和 `npm run check`；报告区分编译时间与测试执行时间。

### 7. 错误与正确做法

#### 错误

```powershell
$env:CARGO_TARGET_DIR = Join-Path $env:TEMP ("codex-relay-" + [guid]::NewGuid())
npm run dev:safe
cargo test --manifest-path src-tauri/Cargo.toml --lib provider_http
```

该方式同时产生 watcher 构建和全新测试缓存。

#### 正确

```powershell
npm run dev:safe:no-watch
npm run test:rust:lib -- provider_http
```

需要观察最新 Rust 应用行为时，主动重启安全 no-watch 应用；完整 fmt/Clippy/集成测试留到阶段门禁。
