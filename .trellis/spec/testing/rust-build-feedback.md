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

## Scenario：Windows 冷 runner 进程树测试

### 1. 范围/触发条件

- 触发条件：Windows Rust 测试启动 PowerShell、创建后代进程、等待 PID 文件、验证进程完成前的
  流式输出，或通过大量 stdout 触发进程输出上限。
- 目标是让测试验证 Job Object 的取消与输出限制语义，而不是把 GitHub 冷 runner 的进程启动和
  管道吞吐误当成 5 秒产品 SLA。

### 2. 签名

- Windows 测试模块使用 `PROCESS_TREE_TEST_TIMEOUT: Duration = Duration::from_secs(30)` 作为有界的
  测试级总预算。
- core 与 release-console 的 PowerShell 夹具入口显式设置 `$PSModuleAutoLoadingPreference = 'None'`
  和 `$ErrorActionPreference = 'Stop'`。夹具只使用直接 .NET API，不依赖 Utility/Management 模块自动加载。
- release-console 集成测试使用同值的 `WINDOWS_PROCESS_TEST_TIMEOUT`；其流式 PowerShell 夹具还必须
  在 20 秒内未收到释放标记时自行失败，短于外层测试预算。
- 预期真实 PowerShell 正常完成、产生输出或消费 stdin 的测试统一把该常量传给 runner；只有专门
  验证 `Timeout` / `Cancelled` 的测试可以传入更短的行为预算。
- 父 PowerShell 必须使用 `[System.Diagnostics.ProcessStartInfo]::new()` 直接创建后代，设置
  `UseShellExecute = false` 与 `CreateNoWindow = true`；不要在“挂起后加入 Job Object 再恢复”的
  启动链中依赖 `Start-Process` 的 ShellExecute 包装。
- 父进程创建后代后立即把返回的 `$child.Id` 写入 PID 文件；
  `wait_for_process_id(path, status_path, error_path) -> u32` 每 20 毫秒重新读取该文件，直到条件满足
  或测试预算到期。
- 父脚本在创建前、创建后和写入 PID 后写入临时状态文件；捕获异常时写入临时错误文件，超时消息必须带上
  这些非秘密诊断，不能只报告一个无区分度的超时。写入使用 `[IO.File]::WriteAllText`，异常使用
  `$_.Exception.ToString()`，避免诊断自身依赖 `Set-Content` 或 `Out-String`。
- `SystemCodexProcessBackend::run(..., PROCESS_TREE_TEST_TIMEOUT, ...)` 用于需要等待 PowerShell
  正常完成、启动后代、产生输出或触发输出上限的测试；生产调用方的显式超时不因此改变。

### 3. 契约

- 等待后代进程时必须轮询“PID 文件已写入且可解析”这一真实条件，不能用固定 sleep 猜测启动时间。
- PowerShell 字节数组通过 `[byte[]]::new(size)` 创建，文件存在性通过 `[IO.File]::Exists` 检查，
  轮询和挂起夹具通过 `[Threading.Thread]::Sleep` 实现；父进程启动的后代也禁用模块自动加载。
- 禁用自动加载后出现 `CommandNotFoundException` 表示夹具误用了模块 cmdlet，应修正夹具，不能重新开启
  自动加载、扩大生产超时或把 `Timeout` 接受为预期输出限制结果。
- 验证进程完成前流式输出时，子进程必须先 flush 首段输出，再等待测试位于临时目录的释放标记；
  测试收到事件并确认 runner 尚未结束后写入标记。不得用固定 500 毫秒 sleep 猜测测试线程会及时调度。
- PID 必须由创建后代的父进程从 `ProcessStartInfo` 返回的 `Process` 对象写入；不能要求子 PowerShell 先完成
  自身冷启动并执行脚本后再写 PID，因为那验证的是子运行时就绪，不是 Job Object 所需的“后代已创建”。
- `Start-Process` 在本机缓存环境可能通过，但其 ShellExecute 包装在冷 runner 的挂起/Job Object 嵌套链中
  可能阻塞；不得把本机通过视为该启动边界在 CI 中稳定。
- 轮询必须有清晰的总 deadline；条件始终不满足时在 deadline 后失败，不能无限等待。
- 后代取消测试只能在已取得子 PID 后发送取消信号，再断言运行结果为 `Cancelled` 且子进程已退出。
- 输出上限测试必须给冷 PowerShell 和管道足够的测试预算，再断言错误为 `OutputTooLarge`；若先得到
  `Timeout`，应视为测试预算或实现退化的真实失败，不能放宽断言。
