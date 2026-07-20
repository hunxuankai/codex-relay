# Codex Relay 全量 Trellis 迁移实施检查点

## 当前进度

Trellis 框架、迁移任务和七个领域规范已建立并通过结构审计；正在精简入口文档并准备删除已完成映射的旧 `docs/`。

## 已完成

- 已批准迁移设计，提交为 `9367e79 docs: design full Trellis migration`。
- 已创建中文实施计划，提交为 `3f49d36 docs: plan full Trellis migration`。
- 已确认工作分支为 `master`，并获得 Kai 对直接实施的批准。
- 已记录 Trellis/Python/Node/npm 版本及 AGENTS/README/package 哈希。
- 迁移前 `npm run check` 通过：Vitest 15 个文件、65 个测试；Rust 107 个单元测试、2 个路径安全测试、1 个 Provider 工作流测试；fmt 与 Clippy 通过。
- 首次初始化因缺少非交互参数停在 spec 模板选择，确认未生成文件；加入 `--yes` 后初始化成功。
- 初始化后 AGENTS/README/package 哈希未变化，现有文件未被覆盖。
- bootstrap 任务已写入中文 PRD、设计、实施检查点、研究映射和 implement/check JSONL。
- 已用 `TRELLIS_CONTEXT_ID=codex-relay-trellis-migration` 建立当前任务指针，`current --source` 返回该任务。
- `task.py validate 00-bootstrap-guidelines` 通过：implement 5 条、check 7 条。
- 已建立 project、security、backend、frontend、testing、release、workflow 七个领域和全部索引。
- 已完整读取架构、事务、安全、主产品设计、NSIS 设计和验证报告，并将长期规则映射到新规范。
- spec 索引链接检查通过；路径、密钥、事务、回滚、卸载、NSIS、签名、Vue 和新鲜证据关键词均有明确目标。

## 关键决策

- Trellis 完整接管非平凡任务生命周期，工作流使用 `tdd`。
- Codex 使用 inline dispatch，不使用 channel 或 sub-agent dispatch。
- 复用 `00-bootstrap-guidelines` 承载本次迁移，不创建重复任务。
- `.developer`、`.runtime/`、缓存与临时状态保持 Git 忽略；共享脚本、任务与规范进入 Git。
- 所有项目文档与任务材料默认使用简体中文。

## 验证证据

```text
trellis --version: 0.6.7
python --version: Python 3.14.0
node --version: v22.20.0
npm --version: 10.9.3
npm run check: exit 0
trellis init ... --skip-existing --yes: exit 0
task.py current --source: session:codex-relay-trellis-migration
task.py validate: implement 5 entries, check 7 entries, exit 0
spec index link audit: All spec index links exist
AGENTS.md SHA256: 49FF749AFB0524F1E2695D77A5BCEA9124B9D92775571D5FA4D7B2B46D57725D
README.md SHA256: 1E416E7F91B3F39B8F3F057AA4EA7D5F36AAF0475ACE2B79B9EEC1F76DDBED46
package.json SHA256: 4CD14A652ADF6A0776E4EEFB39A0E5A6FF55AD0C556F3B56E74A766D76773FF8
```

## 下一步

1. 提交 Trellis 框架、任务和分层规范作为迁移回滚点。
2. 检查精简后的 AGENTS 和 README，不含旧文档引用后删除 docs。
3. 演练恢复、`trellis update --dry-run` 和 `trellis mem`。
4. 运行完整项目检查、安全/Git 边界检查，提交并归档。

## 尚未解决的问题

- 需要检查 `trellis update --dry-run` 对本地 spec 结构和 inline 配置的差异报告。
- 需要确认 `trellis mem` 在当前 Codex 会话历史上的实际可用范围，并检查输出不含秘密。
