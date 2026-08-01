# 发布门禁进程收尾与失败进度持久化实施计划

> **执行约束：** 使用 `trellis-before-dev`、`superpowers:test-driven-development`、
> `vue-best-practices` 和 `trellis-check`，按 Codex inline 执行；不得派发写入或检查子 Agent。

**Goal:** 让成功的本地完整检查通过安全进程收尾返回真实退出码，并让真实失败原因与失败进度跨重启保留。

**Architecture:** 修正共享 `SafeProcessRunner` 的主进程退出后清理语义；在本地门禁 service、
orchestrator 和 application 间传递类型化失败；通过现有 `ReleaseStateStore` 原子持久化紧凑失败检查点；
Vue 时间线从后端 DTO 投影终态。

**Tech Stack:** Rust 2024、Windows Job Object、Tokio、Serde、Tauri 2、Vue 3 Composition API、
TypeScript、Vitest、Vue Test Utils。

## 文件职责映射

- `src-tauri/crates/codex-relay-core/src/infrastructure/codex_process.rs`：安全终止可清理后代并保留主退出码。
- `tools/release-console/src-tauri/src/infrastructure/local_verification.rs`：`ProcessError` 到 service 分类映射。
- `tools/release-console/src-tauri/src/services/local_verification.rs`：类型化本地门禁失败契约。
- `tools/release-console/src-tauri/src/services/release_orchestrator.rs`：回滚后传播失败并写入具体检查点。
- `tools/release-console/src-tauri/src/services/release_application.rs`：通用失败兜底与安全消息投影。
- `tools/release-console/src-tauri/src/services/release_state.rs`：原子 `fail` 操作和会话不变量。
- `tools/release-console/src-tauri/src/models.rs`：`ReleaseFailureEvidence` DTO。
- `tools/release-console/src/types/release.ts`：对应 TypeScript DTO。
- `tools/release-console/src/components/release/ReleaseTimeline.vue`：失败状态 reducer 和可见映射。
- Rust/Vitest 对应测试：每个公开行为先 RED 后 GREEN。
- `.trellis/spec/release/publishing.md`：更新进程收尾、错误分类和失败检查点契约。

## 行为切片 1：成功主进程的安全后代清理

- [x] 在 core runner 测试中构造父进程退出 0、后代超过宽限的场景，先断言当前实现错误返回
  `ProcessTreeTermination`（RED）。
- [x] 最小修改主进程退出后的分支，复用 `terminate_job_and_wait`；完整验证成功后返回原退出码。
- [x] 保留无法终止 fake Job、超时、取消、输出上限和后代取消测试为 GREEN。
- [x] 在 `tests/local_verification.rs` 增加真实 `ProcessLocalVerificationBackend` 回归，证明 service 收到
  成功 evidence，而不是无退出码失败。

