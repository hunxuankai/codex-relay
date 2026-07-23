# 拆分 codex-relay-core 缩短 Provider 编译反馈

## 目标

在保持 Codex Relay 现有产品行为、安全边界和完整质量门禁不变的前提下，研究并建立合理的
Cargo workspace/crate 边界，把典型 Provider 行为切片所需的非 Tauri Rust 代码迁入
`codex-relay-core`，使 Provider 专项测试能够只编译和链接 core crate，并将稳定反馈时间降到
20 秒以内。

## 背景与已确认事实

- 上一阶段已提供安全的无 Rust watcher 开发入口、固定默认 target 的快速测试入口和 watcher
  冲突门禁；本任务不得绕过或削弱这些入口。
- Rustls 已统一使用 `ring`，依赖图已移除 `aws-lc-sys`；本任务不得改变 Provider TLS、代理、
  gateway 或 updater 的产品契约。
- 上一阶段新鲜基准为：默认 target 缓存命中 4.12 秒、根 crate 重建 26.81 秒、独立 target
  冷构建 422.67 秒且产物 2.68 GiB、完整 `npm run check` 352 秒。
- 完整 Rust lib 当前包含 172 项测试；完整 `cargo test` 为生成全部目标曾额外编译和链接约
  3 分钟。测试执行本身不是主要瓶颈，当前单体根 crate 的编译和链接范围才是本任务要缩小的对象。
- 上一阶段明确把 crate 拆分留给独立任务；提交 `15b39a3`、规范提交 `635c6ca` 和归档任务
  `07-23-rust-build-feedback-optimization` 是本任务的输入证据。

## 需求

### Cargo 边界与迁移范围

- 先依据当前依赖图、模块耦合、测试归属和编译热点研究边界，不预设最终目录结构或一次性迁移
  所有 Rust 模块。
- 优先把不依赖 Tauri 运行时的 Provider、模型、服务和基础设施逻辑迁入
  `codex-relay-core`；Tauri 应用 crate 只保留命令适配、应用状态/生命周期和平台集成等边界。
- core crate 必须拥有可独立运行的 Provider 行为测试，典型专项入口不得为了复用测试而重新
  链接 Tauri 应用 crate。
- 迁移后依赖方向必须单向：Tauri 应用 crate 可以依赖 core，core 不得依赖 Tauri。

### 行为与兼容性

- 保持现有 Tauri command 名称、参数、返回 DTO、序列化形态和错误码不变。
- 保持 Provider API、Codex gateway、显式 Relay 代理、重定向、超时、SSE、TLS provider 和
  错误分类行为不变。
- 保持受管配置事务的锁、指纹、备份、临时文件、解析、原子替换、写后验证和可验证回滚不变；
  `config.toml` 仍须通过 `toml_edit` 局部修改并保留未知内容。
- 保持路径解析、路径包含关系、符号链接/重解析点和真实用户目录保护不变。

### 安全与测试隔离

- 开发、自动化测试和基准不得读取、写入或删除真实 `%USERPROFILE%\.codex` 与
  `%LOCALAPPDATA%\CodexRelay`。
- fixture 中的密钥只能使用明确的 `test-key-*-not-real`；认证文件、Authorization header 和
  真实密钥不得进入 Git、日志、快照、普通前端状态、Trellis 材料或测试输出。
- 不清理默认 `src-tauri/target`。独立 target 仅用于受控冷构建基准，必须位于已验证的临时
  目录，且只在再次校验绝对路径后清理。

### 实施与证据

- 使用 Trellis `tdd` inline 工作流按行为切片实施，每个切片保留红测、绿测和必要重构证据；
  不派发子 Agent，不建立第二套规划、TDD 或分支收尾流程。
- 记录迁移前后可比较的 Provider 专项测试、根 crate 重建、core crate 重建/链接、独立冷构建
  和完整质量门禁耗时；记录命令、缓存状态、目标目录和并发 watcher 状态。
- 如拆分后的典型 Provider 专项反馈仍不能稳定低于 20 秒，必须保留真实结果和原因，不能通过
  缩减行为覆盖、跳过链接或削弱门禁制造提速。
- 完成后运行完整 `npm run check`、前端构建、安全审计以及优化前后基准；按 Trellis 流程更新
  规范、提交、归档和记录会话，但不得 push。

## 可观察行为切片

1. 运行 core crate 的 Provider 专项测试时，Cargo 只构建/链接 core 及其依赖，不构建 Tauri
   应用 crate，且覆盖迁移前同一 Provider 行为与错误分类。
2. Tauri command 通过薄适配层调用 core 后，前端可观察的 command 名称、DTO、错误码和副作用
   与拆分前一致。
3. Provider HTTP/gateway、代理、TLS、事务和路径安全测试在安全临时目录中保持通过，且测试证据
   不包含真实用户路径或密钥。
4. 使用固定默认 target 的典型 Provider 红绿循环在重复的新鲜测量中稳定低于 20 秒；完整门禁
   仍覆盖应用 crate、core crate、Clippy、集成测试与前端。

## 验收标准

- [x] Cargo workspace 和 `codex-relay-core` 边界由当前依赖/测试证据支持，core 不依赖 Tauri。
- [x] 目标 Provider、模型、服务和基础设施逻辑已迁入 core，Tauri command/DTO/错误码保持兼容。
- [x] Provider 专项测试可只针对 core crate 编译和链接，并覆盖迁移前的关键行为。
- [x] 典型 Provider 专项反馈经重复测量稳定低于 20 秒；若未达标，结果、原因和后续建议如实记录。
- [x] Provider API/gateway、代理、TLS、事务与路径安全行为均有新鲜回归证据。
- [x] 完整 `npm run check`、前端构建和安全审计通过，且未削弱任何既有门禁。
- [x] 基准不清理默认 target；受控冷构建临时 target 经路径验证后已清理。
- [x] 工作区不含真实密钥、认证文件、受保护路径数据或未清理的临时构建产物。
- [ ] 相关长期规则已更新到 `.trellis/spec/`，改动已提交、任务已归档、会话已记录且未 push。

## 范围外

- 不改变面向用户的功能、Provider 协议、配置格式、更新/卸载行为或 Windows 系统状态。
- 不以安装全局编译缓存工具、修改系统级编译器/杀毒配置、删除默认 target 或削弱测试覆盖作为
  本任务的优化手段。
- 不为了追求单一数字把与 Provider 行为无关且仍依赖 Tauri 的模块强行迁入 core。
