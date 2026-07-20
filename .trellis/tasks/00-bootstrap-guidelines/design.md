# Codex Relay 全量 Trellis 迁移设计

## 1. 工作流所有权

Trellis 是非平凡开发任务的唯一生命周期负责人：

```text
create → PRD → design → implement plan → start
→ TDD implementation → check → update spec → commit → finish → archive
```

Trellis 负责任务、研究、上下文注入、TDD 编排、检查、规范更新和开发日志。Superpowers 只保留能力发现、系统化调试、审查反馈验证和完成前证据验证；在 Trellis 任务中不再另行运行 brainstorming、writing-plans、test-driven-development、executing-plans、subagent-driven-development 或 finishing-a-development-branch，以免产生第二套任务状态。

Codex 使用 inline dispatch；不启用 channel 或 sub-agent dispatch。

## 2. 上下文持久化

每个复杂任务维护：

```text
task.json
prd.md
design.md
implement.md
implement.jsonl
check.jsonl
research/
```

`implement.md` 固定包含“当前进度、已完成、关键决策、验证证据、下一步、尚未解决的问题”。每个实施步骤、Git 提交、关键决定改变、长时间验证之前、对话可能压缩时以及暂停前都要更新。

上下文恢复顺序：

1. `python .trellis/scripts/task.py current --source`；
2. 读取任务的 JSON、PRD、设计与实施检查点；
3. 验证 JSONL 引用；
4. 只加载当前阶段需要的 spec；
5. 信息仍不足时使用 `trellis mem search` 和 `trellis mem context`；
6. 将恢复结论立即写回任务文件。

`trellis mem` 只作补救，不替代检查点。

## 3. 规则分层

`AGENTS.md` 只保留每轮必读红线。长期规则进入：

```text
.trellis/spec/
├─ project/
├─ security/
├─ backend/
├─ frontend/
├─ testing/
├─ release/
└─ workflow/
```

每个领域的 `index.md` 只负责导航、开发前检查和质量检查；详细文件按任务选择加载，禁止每次全量注入。

## 4. 文档迁移

- `architecture.md` 和主产品设计拆入 project、backend、frontend、testing 与 release。
- `config-transaction.md` 迁入 security/transaction-safety。
- `security-notes.md` 迁入 security/path-and-secret-safety 与 data-retention。
- NSIS 设计迁入 release/tauri-nsis。
- 验证报告只提取长期验证原则，过期运行结果留在 Git 历史。
- 被新决定取代的工作流设计、已完成计划和本迁移过渡设计在迁移完成后删除。
- README 的进一步阅读改到 `.trellis/spec/`，最终 `docs/` 不存在。

## 5. Git 与本机边界

共享工作流、脚本、任务、spec、Codex 集成和模板哈希提交。`.developer`、`.runtime/`、当前任务指针、缓存、日志和临时文件由 `.trellis/.gitignore` 隔离。升级必须先运行 `trellis update --dry-run`，禁止静默覆盖本地定制。

## 6. 失败与回滚

- 初始化前不删除旧文档，初始化使用 `--skip-existing --yes`，禁止 `--force`。
- 目标规范写入并通过索引与安全映射检查后，才删除源文档。
- 初始化、validate、dry-run 或恢复演练失败时保留旧文档，先修复框架。
- Git 提交提供回滚边界，不使用破坏性 reset。
- 所有失败、限制、未执行的签名/安装/手工验证必须如实记录。

## 7. 验证

验证 Trellis 版本与 TDD 模板、inline 配置、任务生命周期、JSONL 引用、所有索引、上下文恢复、记忆查询、安全规则映射、README 链接、Git 私有边界、`npm run check` 和 `git diff --check`。
