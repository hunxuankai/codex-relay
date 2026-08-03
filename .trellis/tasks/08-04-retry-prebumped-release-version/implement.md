# 修复已预升版本发布重试实施计划

## 当前进度

- [x] 复核当前仓库、线上 Latest、失败会话、候选提交和远端 Run 根因。
- [x] 收敛 PRD 与技术设计，确认公开版本、仓库版本和目标版本必须分离。
- [x] 红灯 1A：增加已预升候选计划测试，确认缺少显式公开版本入口。
- [x] 绿灯 1A：实现显式公开版本候选计划，六文件幂等应用测试通过。
- [x] 红灯 1B：增加非法版本关系和 Latest tag 解析测试。
- [x] 绿灯 1B：实现版本方向校验与公开版本基准解析。
- [x] 红灯 2：增加部分变化提交、零变化复用 HEAD、HEAD 漂移和日志真实性测试。
- [x] 绿灯 2：实现真实变化集合提交、已同步 HEAD 复用、精确远端确认和准确进度证据。
- [x] 重构与专项检查：Rustfmt、Clippy、release-console Rust 全套、类型检查和 17/89 前端测试通过。
- [x] 全范围检查：成对安全 Relay 临时覆盖下 `npm run check` 退出 0。
- [x] 构建发布控制台；源 EXE 与便携副本均为 12,981,248 字节，SHA-256 为
  `165D58BBFF52357344C1628AB509348AC6D1CA9DFBB8CFDF0DFB39E98974BD5A`，新版进程已从默认交付路径启动。
- [x] 更新发布规范，记录 Latest 基准、幂等候选与 HEAD 复用契约。
- [x] 完成差异、安全、忽略项和秘密扫描。
- [ ] 精确提交工作改动，归档任务、记录会话日志并普通 push 到已配置上游。

## 验证证据

- TDD 红灯分别确认：缺少显式公开版本计划入口；仓库高于目标仍会生成降级计划；部分变化被固定
  六文件 Git 集合拒绝；零变化进入空提交失败；零变化日志错误声称创建提交；公开门禁错误优先级错误。
- 直接相关 Rust 套件：`release_candidate` 14/14、`git_release` 25/25、`release_orchestrator` 15/15
  通过；HEAD 漂移、部分变化、零变化与事务 finalize 均使用临时 Git 仓库验证。
- release-console Rust 全套退出 0；库 35 项及各集成套件通过，1 个生产后端完整项目探针按既有设计 ignored。
- release-console TypeScript 检查退出 0，前端 17 个文件、89 项测试通过；Clippy `-D warnings` 退出 0。
- 成对安全 Relay 临时覆盖下 `npm run check` 退出 0：Trellis 8 项、主前端 60/338、发布控制台
  17/89、主库 Rust 249 项、路径安全 3 项、Provider 工作流 1 项及 release-console Rust 套件通过；
  同一慢速探针 ignored。
- `npm run build:release-console` 退出 0。源 EXE 与默认便携副本均为 12,981,248 字节，SHA-256
  均为 `165D58BBFF52357344C1628AB509348AC6D1CA9DFBB8CFDF0DFB39E98974BD5A`；新版 PID 32772 已启动。
- Windows UI 自动化连接不可用，因此没有声称人工点击“生成发布计划”成功；没有触发真实 workflow、
  Draft、Tag、Release、签名、安装、升级或公开操作。
- `git diff --check` 退出 0；构建产物由既有 ignore 规则排除；高置信度密钥扫描无命中。

## 行为切片

1. 公开 Latest=`v0.4.0`、仓库版本=`0.5.0`、目标=`0.5.0` -> 计划成功，公开起点为 `0.4.0`。
2. 仓库版本高于目标或 Latest tag 非法 -> 稳定错误，六文件保持原字节。
3. 计划只有发布说明变化 -> Git 只提交发布说明，任何额外工作区状态都拒绝。
4. 计划没有文件变化 -> 不创建空提交，复用经过本地/远端复核的 HEAD 并走现有精确 Push。

## 验证命令

```powershell
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test release_candidate
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test git_release
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml --test release_orchestrator
cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml latest_release
cargo fmt --all --check --manifest-path tools/release-console/src-tauri/Cargo.toml
npm run typecheck:release-console
npm run test:release-console
$safeRoot = Join-Path $env:TEMP ('codex-relay-check-' + [guid]::NewGuid().ToString('N'))
$env:CODEX_RELAY_CODEX_HOME = Join-Path $safeRoot 'codex-home'
$env:CODEX_RELAY_APP_DATA_DIR = Join-Path $safeRoot 'app-data'
npm run check
npm run build:release-console
git diff --check
```

完整检查必须在同一进程成对设置两个 Relay 覆盖。测试和构建不触发真实 workflow、Draft、Tag、
Release、签名、安装或升级。

## 风险文件与回滚点

- `release_application.rs`：公开 Latest 到计划基准的数据流；回滚点为既有本地版本基准。
- `release_candidate.rs`：固定六文件事务与版本方向；不得削弱指纹、备份和回滚。
- `git_release.rs`：实际变化集合、HEAD/远端竞态和空提交复用；任何专项失败先回到该切片分析。
- `release_orchestrator.rs`：只调整候选提交结果的准确日志，不改变会话恢复阶段。
- `.trellis/spec/release/publishing.md`：记录可执行发布契约，不写入本次未执行的远端发布声明。
