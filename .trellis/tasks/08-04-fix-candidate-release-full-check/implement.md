# 修复候选发布完整检查失败实施计划

## 当前进度

- [x] 读取 Run `30869666756` 的 Job 与失败日志，确认唯一失败测试和未生成 Draft 的边界。
- [x] 对照上一轮 Run `30833079285` 与提交 `cfe932c`，确认 8.3 路径问题已通过且本次根因独立。
- [x] 读取 `codex_process` 实现、引入提交、Git blame、Windows 冷 Runner 测试规范和发布重试契约。
- [x] 记录本机基线：专项 1/1 与完整 core 249/249 通过；远端红灯在硬编码 5 秒处稳定失败。
- [x] 红灯复核：Run `30869666756` 在收到首段输出后于 5.03 秒返回 `Timeout`；本机缓存环境基线通过。
- [x] 绿灯：把流式输出夹具改为条件协调，并统一使用 `PROCESS_TREE_TEST_TIMEOUT`。
- [x] 连续运行 `codex_process` 全部 9 项专项 3 次，三轮均为 9/9 通过。
- [x] 运行 Rustfmt、Clippy 和完整项目门禁。
- [x] 更新 Windows 冷 Runner 测试规范，记录正常完成预算与流式条件协调契约。
- [x] 完成 Trellis 全范围检查、差异检查、路径隔离和秘密扫描。
- [ ] 精确提交修复并普通 push，验证 `origin/main` 与本地 `HEAD` 一致。
- [ ] 触发并监控唯一的新候选发布 Run；成功后审计 `v0.5.0` Draft 及三类资产，不公开。
- [ ] 更新最终证据，归档任务、记录会话日志并推送全部收尾提交。

## 验证证据

- 远端红灯：Run `30869666756` 的 Step #7 退出 1；唯一失败是流式输出测试在收到首段输出后于
  约 5.03 秒得到 `Timeout`，core 248 项通过，Step #8 跳过且未生成 Draft。
- 诊断期间第一次 npm 快速测试把 `--exact` 传给 Cargo 主参数而退出 1；修正分隔符后因完整测试名
  不匹配运行了 0 项。去掉 `--exact` 后基线 1/1 通过，这两次调用不计为测试通过证据。
- 首次绿灯编译因释放标记路径被移动后再次借用而退出 1；克隆子进程参数路径后相同专项 1/1 通过。
- 最终代码下 `codex_process::tests` 串行连续 3 轮均为 9/9 通过，执行时间分别为 9.64、9.39、
  9.47 秒；没有接受 `Timeout` 作为等价结果。
- `cargo fmt --all --check --manifest-path src-tauri/Cargo.toml` 初次指出一处自动换行差异；运行 formatter
  后复查退出 0。workspace Clippy `--all-targets --all-features -- -D warnings` 退出 0。
- 成对安全 Relay 覆盖下 `npm run check` 退出 0，用时 678.4 秒：Trellis 8 项、主前端
  60 个文件/338 项、Rust core 249 项、路径安全 3 项、Provider 工作流 1 项及 release-console
  Rust 套件通过；1 个完整项目后端探针按既有设计 ignored。
- 完整门禁使用的 `codex-relay-final-check-*` 路径解析在系统 `%TEMP%` 下；命令结束后该目录已不存在。
- `git diff --check` 退出 0；改动只位于 Windows 测试模块、测试规范与当前任务材料。高置信度
  API Key/Bearer/Authorization 扫描无命中，`artifacts/` 与 `src-tauri/target/` 保持 ignored。
- 尚未创建修复提交、push、重跑远端 workflow 或生成 Draft；这些项目继续保持未完成。

## 验证命令

```powershell
$safeRoot = Join-Path $env:TEMP ('codex-relay-check-' + [guid]::NewGuid().ToString('N'))
$env:CODEX_RELAY_CODEX_HOME = Join-Path $safeRoot 'codex-home'
$env:CODEX_RELAY_APP_DATA_DIR = Join-Path $safeRoot 'app-data'
npm run test:rust:lib -- codex_process::tests -- --test-threads=1
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features -- -D warnings
npm run check
git diff --check
```

远端阶段使用新提交的精确 SHA 触发 `.github/workflows/release.yml`，监控到终态后读取 Run、Job、
Step 和 Draft Release API 证据。任何失败都保留实际输出，不把本地通过写成远端发布成功。

## 风险文件与回滚点

- `src-tauri/crates/codex-relay-core/src/infrastructure/codex_process.rs`：只允许修改测试模块；若出现
  生产 diff，先停止并重新审查范围。
- `.trellis/spec/testing/rust-build-feedback.md`：记录长期测试时序契约，不写入一次性成功声明。
- GitHub Draft：仅在 Run 全绿后由现有 workflow 创建；审计失败时保持 Draft，不公开、不改 Tag。
- 真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay` 始终不在测试或清理范围内。

## 缺陷复盘

### 1. 根因类别

- **D：测试覆盖缺口**：本机缓存环境与此前远端 Run 未覆盖新增流式测试在冷 Runner 超过 5 秒的路径。
- **E：隐式假设**：夹具把 5 秒完成预算和 500 毫秒调度窗口当成 Windows CI 不变量。

### 2. 修复过程中的失败

- 首次绿灯编译因 `release_file` 同时被异步任务移动和主测试借用而失败；改为只克隆子进程参数路径，
  主测试继续拥有原路径。该失败没有进入行为运行，也没有通过放宽所有权或使用共享可变状态绕过。

### 3. 预防机制

| 优先级 | 机制 | 具体行动 | 状态 |
|---|---|---|---|
| P0 | 测试结构 | 流式先后关系改为临时释放标记，不使用固定短 sleep | 已完成 |
| P0 | 共享预算 | 正常完成的真实 PowerShell 测试统一复用 30 秒测试预算 | 已完成 |
| P0 | 远端证据 | 本地全量通过后只触发一个新候选 Run，并要求 Draft 审计通过 | 待执行 |
| P1 | 长期规范 | 更新 Windows 冷 Runner 场景、错误矩阵、测试与反例 | 已完成 |

### 4. 系统性扩展

- 同文件扫描发现输出捕获与结构化 stdin 测试也使用本机经验的 5 秒预算，已一并收口；显式验证
  `Timeout`、取消和 Job 分配失败的短预算保留，因为这些值本身就是被测行为。
- 生产 `SafeProcessRunner`、发布 workflow 和远端监控无需改变；问题位于测试夹具层，而非架构层。

### 5. 知识沉淀

- 已更新 `.trellis/spec/testing/rust-build-feedback.md`；现有索引已覆盖该文件，无需新增索引项。
- 仓库不存在 `src/templates/markdown/spec/`，没有可同步的规范模板。
