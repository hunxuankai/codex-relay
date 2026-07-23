# Rust 约定

## 模块与职责

- `src-tauri/crates/codex-relay-core/src/models/`：前后端共享语义、DTO 和事务/健康/设置数据。
- `src-tauri/crates/codex-relay-core/src/services/`：Provider、设置、事务、备份等平台无关业务规则。
- `src-tauri/crates/codex-relay-core/src/infrastructure/`：路径、原子文件、指纹、Provider 网络和
  Codex 安全运行边界，以及在 core 内完成的错误详情脱敏。
- `src-tauri/src/commands/`：Tauri 适配，不承载文件写入或业务流程。
- `src-tauri/src/services/` 与 `src-tauri/src/infrastructure/`：只保留自检、文件 watcher、开机启动、
  日志初始化/保留等桌面应用生命周期职责，并复用 core 模块。
- `src-tauri/src/app_state.rs`：共享 core/桌面服务、写入守卫和跨命令状态。

保持现有小模块职责，不为单个改动创建第二套路径、日志或事务实现。

`codex-relay-core` 不得依赖 `tauri` 或 `tauri-plugin-*`。Tauri 应用可以依赖并 re-export core；
Provider 快速测试必须直接选择 core package，完整检查则覆盖整个 Cargo workspace。

## 序列化契约

- Rust/前端 DTO 使用稳定字段和 camelCase 序列化。
- JSON 由 `serde_json` 生成，UTF-8、两个空格缩进、末尾换行。
- Provider Debug 输出必须脱敏；公开结构不得包含 API Key 字段。
- `config.toml` 必须使用 `toml_edit::DocumentMut` 局部编辑。

## 异步与并发

- 所有 `TransactionService` 克隆共享同一个 Tokio 异步互斥锁。
- 托盘忙状态用于交互防重，事务锁才是最终一致性边界。
- 不在持锁期间执行不必要的 UI、网络或长时间外部操作。

## 测试方式

- 纯转换和校验放在模块单元测试。
- 文件流程使用 `tempfile`、`AppPaths::for_test` 或成对 Relay 覆盖。
- 写入失败通过可替换 `FileOps` 或现有故障注入点确定性触发。
- 并发、回滚、损坏文件和未知字段保留必须验证公开结果与最终字节，而非只测内部调用顺序。

## 禁止模式

```rust
// 错误：绕过事务直接覆盖受管文件
std::fs::write(paths.config_toml(), rendered)?;
```

```rust
// 正确：由业务服务构造操作并交给共享 TransactionService
provider_service.update_provider(input, expected_fingerprint).await?;
```
