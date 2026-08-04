# 修复候选发布完整检查失败实施计划

## 当前进度

- [x] 读取 Run `30869666756` 的 Job 与失败日志，确认唯一失败测试和未生成 Draft 的边界。
- [x] 对照上一轮 Run `30833079285` 与提交 `cfe932c`，确认 8.3 路径问题已通过且本次根因独立。
- [x] 读取 `codex_process` 实现、引入提交、Git blame、Windows 冷 Runner 测试规范和发布重试契约。
- [x] 记录本机基线：专项 1/1 与完整 core 249/249 通过；远端红灯在硬编码 5 秒处稳定失败。
- [x] 红灯复核：Run `30869666756` 在收到首段输出后于 5.03 秒返回 `Timeout`；本机缓存环境基线通过。
- [x] 绿灯：把流式输出夹具改为条件协调，并统一使用 `PROCESS_TREE_TEST_TIMEOUT`。
- [x] 连续运行 `codex_process` 全部 9 项专项 3 次，三轮均为 9/9 通过。
- [x] 远端 Run `30873762688` 证明 core 修复有效，但暴露 release-console `local_verification` 的独立
  10 秒固定等待缺陷；其余 6 项同套件测试通过。
- [x] 红灯复核第二切片：Run `30873762688` 返回 `Elapsed(())`；本机热缓存基线 1/1 通过。
- [x] 绿灯第二切片：把 release-console 流式夹具改为释放标记、自截止和共享 30 秒测试预算。
- [x] 基于第二切片重新运行 Rustfmt、Clippy 和完整项目门禁。
- [x] 扩展 Windows 冷 Runner 测试规范和发布控制台本地门禁契约。
- [x] 重新完成 Trellis 全范围检查、差异检查、路径隔离和秘密扫描。
- [x] 监控 Run `30880121507` 到 success，并审计真实 `v0.5.0` Draft 的身份、资产、manifest 和签名。
- [x] 用真实 Draft 复现 tag 404 与发布说明行尾差异，各完成一个红色测试、最小修复和绿色回归。
- [x] 用修复后的生产 `SystemGhBackend` 对真实 Draft 完成一次完整审计，随后删除临时联网探针。
- [x] 完成新增审计修复的全范围检查。
- [ ] 精确提交并普通 push，验证 `origin/main` 与本地 `HEAD` 一致。
- [ ] 构建更新后的发布控制台，在精确身份守卫下重建当前 Draft，并通过控制台按钮触发、监控唯一的
  最终候选 Run；成功后再次审计 `v0.5.0` Draft，不公开。
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
- 工作提交 `86c0d92` 已普通 push，远端、本地与跟踪分支当时精确一致。Run `30873762688` 的
  Step #7 在 11 分 15 秒后失败：core 249 项通过；唯一失败为
  `process_backend_persists_safe_output_before_the_command_completes` 在
  `local_verification.rs:357` 返回 `Elapsed(())`，Step #8 跳过且没有 Draft。
- 第二切片首次编译因把 `OsString` 传给 `Vec<String>` 而退出 1；按公开 DTO 在测试边界转换路径后，
  专项 1/1 通过。最终专项连续 3 轮均为 1/1，通过时间 0.60、0.57、0.64 秒。
- 完整 release-console `local_verification` 集成套件为 7 passed、0 failed、1 个既有完整项目探针
  ignored，用时 11.71 秒。
- 第二切片 Rustfmt 与 workspace Clippy `-D warnings` 退出 0；成对安全 Relay 覆盖下最新
  `npm run check` 退出 0，用时 566.6 秒。主前端 60/338、core 249、路径安全 3、Provider 工作流 1、
  release-console `local_verification` 7 passed/1 ignored 及其余 release-console 套件通过。
- 第二轮完整门禁的 `codex-relay-second-final-check-*` 路径位于系统 `%TEMP%`，命令结束后已不存在。
  最新 `git diff --check`、任务校验和高置信度秘密扫描通过。
- Run `30880121507` 在 19 分 18 秒后以 success 结束，Step #7 与 Step #8 均为 success；Run head SHA
  为 `ebf0d69e1ab542f98320225b0c561ff39e781738`。
- Draft `364638762` 保持 `draft=true`、`prerelease=false`、未公开，目标提交为 `ebf0d69...`。
  NSIS 为 4,691,499 字节/SHA-256 `109656b5...c5c5f`，`.sig` 为 424 字节/
  `5876e60c...f78a4`，`latest.json` 为 2,454 字节/`4b2546e4...a0858`；实际下载摘要与 GitHub digest 一致。
- `latest.json.version=0.5.0`，两个 Windows 平台均指向安装器资产 ID `500939747`，内联签名与
  `.sig` 逐字节一致，说明与 Release 正文仅存在 LF/CRLF 和末尾换行传输差异。
- 真实 `gh api repos/.../git/ref/tags/v0.5.0` 返回 404，`git ls-remote` 也没有该 ref；Draft API
  同时返回精确 `target_commitish=ebf0d69...`，证明失败点是审计时点假设而非候选身份缺失。