- 30 秒仅是 CI 测试的冷启动容差，不是产品行为、性能承诺或默认运行超时。
- 普通输出捕获、结构化 stdin、流式事件和大 stdout 文件测试不得各自重新引入 5 秒等本机经验预算；
  共享常量让冷 Runner 容差保持一致，条件满足后测试仍会立即继续。
- release-console 的生产 `LOCAL_COMMAND_TIMEOUT` 保持 2 小时。集成测试不得用 10 秒外层等待模拟该
  产品预算；测试脚本自截止负责防止断言失败后留下长命令，30 秒外层预算只约束测试。

### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 冷 runner 在 5 秒后、30 秒内创建后代并由父进程写入有效 PID | 继续发送取消并验证整个进程树终止 |
| 子 PowerShell 用户代码启动很慢，但 `ProcessStartInfo` 已返回 | 父进程仍立即记录 PID，不等待子脚本就绪 |
| deadline 内始终没有有效 PID | 测试失败消息同时包含父脚本最后阶段和异常诊断（若存在） |
| 超限输出在 deadline 内被读取 | 返回 `OutputTooLarge`，并终止进程树 |
| 超限输出测试先返回 `Timeout` | 保留失败并调查吞吐、读取或预算，不把 `Timeout` 接受为等价结果 |
| 首段输出已到达，但 PowerShell 在 5 秒内未完成 | 正常完成测试继续使用共享 30 秒预算；不得把 CI 调度延迟当成产品超时 |
| 流式测试收到首段输出 | runner 必须仍未结束；写入临时释放标记后再断言完整输出与退出码 |
| release-console 已持久化首段日志 | 写入临时释放标记；30 秒内返回完整日志与退出码，脚本自身最多等待 20 秒 |
| release-console 测试在写标记前失败 | 子脚本按 ASCII 稳定码自截止，不继续占用 2 小时生产命令预算 |
| 普通产品调用传入更短超时 | 仍按调用方超时返回 `Timeout`，不受测试常量影响 |
| 禁用自动加载后使用 `New-Object`、`Set-Content` 等模块 cmdlet | 夹具快速失败，改用直接 .NET API |
| 直接 .NET 夹具禁用自动加载 | PID、流式输出、超限输出和文件输出仍满足原来的全部断言 |

### 5. 良好/基线/错误用例

- 良好：父进程用 `ProcessStartInfo` 直接创建后代，在返回时写入 `$child.Id`，测试按 20 毫秒轮询并立即
  继续，不等待子 PowerShell 执行用户脚本；启动阶段和异常写入临时诊断文件。
- 良好：流式测试收到 `first` 后确认 runner 未完成，写入临时释放标记，随后得到 `firstsecond` 和
  退出码 0；没有固定等待窗口。
- 良好：release-console 从持久化日志读取到 `first\n` 后写标记，随后读取 `second-tail`；夹具未收到
  标记时在 20 秒自行退出。
- 基线：缓存命中的本机在数百毫秒内满足条件，测试仍快速结束。
- 错误：在该启动链中使用 `Start-Process` ShellExecute 包装、让子脚本执行 `Set-Content -Value $PID`，
  把“已创建后代”错误提升为“后代 PowerShell 已完成冷启动”；或用 `for _ in 0..250` 把轮询隐式限制
  为 5 秒。
- 错误：正常完成测试硬编码 `Duration::from_secs(5)`，或用 `Start-Sleep -Milliseconds 500` 证明
  “输出发生在完成前”；两者都把本机调度速度误当成 CI 契约。

### 6. 必需测试

- Windows `codex_process` 专项测试必须全部通过；正常完成测试统一断言共享预算，显式超时和取消
  测试继续断言调用方行为预算。
- `cancellation_terminates_descendant_processes_in_the_job`、
  `output_limit_terminates_the_process_tree` 和
  `generic_runner_streams_output_before_process_completion`、
  `generic_runner_streams_large_stdout_directly_to_new_file` 在相关修复时各连续运行至少 3 次，确认没有
  偶然缓存命中。
- release-console `process_backend_persists_safe_output_before_the_command_completes` 在相关修复时连续
  运行至少 3 次，并运行完整 `local_verification` 集成套件。