目标命令：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core infrastructure::codex_process
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test local_verification
```

## 行为切片 2：类型化本地门禁失败

- [x] 先修改 service 测试，要求非零退出使用 `ExitCode(1)`、backend 进程失败保留具体分类（RED）。
- [x] 新增 service 级 `LocalVerificationProcessError` / `LocalVerificationFailure`，基础设施完成穷尽映射。
- [x] 修改 orchestrator 测试，要求候选回滚后同一分类仍存在且后续 commit/push 未执行（RED→GREEN）。
- [x] 修改 application 测试，覆盖退出码、超时、输出过大、进程树终止等安全消息；不得出现 stdout、
  stderr 或 fixture secret。

目标命令：

```powershell
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test local_verification --test release_orchestrator
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console services::release_application
```

## 行为切片 3：原子失败检查点

- [x] 在 `release_state.rs` 测试中先要求 `ReleaseStateStore::fail` 保存原阶段、stepId、code 与 failed
  终态（RED）。
- [x] 给 `ReleaseSession` 增加 `failure`，使用 serde default 保持 schema version 1 兼容。
- [x] 实现 `fail` 并替换现有失败路径的直接 `advance(..., Failed)`；一次保存完成终态和证据。
- [x] 增加旧 JSON 无 failure、非法非终态 failure、初始化新 session 清空 failure 的测试。
- [x] 更新 Rust/TypeScript fixture，保持 DTO 精确对应。

目标命令：

```powershell
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test release_state --test release_orchestrator
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console services::release_application
```

## 行为切片 4：跨重启失败时间线

- [x] 先扩展 `ReleaseTimeline.test.ts`：失败 DTO 且 events 为空时，失败前步骤已完成、当前步骤失败、
  后续未开始（RED）。
- [x] 在组件内建立共享 stepId/phase 映射和失败投影；增加 `failed` 文本、danger tag 与可见样式。
- [x] 补充实时 `stepFailed` 回退、旧 session 保守回退、新 session 清空后的正常状态测试。
- [x] 更新所有 `ReleaseSession` TypeScript fixture，避免用可选类型掩盖 DTO 漂移。

目标命令：

```powershell
npm exec -- vitest run tools/release-console/src/components/release/ReleaseTimeline.test.ts tools/release-console/src/composables/useReleaseSession.test.ts
npm run typecheck:release-console
```

## 行为切片 5：生产进程后端慢速探针

- [x] 增加默认 ignored 的真实 `full-project-check` 探针，使用安全临时 USERPROFILE/APPDATA/
  LOCALAPPDATA/NPM cache，不打印子进程输出。
- [x] 先构建探针测试二进制；再在 Cargo 外层进程已经退出后直接执行该测试二进制的 ignored case，
  避免嵌套 Cargo 锁和递归执行。
- [x] 记录真实耗时、退出状态和安全分类；只有退出 0 才证明用户路径已修复。

## 规范、完整验证与交付

- [x] 更新 `.trellis/spec/release/publishing.md`，替换旧 `Option<i32>` 契约，记录安全后代清理和
  session failure 规则。
- [x] 运行格式与专项检查：

  ```powershell
  cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
  cargo clippy --manifest-path tools/release-console/src-tauri/Cargo.toml -p codex-relay-release-console --all-targets --all-features -- -D warnings
  npm run test:release-console
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml
  ```

- [x] 使用成对安全临时 Relay 覆盖运行完整门禁：

  ```powershell
  npm run check
  ```

- [x] 重新打包发布控制台：

  ```powershell
  npm run build:release-console
  ```

- [x] 枚举交付 EXE 的完整路径、大小、最后写入时间和 SHA-256；不声明签名、安装或发布成功。
- [x] 运行 `git diff --check`、状态、差异和秘密/真实路径审计。
- [x] 精确暂存本任务文件并提交。
- [ ] 完成 Trellis 规范更新、任务归档和会话日志提交后，普通 push 当前跟踪分支，并验证远端分支
  与本地 `HEAD` 一致。

## 本轮验证证据（2026-08-02）

- runner 回归先观察到 `ProcessTreeTermination`（RED），最小修复后 core 进程专项 9/9 通过；
  `successful_parent_cleans_up_lingering_descendants_without_losing_exit_code` 返回 `Some(0)`，后代已退出。
- `npm run check:rust`：退出 0；依赖图、fmt、Clippy 和整个 Cargo workspace 通过。
- 生产 backend ignored 探针直接运行测试二进制：1/1 通过，耗时 261.73 秒；真实
  `full-project-check` 返回退出码 0，不再落入 backend failure。
- 最终安全临时环境 `npm run check`：退出 0，日志 67,706 字节；主前端 59 个测试文件、发布控制台
  16 个测试文件以及全部 Rust 套件通过。
- `npm run build:app --workspace @codex-relay/release-console`：退出 0，生成 Release EXE。默认
  `npm run build:release-console` 的编译阶段同样成功，但 canonical `dist/release-console` EXE 正由当前
  控制台运行而无法覆盖；未强制结束该进程。随后使用同一包装脚本输出到
  `dist/release-console-verified/CodexRelayReleaseConsole.exe` 成功。
- 验证产物：12,665,856 字节；最后写入时间 `2026-08-02T07:24:56.3039384+08:00`；SHA-256
  `7215CF09556463D08497E7409AE563D5352EDA64F59EE3DD09FAAEE3A63A49B9`。未执行签名、安装或实际发布。
- `git diff --check` 退出 0；新增秘密形态、真实用户路径和调试打印命中均为 0。

## Mock 与真实边界

- core runner 和 `ProcessLocalVerificationBackend` 的进程树行为使用真实 Windows 临时子进程，不 mock
  Job Object 的成功路径；无法终止分支允许 fake Job 控制器。
- service/orchestrator 使用确定性 fake backend 验证错误传播与回滚，不 mock 错误对象内部实现。
- state 使用临时 Git dir 和真实原子文件写入。
- Vue 测试只传 typed props/events，不启动 Tauri 或真实文件系统。
- 慢速 full-project 探针穿过生产 runner 和过滤环境，但不访问网络、不读取或写入真实 Codex/Relay
  数据目录，也不输出可能含秘密的原始日志。

## 回滚点

- 行为切片 1 可独立回滚共享 runner 收尾逻辑；回滚后不得保留声称成功的测试结果。
- 类型化错误与 failure DTO 可一起回滚，schema version 1 不需要数据迁移。
- 时间线投影可独立恢复旧逻辑；不得删除现有 session、候选事务备份或用户数据。