- `draft_audit_uses_target_commitish_before_github_creates_the_tag_ref` 先以 `BackendFailed` 红灯，再转绿；
  `draft_audit_accepts_github_line_endings_without_accepting_note_drift` 先以 `DraftAuditFailed` 红灯，再转绿。
- `github_release` 集成套件最终 20/20 通过；修复后的生产后端真实 Draft 审计 1/1 通过，并记录上述
  Release ID、候选 SHA 与三项资产摘要。联网探针和下载目录均已删除。
- 成对安全 Relay 覆盖下新增审计修复的完整 `npm run check` 退出 0，用时 671.6 秒：Trellis 8 项、
  主前端 60/338、core 249、路径安全 3、Provider 工作流 1、release-console `github_release` 20/20、
  `local_verification` 7 passed/1 ignored 及其余套件通过；`codex-relay-draft-audit-final-check-*` 已删除。

## 验证命令

```powershell
$safeRoot = Join-Path $env:TEMP ('codex-relay-check-' + [guid]::NewGuid().ToString('N'))
$env:CODEX_RELAY_CODEX_HOME = Join-Path $safeRoot 'codex-home'
$env:CODEX_RELAY_APP_DATA_DIR = Join-Path $safeRoot 'app-data'
npm run test:rust:lib -- codex_process::tests -- --test-threads=1
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test github_release
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
- `tools/release-console/src-tauri/tests/local_verification.rs`：第二切片只修改 Windows 进程测试夹具和
  测试级预算，不改变生产 `LOCAL_COMMAND_TIMEOUT` 或日志实现。
- `tools/release-console/src-tauri/src/services/github_release.rs`：Draft 只使用精确 `target_commitish`，
  公开后仍校验 tag ref；文本规范化不得扩展为内部空白折叠。
- GitHub Draft：仅在 Run 全绿后由现有 workflow 创建；审计失败时保持 Draft，不公开、不改 Tag。
- Draft 重建：只允许删除本任务已核对的未公开 Draft ID，删除前再次验证 tag、目标 SHA 与
  `draft=true`，随后由更新后的控制台创建唯一新候选；身份任一漂移立即停止。
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
- 第一次扩展只扫描 core 文件，遗漏了 release-console 对同一 runner 的集成测试；第二轮已把搜索
  边界扩大到 `tools/release-console/src-tauri`，并用 Run `30873762688` 记录该传播失败。
- 生产 `SafeProcessRunner`、发布 workflow 和远端监控无需改变；问题位于测试夹具层，而非架构层。

### 5. 知识沉淀

- 已更新 `.trellis/spec/testing/rust-build-feedback.md`；现有索引已覆盖该文件，无需新增索引项。
- 仓库不存在 `src/templates/markdown/spec/`，没有可同步的规范模板。

### 6. Draft 审计平台状态与文本传输复盘

#### 1. 根因类别

- **B：跨层契约**：控制台没有区分 GitHub Draft 的预定 tag 名称与公开后 Git ref 的生命周期。
- **D：测试覆盖缺口**：夹具同时伪造 Draft tag 已存在、API 正文与本地说明字节完全相同。
- **E：隐式假设**：把 GitHub/Windows Actions 的多行文本传输当成不改变行尾和末尾换行。

#### 2. 修复为何此前未暴露

前两轮候选 Run 都在 Step #7 失败，未进入首个真实 Draft 审计；本地测试又完全由同一字符串构造
Release、manifest 和 expected notes，并为所有阶段返回 `GetTag` 成功，因此不能暴露平台时点和文本
规范化差异。只修复远端测试时序不能证明按钮完成，因为审计边界从未被真实数据执行。

#### 3. 预防机制

| 优先级 | 机制 | 具体行动 | 状态 |
|---|---|---|---|
| P0 | 测试覆盖 | Draft tag 404 与 CRLF/末尾换行分别建立先红后绿回归 | 已完成 |
| P0 | 架构边界 | Draft 用 `target_commitish`，公开复核才读取 tag ref | 已完成 |
| P0 | 真实集成 | 候选验收用生产后端审计一次真实 Draft，不保留默认联网测试 | 已完成 |
| P1 | 长期规范 | 发布规范新增状态时点、文本规范化、矩阵和反例 | 已完成 |

#### 4. 系统性扩展

- Draft 与公开 Release 共用审计函数时，任何只在公开后成立的证据都必须由 lookup 状态显式分支。
- 外部系统返回的 Markdown 应比较语义稳定的传输规范形态，但不得用全局 trim/split-whitespace 掩盖正文漂移。
- “workflow success”与“控制台按钮完成”是两个边界；最终验收必须包含生产后端 Draft 审计和 UI 会话终态。

#### 5. 知识沉淀

- 已更新 `.trellis/spec/release/publishing.md` 和发布规范质量检查索引。
- 仓库仍不存在 `src/templates/markdown/spec/`，无需模板同步。
