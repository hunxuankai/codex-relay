# Rust 后端规范导航

## 开发前检查

- 修改 Rust 结构、依赖或序列化：读取 [rust-guidelines.md](rust-guidelines.md)。
- 修改 command、service、repository/infrastructure 边界：读取 [service-boundaries.md](service-boundaries.md)。
- 修改错误、日志、通知或事件：读取 [error-and-logging.md](error-and-logging.md)。
- 涉及配置写入时同时读取 `../security/transaction-safety.md`。

## 质量检查

- command 是否只做参数转换、单次服务调用与结果映射？
- 是否复用 `AppState` 中的共享事务锁和服务实例？
- 是否保留 TOML 未知内容、JSON 格式和稳定错误码？
- 是否为新行为提供 Rust 单元或临时目录集成测试？

## 文件

- [Rust 约定](rust-guidelines.md)
- [服务边界](service-boundaries.md)
- [错误与日志](error-and-logging.md)
