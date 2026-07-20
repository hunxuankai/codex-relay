# 旧文档迁移来源映射

## 长期内容来源

| 源文件 | 目标 |
|---|---|
| `docs/architecture.md` | `project/architecture.md`、`backend/service-boundaries.md`、`frontend/state-management.md` |
| `docs/config-transaction.md` | `security/transaction-safety.md`、`backend/service-boundaries.md` |
| `docs/security-notes.md` | `security/path-and-secret-safety.md`、`security/data-retention.md`、`release/signing.md` |
| 主产品设计 | `project/product-contract.md` 以及 backend/frontend/testing/release 各规范 |
| 条件 NSIS 设计 | `release/tauri-nsis.md` |
| `docs/verification-report.md` | `testing/verification.md` 中的证据原则、构建锁文件陷阱和安全审计方式 |
| Trellis 迁移设计 | 当前任务 `prd.md`、`design.md`、`implement.md` 与 `workflow/` |

## 不迁移为长期规范

- 旧报告的时间戳、旧产物大小和 SHA-256。
- 已完成任务的逐步实施计划。
- 被全量 Trellis 决策取代的“不引入 Trellis”工作流设计。
- 仅描述当时执行过程、对未来实现没有约束力的命令日志。

上述内容继续由 Git 历史保留，可在确有需要时追溯。
