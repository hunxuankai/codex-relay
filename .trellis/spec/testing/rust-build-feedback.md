# Rust 开发编译反馈契约

## Scenario：日常 Rust 行为切片

### 1. 范围/触发条件

- 触发条件：开发者或 AI 修改 `src-tauri` Rust 源码、`codex-relay-core` 源码并运行专项单元/
  集成测试，或启动完整 Tauri 开发应用。
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
  -p codex-relay-core
  ```

- `src-tauri/Cargo.toml` 是包含 Tauri 应用与 `crates/codex-relay-core` 的 workspace 根；core 的
  manifest 和真实依赖树不得包含 `tauri` 或 `tauri-plugin-*`。

- watcher 门禁只读 `node.exe` 的 Tauri CLI 命令行；不得终止、暂停或重启任何进程。
- 普通 `tauri dev` 视为冲突；带 `--no-watch` 或没有 Tauri dev 时允许测试。
- 直接 `reqwest` 使用 `rustls-no-provider`，项目显式启用并在客户端构造前安装 Rustls `ring`；
  依赖图不得包含 `aws-lc-sys`。
- `check:rust` 必须先运行依赖图检查，再以 workspace 范围运行 fmt、Clippy 和完整 Rust tests；
  不能只测试依赖 core 的根 package 而漏掉 core 自身的 `#[cfg(test)]` 与 integration targets。

### 4. 验证与错误矩阵

| 条件 | 结果 |
|---|---|
| 检测到普通 `tauri dev` watcher | 快速测试在 Cargo 前退出 2，并提示安全 no-watch/前端入口 |
| Tauri dev 带 `--no-watch` | 门禁通过 |
| 没有 Tauri dev | 门禁通过 |
| 依赖图出现 `aws-lc-sys` | `check:rust:deps` 失败 |
| 依赖图缺少 Rustls `ring` | `check:rust:deps` 失败 |
| `codex-relay-core` 依赖图出现 Tauri | `check:rust:deps` 失败 |
| `ring` 未在 reqwest Client 构造前安装 | 专项测试必须捕获 panic/构造失败，不得忽略 |
| 最终构建目标被运行进程锁定 | 如实报告失败；关闭应用后重试，不把该次计为成功 |

### 5. 良好/基线/错误用例

- 良好：Rust TDD 使用 `dev:safe:no-watch` 加 `test:rust:lib -- provider_http`，只编译/链接 core
  test target，完成后再运行 workspace 完整检查。
- 良好：纯 Vue 修改只运行 `dev:frontend`，不启动 Rust watcher。
- 基线：没有开发应用时直接运行快速 Rust 测试，门禁静默通过。
- 错误：每个任务创建带随机后缀的 `CARGO_TARGET_DIR`，把缓存命中变成完整冷编译。
- 错误：保持 `tauri dev` watcher 运行，同时在另一 target 执行 Cargo 测试。
- 错误：为缩短当前任务删除整个 `target`；这只会把成本推迟到下一次构建。

### 6. 必需测试及断言点

- `test:rust-dev-guard`：合成普通 watcher、`--no-watch`、无进程三种命令行；断言退出码和行动建议。
- `test:rust-dev-guard`：dry-run 断言安全双路径和 `npm.cmd run dev -- --no-watch`，不启动真实应用。
- `test:rust-deps`：合成依赖图断言 aws-lc 失败、缺 ring 失败、ring-only 通过。
- `test:rust-dev-guard`：断言三个快速 Rust 入口选择 `codex-relay-core`，完整门禁使用 workspace。
- `test:rust-deps`：合成 core 依赖图断言出现 Tauri 失败、无 Tauri 通过。
- `check:rust:deps`：真实 Cargo feature 图不含 `aws-lc-sys` 且含 ring，core 真实依赖树不含 Tauri。
- Provider HTTP/gateway tests：断言 Client 可构造，代理、认证、JSON、SSE、重定向与错误分类不变。
- 完成前运行 path safety、provider workflow 和 `npm run check`；报告区分编译时间与测试执行时间。

Provider 编译反馈基准必须至少连续 3 次更新时间戳但不改源码内容，再运行完整快速 npm 入口；
每次墙钟和 Cargo 编译/链接时间都要记录。只报告缓存命中或第一次新 package 冷生成不能证明稳定
反馈。日常目标为每次完整 Provider 快速入口低于 20 秒。

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

## Scenario：Windows PowerShell 原生命令诊断

### 1. 范围/触发条件

- 触发条件：PowerShell 脚本在 `$ErrorActionPreference = 'Stop'` 下包装 `cargo`、`npm` 或其它原生命令，并把 stdout/stderr 合并为结构检查输入。
- 目标是区分“原生命令退出失败”和“成功命令向 stderr 输出进度/诊断信息”，避免 Windows runner 首次更新 crates.io index 时误报失败。

### 2. 签名

- `scripts/check-rust-dependency-graph.ps1 -CargoExecutable <command>`：默认执行 `cargo`，测试可注入位于安全临时目录的等价命令。
- 内部 `Invoke-CargoTree -Arguments <string[]>` 返回 `{ Lines, ExitCode }`，不把原始 stderr 当作业务成功标志。

### 3. 契约

- 调用原生命令前暂时把 `$ErrorActionPreference` 设为 `Continue`，使用 `2>&1` 收集诊断行，并立即保存 `$LASTEXITCODE`；`finally` 中恢复调用方原值。
- 退出码为 `0` 时，即使 stderr 有 `Updating crates.io index` 等进度行，也继续解析 stdout 并完成依赖图校验。
- 退出码非 `0`、命令找不到或参数错误时，打印收集到的诊断并返回失败；不得把 stderr 静默吞掉，也不得仅凭输出文本判断成功。

### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| stdout 正常、stderr 为空、退出码 0 | 依赖图检查通过 |
| stdout 正常、stderr 有进度信息、退出码 0 | 依赖图检查通过，保留诊断供调试 |
| stderr 有错误且退出码非 0 | 依赖图检查失败并保留诊断 |
| workspace 或 core 任一 Cargo 调用失败 | 立即失败，不继续使用不完整依赖图 |

### 5. 良好/基线/错误用例

- 良好：安全临时 `.cmd` 输出 `Updating crates.io index` 到 stderr、输出 ring 依赖图并退出 0，脚本返回 0。
- 基线：本机 Cargo 缓存命中且没有 stderr，脚本仍按相同 `{ Lines, ExitCode }` 契约运行。
- 错误：在 `Stop` 偏好下直接执行 `& cargo tree ... 2>&1`，把退出 0 的 stderr 记录提升为 `NativeCommandError`。

### 6. 必需测试

- `scripts/tests/rust-dependency-graph.tests.ps1` 必须注入临时 fake Cargo，断言 stderr+退出 0 通过、workspace/core 两次调用均被执行，且真实成功摘要仍出现。
- 同一测试必须保留 aws-lc、缺 ring、core 含 Tauri、core 不含 Tauri 的退出码断言；测试结束验证并删除临时目录。
- CI 真实 `npm run check:rust:deps` 必须在冷 Cargo index 和缓存命中两种情况下都依据退出码判定，不依据 stderr 是否为空判定。

### 7. 错误与正确做法

#### 错误

```powershell
$ErrorActionPreference = 'Stop'
$lines = & cargo tree ... 2>&1
```

#### 正确

```powershell
$previous = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
  $lines = & $CargoExecutable @Arguments 2>&1
  $exitCode = $LASTEXITCODE
} finally {
  $ErrorActionPreference = $previous
}
```
