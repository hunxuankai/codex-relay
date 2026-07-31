# 测试规范导航

## 开发前检查

- 新功能、缺陷修复、路径或文件写入：读取 [tdd-and-isolation.md](tdd-and-isolation.md)。
- Rust/Tauri 开发反馈、Cargo target、watcher 或 TLS 依赖图：读取
  [rust-build-feedback.md](rust-build-feedback.md)。
- 准备提交、构建或完成声明：读取 [verification.md](verification.md)。

## 质量检查

- 新行为是否先有因预期原因失败的测试？
- 测试是否通过公开边界验证行为，而非内部调用顺序？
- 所有文件测试是否使用临时路径并证明真实默认路径未变化？
- Vitest 是否继续限制为 4 个 worker，且没有用放宽 Sandbox/生产超时代替并发争用修复？
- 报告是否区分测试、构建、人工观察、安装和签名证据？

## 文件

- [TDD 与路径隔离](tdd-and-isolation.md)
- [Rust 开发编译反馈](rust-build-feedback.md)
- [验证与完成证据](verification.md)
