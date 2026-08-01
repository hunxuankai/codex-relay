# 发布控制台本地门禁失败诊断实施计划

> **执行约束：** 使用 `trellis-before-dev` 与 `superpowers:test-driven-development`，按 Codex inline 执行；不得派发写入或检查子 Agent。

**Goal:** 消除发布结构测试对 PowerShell 中文编码的依赖，并让控制台显示具体本地门禁命令与安全退出状态。

**Architecture:** 验证脚本提供 ASCII 稳定码；Rust 在 local verification service、release orchestrator 和 application event 间传递命令 ID 与可选退出码，不传播原始子进程输出。

**Tech Stack:** PowerShell 7、Vitest、Tauri 2、Rust 2024、Tokio。

## 文件职责映射

- `src/release-request.test.ts`：稳定错误码和 workflow output 回归测试。
- `scripts/validate-release-request.ps1`：秘密检测的 ASCII 错误码契约。
- `tools/release-console/src-tauri/tests/local_verification.rs`：本地 service 字段与过滤环境真实路径回归。
- `tools/release-console/src-tauri/src/services/local_verification.rs`：命令 ID 与可选退出码建模。
- `tools/release-console/src-tauri/tests/release_orchestrator.rs`：回滚后错误证据保留。
- `tools/release-console/src-tauri/src/services/release_orchestrator.rs`：错误字段传播和安全事件投影。
- `tools/release-console/src-tauri/src/services/release_application.rs`：用完整编排错误生成 `StepFailed`。
- `.trellis/spec/release/publishing.md`：记录编码无关门禁和失败证据契约。

## 行为切片 1：稳定秘密检测错误码

- [x] 修改 `src/release-request.test.ts`，断言 `RELEASE_NOTES_SECRET_DETECTED` 且 output 文件不存在。
- [x] 运行：

  ```powershell
  npm exec -- vitest run src/release-request.test.ts
  ```

  预期：因脚本尚未输出稳定码而失败。
- [x] 最小修改验证脚本，在现有中文消息前加入稳定 ASCII 码。
- [x] 重跑目标 Vitest，确认绿色。

RED 证据：目标 Vitest 退出 1，唯一失败为缺少 `RELEASE_NOTES_SECRET_DETECTED`。GREEN 证据：同一文件 1/1 测试通过。

## 行为切片 2：本地 service 保留退出状态

- [x] 扩展 `local_verification.rs` 测试：非零命令要求 `Some(1)`；backend 失败要求命令 ID 与 `None`。
- [x] 运行：

  ```powershell
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test local_verification -- --nocapture
  ```

  预期：因错误类型没有 `exit_code` 字段而编译失败。
- [x] 扩展 `LocalVerificationError::CommandFailed` 并做最小映射。
- [x] 重跑专项测试至绿色。

RED 证据：Rust 集成测试因 `CommandFailed` 缺少 `exit_code` 出现两处 E0026 并退出 1。GREEN 证据：`local_verification` 5/5 测试通过。

## 行为切片 3：编排层与事件保留具体步骤

- [x] 修改 orchestrator 测试，要求本地失败在回滚后保留 `release-structure-tests` 与可选退出码。
- [x] 修改 application 内部测试，要求事件使用具体 `stepId`、稳定码和候选已回滚文案。
- [x] 运行相关 Rust 测试，确认旧通用错误/事件导致 RED。
- [x] 扩展 `ReleaseOrchestratorError::LocalVerificationFailed`，增加安全 `step_id` / `message` 投影，并让 application 用完整错误完成收尾。
- [x] 重跑 orchestrator、application 专项测试至绿色；完整 release-console Rust 套件留在质量检查阶段执行。

RED 证据：orchestrator 因枚举缺少 `command_id/exit_code` 退出 1；application 因缺少字段和 `finish_with_orchestrator_error` 出现 6 个编译错误。质量审查又发现“未能启动”会虚构 backend failure 类别，修改测试后按预期失败。GREEN 证据：回滚传播测试 1/1、应用有/无退出码安全事件测试通过。

