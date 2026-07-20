# 错误、日志与事件

## 公开错误

公开错误必须包含稳定 `code` 和可理解的简体中文 `message`。现有关键错误类别包括验证失败、配置损坏、缺少密钥、Provider 不存在、当前 Provider 禁止删除、外部修改冲突、事务失败、回滚不完整、自启失败和系统集成错误。

底层错误、Rust backtrace、完整路径上下文和秘密只可在脱敏后进入内部日志；前端、托盘和 Windows 通知不得收到它们。

## 真实性规则

- 只有所有触及文件恢复并逐字节/存在状态验证成功，消息才可说“原配置已恢复”。
- 回滚不完整必须返回 `TRANSACTION_ROLLBACK_INCOMPLETE`，保留事务标记并引导备份恢复。
- 恢复文件成功但后续 Provider/自检刷新失败时，分别报告“恢复完成”和“状态刷新未完全成功”。
- Codex CLI 缺失或超时是 warning，不阻止 Provider 管理。

## 脱敏范围

必须覆盖 `OPENAI_API_KEY`、`apiKey`、Authorization、Bearer、JSON 密钥字段以及 URL 查询中的 token/key。新增日志时优先不传入秘密，不能依赖正则脱敏替代数据最小化。

## 错误矩阵

| 条件 | 公开行为 |
|---|---|
| `config.toml` 无法解析 | 返回安全错误，不修改文件 |
| `providers.json` 无法解析 | 保存损坏副本，返回错误，不覆盖原件 |
| 编辑指纹过期 | 返回外部修改冲突，不创建旧内容写入 |
| 目标 Provider 无密钥 | 返回缺少密钥，不修改配置 |
| 自动回滚未完全验证 | 报告回滚不完整，保留事务标记 |

## 事件与通知

事件只传 DTO、状态、指纹或安全消息。禁止传 `auth.json`、`providers.json` 全文、Authorization Header 或 API Key。测试快照和 Debug 输出适用同一规则。
