# Rust 约定

## 模块与职责

- `src-tauri/src/models/`：前后端共享语义、DTO 和事务/健康/设置数据。
- `commands/`：Tauri 适配，不承载文件写入或业务流程。
- `services/`：业务规则与服务组合。
- `infrastructure/`：路径、原子文件、指纹和脱敏日志等可复用机制。
- `app_state.rs`：共享服务、写入守卫和跨命令状态。

保持现有小模块职责，不为单个改动创建第二套路径、日志或事务实现。

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
