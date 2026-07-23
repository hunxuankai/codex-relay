# 技术设计

## 总体方案

本阶段在不改变运行时业务架构的前提下，从开发入口、Cargo feature 图和验证命令三个边界消除
可避免的编译工作。

```text
安全开发入口
  prepare-dev-data.ps1
    ├─ dev:safe          → tauri dev（保留现状）
    └─ dev:safe:no-watch → tauri dev --no-watch

快速 Rust 测试
  npm pre-script
    → check-rust-dev-environment.ps1
       ├─ 发现 tauri dev watcher → 在 Cargo 前失败
       └─ 无 watcher             → cargo test + 固定 src-tauri/target

依赖收敛
  reqwest(rustls-no-provider) + rustls(ring)
    → 不再构建 aws-lc-sys
    → 保持 platform verifier / JSON / HTTP2 / charset / system-proxy
```

## 开发进程门禁

新增 `scripts/check-rust-dev-environment.ps1`。默认通过 CIM 查询 `node.exe`，只把命令行同时包含
Tauri CLI 与 `dev`、且不包含 `--no-watch` 的进程判定为冲突。脚本提供仅供自动化测试的
`-ObservedCommandLine` 参数；传入时不查询真实进程。

门禁不终止进程、不修改环境，也不扫描用户文件。冲突返回稳定非零退出码和中文行动建议；
无冲突返回 0。npm 使用 `pretest:rust:*` 生命周期在 Cargo 命令前调用该脚本。

快速测试命令显式传入：

```text
--manifest-path src-tauri/Cargo.toml
--target-dir src-tauri/target
```

这样即使调用终端残留 `CARGO_TARGET_DIR`，项目入口仍复用唯一默认缓存。需要特殊备用 target 的
诊断仍可直接调用 Cargo，但不作为标准开发流程。

## 安全无 watcher 入口

扩展 `scripts/prepare-dev-data.ps1`，增加 `-NoRustWatch`。脚本仍先创建 `dev-data/codex` 与
`dev-data/app-data`、写入明确假密钥并设置两个 Relay 覆盖；最后根据参数调用：

- 默认：`npm.cmd run dev`
- 无 watcher：`npm.cmd run dev -- --no-watch`

为了不在测试中启动 Tauri，脚本增加可注入的 npm command 路径或 dry-run/参数观察边界；测试只
验证参数和安全环境，不启动 Node、Cargo、应用或真实网络。

## TLS feature 收敛

`reqwest` 改为：

```toml
reqwest = { version = "0.13.4", default-features = false, features = [
  "charset",
  "http2",
  "json",
  "rustls-no-provider",
  "system-proxy",
] }
rustls = { version = "0.23.42", default-features = false, features = [
  "ring",
  "std",
  "tls12",
] }
```

保留 reqwest 默认集合中除 `default-tls` 外的能力，使用 `rustls-no-provider` 避免自动选择
`aws-lc`，再由直接 `rustls` 依赖明确提供 `ring`。这避免依赖 Tauri updater 的 feature 合并
偶然保证 TLS provider。

新增构建图验证脚本，使用 `cargo tree` 检查：

- 不存在 `aws-lc-sys`；
- `rustls` feature 图包含 `ring`；
- 检查失败时不继续完整 Rust 门禁。

## 兼容性与回滚

- `reqwest::Client`、显式 `Proxy`、`.no_proxy()`、redirect/timeout 和 JSON API 不改代码。
- 保留 `charset`、`http2`、`system-proxy`，避免无关行为漂移。
- 若 provider HTTP/gateway 测试、Clippy、完整 Rust 测试或构建任一失败，回滚 TLS feature 收敛，
  保留已独立通过的开发入口优化。
- 若构建图仍含 `aws-lc-sys`，通过 `cargo tree -i aws-lc-sys -e features` 定位新的引入者，不能以
  文本删除锁文件条目冒充优化成功。

## 基准方法

- 缓存命中：固定默认 target 运行 `cargo test --lib --no-run`。
- 根目标反馈：只在安全、不会触发用户 watcher 的隔离方式下强制根测试目标重建，记录编译与
  链接总时间。
- 冷构建：在已验证的 `%TEMP%` 子目录使用独立 `CARGO_TARGET_DIR`，记录完整命令和是否超时，
  完成后校验绝对路径并删除。
- 基准不读取真实 Codex/Relay 数据；只编译和运行现有临时目录测试。

## core crate 决策门禁

本任务结束时根据实测判断：若典型 Provider 行为切片仍稳定超过 20 秒，建议另建任务把不直接
依赖 Tauri 的 models/services/infrastructure 移入 core crate。该迁移涉及模块可见性、依赖归属、
Tauri app 组合和 155 个测试，必须单独设计和回滚，不能混入本阶段。