## 行为切片 4：过滤环境回归

- [x] 在 `tests/local_verification.rs` 通过实际 `ProcessLocalVerificationBackend`、当前工具链路径和过滤环境执行固定发布结构测试；不输出 stdout/stderr。
- [x] 运行该测试，确认稳定错误码修复后退出 0。
- [x] 保留测试为永久回归，确保未来一键发布的实际进程边界不会再次因编码契约失败。

回归证据：生产过滤环境通过真实 npm shim 执行生产定义的 `release-structure-tests`，目标测试 1/1 通过，进程退出码为 0；修复前同一路径已稳定复现退出 1。

## 规范、完整验证与交付

- [x] 更新 `.trellis/spec/release/publishing.md`，记录 ASCII 稳定码、可选退出码传播、具体事件与过滤环境回归契约。
- [x] 运行专项验证：

  ```powershell
  npm exec -- vitest run src/release-request.test.ts
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test local_verification --test release_orchestrator
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console services::release_application
  ```

- [x] 运行格式、Clippy、发布控制台测试和项目完整门禁：

  ```powershell
  cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
  cargo clippy --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console --all-targets --all-features -- -D warnings
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml
  npm run check
  ```

- [x] 重新打包：

  ```powershell
  npm run build:release-console
  ```

- [x] 枚举 `dist/release-console/CodexRelayReleaseConsole.exe` 的完整路径、大小、最后写入时间和 SHA-256，并核对源/交付 EXE 哈希。
- [x] 运行 `git diff --check`、状态、差异与秘密/真实路径审计；精确暂存本任务文件并提交简体中文 Conventional Commit。

### 本轮验证与打包证据

- `npm exec -- vitest run src/release-request.test.ts`：1/1 通过。
- `cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test local_verification --test release_orchestrator`：6/6 + 11/11 通过；过滤环境真实 npm/Vitest 回归退出 0。
- `cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console services::release_application`：11/11 通过。
- `cargo clippy --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console --all-targets --all-features -- -D warnings`：退出 0。
- `cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml`：104 项通过。
- `npm run test:release-console`：16 个测试文件、61 项测试通过。
- `npm run check`：退出 0，耗时 544.8 秒；Trellis 8 项、主前端 59 个文件/309 项、发布控制台 16 个文件/61 项及整个 Rust workspace 通过。
- `npm run build:release-console`：退出 0；Vite 记录两条第三方 `@vueuse/core` PURE 注释位置警告并自动移除注释，不影响构建结果。
- 源与交付 EXE：`12,648,448` 字节，最后写入时间 `2026-08-02T01:02:09.5613446+08:00`，SHA-256 均为 `E0006F09310803A4346BD3D9B82FF16A7A2B8C086A5CB423244529BA7FDCDAC1`。
- `git diff --check`：退出 0；高置信度秘密前缀、非测试 Authorization/Bearer、真实用户/工作区绝对路径和受管认证文件跟踪扫描均无命中。
- `git status --short --ignored`：仅显示本任务改动与既有忽略目录；扫描既有 `node_modules/.pnpm` 时 Git 报告若干已缺失链接目录警告，但普通状态、完整检查和构建均退出 0。
- 临时 detached 诊断 worktree 已安全移除并 prune，只保留主工作树。
- 本轮未执行签名、安装、UAC、应用内升级或卸载，不作相关成功声明。

## Mock 边界

- service 和 orchestrator 使用确定性 fake backend，不 mock 错误类型内部实现。
- 过滤环境回归使用真实本地 npm/Vitest 进程，但不访问网络、不读写真实 Codex 配置或 Relay 应用数据。
- application 事件测试使用真实 `ReleaseStateStore` 临时目录与内存事件 sink。

## 回滚点

- 稳定错误码与 Rust 错误字段可独立回滚；二者不改变发布 session schema。
- 候选文件回滚仍由现有 `ReleaseCandidateTransaction` 负责，失败时不得继续 commit 或 push。
