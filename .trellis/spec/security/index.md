# 安全规范导航

安全规则优先于便利性和开发速度。

## 开发前检查

- 涉及路径、fixture、启动脚本、日志或密钥：读取 [path-and-secret-safety.md](path-and-secret-safety.md)。
- 涉及任何配置写入、切换、同步、删除或恢复：读取 [transaction-safety.md](transaction-safety.md)。
- 涉及备份、卸载、清理或保留策略：读取 [data-retention.md](data-retention.md)。

## 质量检查

- 测试/开发是否可能回退到真实 `.codex` 或 `%LOCALAPPDATA%\CodexRelay`？
- 密钥是否可能进入 Git、日志、通知、快照、Trellis 资料或普通前端状态？
- 所有受管文件写入是否经过完整事务和可验证回滚？
- 删除、卸载或清理是否会擅自移除用户配置与备份？

## 文件

- [路径与密钥安全](path-and-secret-safety.md)
- [事务安全](transaction-safety.md)
- [数据保留](data-retention.md)