- 完成前运行 `npm run check`；GitHub Actions 的冷 runner 也必须通过当前完整 core 测试后才能
  进入 Draft 构建。

### 7. 错误与正确做法

#### 错误

```powershell
# 子进程只有在 PowerShell 冷启动并开始执行脚本后才报告 PID。
Set-Content -LiteralPath $PidFile -Value $PID
Start-Sleep -Seconds 120
```

```rust
for _ in 0..250 {
    if let Ok(pid) = read_pid(path) {
        return pid;
    }
    tokio::time::sleep(Duration::from_millis(20)).await;
}
```

```rust
SafeProcessRunner::default()
    .run(invocation, Duration::from_secs(5), cancel, Some(sink))
    .await?;
// 子脚本固定 sleep 500ms，假设测试线程必然在窗口内获得调度。
```

#### 正确

```powershell
$PSModuleAutoLoadingPreference = 'None'
$ErrorActionPreference = 'Stop'
$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = "$PSHOME\powershell.exe"
$startInfo.Arguments = '-NoLogo -NoProfile -NonInteractive -Command "$PSModuleAutoLoadingPreference = ''None''; [Threading.Thread]::Sleep(120000)"'
$startInfo.UseShellExecute = $false
$startInfo.CreateNoWindow = $true
$child = [System.Diagnostics.Process]::Start($startInfo)
[IO.File]::WriteAllText($PidFile, $child.Id.ToString())
$child.WaitForExit()
```

```rust
let deadline = Instant::now() + PROCESS_TREE_TEST_TIMEOUT;
while Instant::now() < deadline {
    if let Ok(pid) = read_pid(path) {
        return pid;
    }
    tokio::time::sleep(Duration::from_millis(20)).await;
}
```

```rust
SafeProcessRunner::default()
    .run(invocation, PROCESS_TREE_TEST_TIMEOUT, cancel, Some(sink))
    .await?;
// 子脚本 flush 首段输出后等待临时释放标记；测试观察事件后立即写标记。
```

### 8. 失败复盘与防复发

- **根因类别**：D（测试覆盖缺口）+ E（隐式时序/启动边界假设）。第一个修复把轮询预算从 5 秒扩大到
  30 秒，第二个修复把 PID 观察点移到父进程，但仍保留 `Start-Process` ShellExecute 启动边界；本地缓存
  命中不能覆盖冷 runner 的挂起与 Job Object 嵌套环境。
- **本次证据**：Run `30167838439` 的前端 178 项和 core 171 项通过，唯一失败为
  `cancellation_terminates_descendant_processes_in_the_job`，30 秒内 PID 文件为空；本地改用
  `ProcessStartInfo` 后后代取消和输出上限测试各连续 3 次通过。
- **预防机制**：测试夹具使用无 ShellExecute 的直接创建 API；阶段/异常诊断仅写入安全临时目录，并在
  deadline 错误中呈现；专项测试和完整 `npm run check` 必须在进入发布 Draft 前通过。该修复不改变生产
  Job Object、取消、超时或输出上限逻辑。

### 9. 流式输出固定短预算复盘

- **根因类别**：D（冷 Runner 覆盖缺口）+ E（隐式调度假设）。新增通用 runner 测试没有复用同模块
  已建立的 `PROCESS_TREE_TEST_TIMEOUT`，并用固定 500 毫秒 sleep 表达“进程仍在运行”。
- **区分证据**：Run `30869666756` 已收到 `first`，但用例在约 5.03 秒精确返回 `Timeout`；同轮其余
  248 项 core 测试和本机 249 项基线通过，排除了发布监控、版本门禁和流式读取完全失效。
- **预防机制**：所有预期真实 PowerShell 正常完成的测试复用共享冷 Runner 预算；跨异步边界的先后关系
  使用临时标记等可观察条件协调；相关 `codex_process` 套件连续 3 次、完整本地门禁和唯一远端发布 Run
  共同构成完成证据。
- **传播补充**：第一次系统性扫描只覆盖 core 文件，遗漏了 release-console 对同一
  `SafeProcessRunner` 的集成测试。Run `30873762688` 中 core 249 项全部通过，随后该集成测试在固定
  10 秒完成等待返回 `Elapsed(())`。预防扫描必须覆盖 `tools/release-console/src-tauri/tests`，不能按
  crate 边界提前停止。

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
