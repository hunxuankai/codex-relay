# 缩短 Rust 开发编译反馈

## 目标

缩短 Codex Relay 日常 Rust 行为切片的反馈时间，优先消除可避免的双重编译、冷缓存和重复依赖，
同时保持安全开发路径、TLS 行为、完整质量门禁和现有发布构建不变。

## 已确认事实与范围

- 当前机器实测：默认 target 完全命中约 2 秒；新 target 冷编译超过 7 分 43 秒；仅触发根
  crate 增量重建约 16 秒；8 个专项测试本身只运行约 0.06 秒。
- `tauri dev` 会监听 Rust 源码并自动重编译；与手动 `cargo test` 并行时会构建两套目标并争用
  8 个逻辑处理器、16 GiB 内存和文件锁。
- `src-tauri/target/debug` 约 27 GiB，单个测试 PDB 约 180 MiB；过滤测试名称不会缩小当前单体
  `lib` 测试二进制的编译范围。
- 直接 `reqwest` 默认 feature 启用了 `aws-lc-sys`，而 `tauri-plugin-updater` 已启用 Rustls
  `ring` provider；本阶段只移除重复的 `aws-lc` 编译，不改变 Provider API、代理或 updater 产品契约。
- 本阶段实施高收益、低风险优化。是否拆分 `codex-relay-core` 由优化后的新鲜基准决定，不在本
  任务内直接进行跨 crate 架构迁移。

## 需求

### 安全的无 Rust watcher 开发入口

- 提供 `npm run dev:safe:no-watch`，复用 `scripts/prepare-dev-data.ps1` 创建的假配置、假密钥和
  成对 Relay 路径覆盖，再以 `tauri dev --no-watch` 启动应用。
- 现有 `npm run dev:safe` 行为保持不变；不得新增绕过安全路径覆盖的快捷入口。
- README 必须说明：Rust TDD 期间使用无 watcher 入口或只启动前端；需要观察最新 Rust 行为时
  主动重启应用。

### 快速 Rust 测试入口与冲突门禁

- 提供稳定的 `npm run test:rust:lib`、`test:rust:path-safety` 和
  `test:rust:provider-workflow`，显式复用仓库默认 `src-tauri/target`。
- 快速 Rust 测试开始前检测正在运行且未带 `--no-watch` 的 Tauri dev 进程；发现冲突时在启动
  Cargo 前失败，并给出 `dev:safe:no-watch` / `dev:frontend` 行动建议。
- 门禁必须可用合成命令行测试，不依赖真实启动或终止用户进程；无 watcher、`--no-watch` 和
  无 Tauri dev 三种情况均有可重复验证结果。
- 完整 `npm run check:rust` 仍是最终门禁；快速入口不能替代 Clippy、全部目标或集成测试。

### Rust TLS 依赖收敛

- 直接 `reqwest` 关闭默认 feature，保留当前需要的 JSON、HTTP/2、charset、系统代理能力和
  Rustls platform verifier，但改用无内置 provider 形式。
- 显式启用 Rustls `ring` provider，保证 Provider API 客户端在不依赖 updater 偶然传递 feature
  的情况下仍有稳定 TLS provider。
- 依赖图不得再包含 `aws-lc-sys`，并通过可重复的构建配置检查防止回归。
- Provider API 请求构造、显式 Relay 代理、重定向策略、超时、SSE/gateway 和 updater 构建行为
  不得改变。

### 测量与完成声明

- 记录优化后缓存命中、根 crate 重建和独立 target 冷构建的命令及耗时；若环境中存在并发
  Tauri dev，报告必须注明。
- 不以 `rust-lld`、仅关闭根 crate debug、`cargo-nextest` 或频繁清理 target 作为本阶段方案；
  本轮实验未证明它们能减少当前增量反馈时间。
- 若移除双编译和重复依赖后，典型 Provider Rust 行为切片仍稳定超过 20 秒，后续单独规划
  `codex-relay-core` 拆分，不在本任务中临时扩大范围。

## 可观察行为切片

1. 以合成的 `tauri.js dev` 命令行运行 Rust 开发门禁时，命令在调用 Cargo 前返回非零并显示
   无 watcher 建议；合成 `tauri.js dev --no-watch` 或空进程集合时返回成功。
2. 运行 `npm run dev:safe:no-watch` 时，安全开发脚本仍准备 `dev-data` 假数据和成对路径覆盖，
   传递给 Tauri CLI 的参数包含 `--no-watch`；测试不得启动真实应用或访问真实用户目录。
3. 依赖配置变更后，`cargo tree` 不再包含 `aws-lc-sys`，包含 Rustls `ring` provider；现有
   Provider HTTP/gateway 单元测试和 Rust 完整检查保持绿色。
4. 快速测试脚本使用固定 `src-tauri/target` 并只选择相应 Rust target；最终完整门禁仍覆盖 fmt、
   Clippy、170 个单元测试及两个集成测试目标。

## 验收标准

- [x] `npm run dev:safe:no-watch` 安全启动且禁用 Rust watcher；现有 `dev:safe` 不回归。
- [x] 快速 Rust 测试入口在 watcher 冲突时于 Cargo 启动前失败，在无冲突时使用固定默认 target。
- [x] 门禁脚本的三类合成进程场景均有自动化测试。
- [x] Cargo 依赖图不含 `aws-lc-sys`，Rustls `ring` provider 显式存在并有回归检查。
- [x] Provider HTTP、gateway、路径安全和 provider workflow 行为不变，测试不访问真实用户路径。
- [x] README 与测试规范记录新的开发入口、快速/完整门禁边界和禁止随机 target 的规则。
- [x] 记录优化后新鲜基准，并据此决定是否另建 core crate 拆分任务。
- [x] `npm run check` 通过；工作区不包含临时 target、真实密钥或受保护路径数据。

## 范围外

- 本任务不拆分 workspace/crate，不移动现有 Rust 模块或改变 Tauri command/API。
- 不修改生产网络请求、Provider 测试结果、配置文件格式、事务、备份或卸载行为。
- 不安装全局 `sccache`、`cargo-nextest`、LLVM 工具或修改 Windows Defender/系统级配置。
- 不删除默认 Cargo 缓存；临时基准 target 必须位于已验证的系统临时目录并在完成后清理。
