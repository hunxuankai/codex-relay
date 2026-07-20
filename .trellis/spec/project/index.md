# 项目规范导航

Codex Relay 的产品边界与总体架构。实现跨前后端、配置文件或 Windows 系统集成的任务时先读本目录。

## 开发前检查

- 产品能力、非目标或兼容性边界：读取 [product-contract.md](product-contract.md)。
- 跨层数据流、文件所有权或启动流程：读取 [architecture.md](architecture.md)。
- 再按改动领域加载 `backend/`、`frontend/`、`security/`、`testing/` 或 `release/`。

## 质量检查

- 改动是否仍面向 Windows 当前用户的本机 Provider 管理？
- 是否保持 `config.toml`、`providers.json`、`auth.json` 的权威关系？
- 是否引入了网络验证、密钥加密、云同步等未批准能力？
- 跨层调用是否沿用 Vue → typed service/composable → Tauri command → Rust service → infrastructure？

## 文件

- [产品契约](product-contract.md)
- [总体架构](architecture.md)
