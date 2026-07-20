# Codex Relay 全量 Trellis 迁移设计

日期：2026-07-20

## 1. 决策与背景

Kai 个人使用 Codex 持续开发 Codex Relay。引入 Trellis 的主要目标不是应对代码行数增长，
而是解决两个长期问题：

1. 复杂任务在同一 Codex 对话中占满上下文并触发自动压缩后，关键决策、进度和下一步可能
   只存在于聊天内容中；
2. 项目偏好和开发规则持续增加时，全部放入 `AGENTS.md` 会导致每轮都加载大量不相关内容。

本设计采用完整 Trellis 工作流，并选择完全迁移现有 `docs/`：长期有效知识进入
`.trellis/spec/`，当前迁移工作进入 Trellis 任务，一次性计划和过期报告从工作树删除。Git
历史继续保留所有旧文档原文。

本设计取代 `docs/superpowers/specs/2026-07-20-project-workflow-design.md` 中“不引入
Trellis”的旧决定。

## 2. 已确认的本机前置条件

- Trellis CLI：`0.6.7`；
- Python：`3.14.0`；
- Node.js：`22.20.0`；
- npm：`10.9.3`；
- Trellis CLI 支持 `--codex`、`--workflow tdd`、`update --dry-run` 和 `trellis mem`；
- 当前仓库位于 `master`，实施前必须再次确认工作树干净。

如果实施时 CLI 最新版本发生变化，先记录实际版本和更新差异，不静默强制覆盖本地定制。

## 3. 工作流所有权

Trellis 是非平凡开发任务的唯一生命周期负责人：

```text
create → brainstorm/PRD → design → implement plan → start
→ TDD implementation → check → update spec → commit → finish → archive
```

初始化使用 Trellis `tdd` 工作流，并为 Codex 使用 inline dispatch。不得启用 channel 或
sub-agent dispatch。

### 3.1 Trellis 负责

- 创建、启动、定位、完成和归档任务；
- PRD、设计、实施计划和研究资料；
- 当前任务指针；
- 按任务选择并注入相关项目规范；
- TDD 阶段编排；
- 检查阶段编排；
- 规范更新和开发日志。

### 3.2 Superpowers 保留

- `using-superpowers`：能力发现；
- `systematic-debugging`：测试失败或异常行为的根因分析；
- `receiving-code-review`：审查意见的技术验证；
- `verification-before-completion`：任何完成声明必须有新鲜证据。

### 3.3 Superpowers 不再用于 Trellis 任务

- `brainstorming`；
- `writing-plans`；
- `test-driven-development`；
- `executing-plans`；
- `subagent-driven-development`；
- `finishing-a-development-branch`。

这些阶段由 Trellis `tdd` 工作流统一负责，避免重复生成设计、计划、检查清单或分支收尾
菜单。`verification-before-completion` 只约束证据真实性，不创建第二套任务状态。

## 4. 上下文持久化与恢复

每个任务位于：

```text
.trellis/tasks/<task>/
├─ task.json
├─ prd.md
├─ design.md
├─ implement.md
├─ implement.jsonl
├─ check.jsonl
└─ research/
```

### 4.1 强制检查点

`implement.md` 必须持续包含：

```markdown
## 当前进度

## 已完成

## 关键决策

## 验证证据

## 下一步

## 尚未解决的问题
```

以下时机必须更新：

- 每个实施步骤完成后；
- 每次 Git 提交后；
- 关键需求或设计决定改变后；
- 运行耗时较长的测试、构建或外部工具前；
- 对话已经较长、可能发生上下文压缩时；
- 暂停工作或结束当前回合前。

Trellis 不能提前知道 Codex 何时压缩上下文，因此检查点纪律是恢复可靠性的必要条件。

### 4.2 压缩后的恢复顺序

1. 查询当前任务：

   ```powershell
   python .trellis/scripts/task.py current --source
   ```

2. 读取 `task.json`、`prd.md`、`design.md` 和 `implement.md`；
3. 验证 `implement.jsonl` 和 `check.jsonl` 引用仍存在；
4. 只加载当前阶段需要的 `.trellis/spec/` 文件；
5. 如果检查点仍缺少某段对话细节，使用 `trellis mem search` 和
   `trellis mem context` 从当前项目的 Codex 会话历史中按关键词恢复；
6. 将恢复到的新结论立即写回任务文件，不能继续只依赖聊天上下文。

`trellis mem` 是补救手段，不代替任务检查点。

## 5. 规则分层

### 5.1 精简后的 `AGENTS.md`

只保留每轮必须加载的最高优先级规则：

1. 非平凡开发必须使用当前 Trellis 任务；
2. Trellis `tdd` 工作流拥有完整生命周期；
3. 严禁测试或开发触碰真实 `.codex` 和 `%LOCALAPPDATA%\CodexRelay`；
4. 严禁泄漏、记录或提交真实密钥；
5. 配置写入必须经过事务服务且不能破坏 TOML 未知内容；
6. 完成声明必须基于本轮实际验证；
7. 详细规则必须根据当前任务从 `.trellis/spec/` 选择加载；
8. 项目文档和 Trellis 任务材料默认使用简体中文。

### 5.2 `.trellis/spec/` 目标结构

```text
.trellis/spec/
├─ project/
│  ├─ index.md
│  ├─ product-contract.md
│  └─ architecture.md
├─ security/
│  ├─ index.md
│  ├─ path-and-secret-safety.md
│  ├─ transaction-safety.md
│  └─ data-retention.md
├─ backend/
│  ├─ index.md
│  ├─ rust-guidelines.md
│  ├─ service-boundaries.md
│  └─ error-and-logging.md
├─ frontend/
│  ├─ index.md
│  ├─ vue-guidelines.md
│  ├─ state-management.md
│  └─ accessibility.md
├─ testing/
│  ├─ index.md
│  ├─ tdd-and-isolation.md
│  └─ verification.md
├─ release/
│  ├─ index.md
│  ├─ tauri-nsis.md
│  └─ signing.md
└─ workflow/
   ├─ index.md
   ├─ trellis-superpowers.md
   ├─ context-recovery.md
   └─ documentation.md
```

每个 `index.md` 只做简短导航。详细文件按任务通过 `implement.jsonl` 或 `check.jsonl`
选择，禁止每次全量注入整个规范树。

## 6. 现有文档迁移清单

| 当前文件 | 处理方式 | 目标或理由 |
|---|---|---|
| `docs/architecture.md` | 迁移后删除 | `.trellis/spec/project/architecture.md`，并拆出前后端边界摘要 |
| `docs/config-transaction.md` | 迁移后删除 | `.trellis/spec/security/transaction-safety.md` |
| `docs/security-notes.md` | 迁移后删除 | `.trellis/spec/security/path-and-secret-safety.md` 与 `data-retention.md` |
| `docs/verification-report.md` | 提取后删除 | 长期验证原则进入 `testing/verification.md`；过期运行结果只保留在 Git 历史 |
| `docs/superpowers/specs/2026-07-20-codex-relay-design.md` | 拆分迁移后删除 | 产品契约、架构、安全、测试和发布规范 |
| `docs/superpowers/specs/2026-07-20-conditional-nsis-install-directory-design.md` | 迁移后删除 | `.trellis/spec/release/tauri-nsis.md` |
| `docs/superpowers/specs/2026-07-20-project-workflow-design.md` | 删除 | 已被本设计取代 |
| `docs/superpowers/specs/2026-07-20-trellis-full-migration-design.md` | 迁入任务后删除 | 成为 Trellis 初始化任务的 `design.md` |
| `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md` | 删除 | 已完成的一次性实施计划，Git 历史保留 |
| `docs/superpowers/plans/2026-07-20-conditional-nsis-install-directory.md` | 删除 | 已完成的一次性实施计划，长期规则已迁入 release spec |
| `docs/superpowers/plans/2026-07-20-project-workflow-cleanup.md` | 删除 | 已完成的一次性实施计划 |
| `docs/superpowers/plans/2026-07-20-readme-development-accuracy.md` | 删除 | 已完成的一次性实施计划，最终行为已进入代码和 README |

迁移结束后 `docs/` 目录不存在。README 的进一步阅读链接改为 `.trellis/spec/` 对应入口。

## 7. 初始化与迁移顺序

1. 记录当前 Git 状态、Trellis/Python/Node 版本和完整测试基线；
2. 使用 `trellis init -u kai --codex --workflow tdd --skip-existing` 初始化，禁止 `--force`；
3. 检查全部生成文件和 `.trellis/.template-hashes.json`，确认现有 `AGENTS.md` 未被覆盖；
4. 在 `.trellis/config.yaml` 明确 Codex inline dispatch；
5. 使用初始化产生的 bootstrap 任务承载本迁移的 PRD、设计、实施计划和研究记录；
6. 按第 6 节逐项提取长期内容，写入分层 spec；
7. 缩减 `AGENTS.md`，写入 Trellis/Superpowers 冲突边界；
8. 更新 README 的开发工作流、上下文恢复、规范入口和 Trellis CLI 版本说明；
9. 删除已迁移或无继续价值的 `docs/`；
10. 验证任务上下文、规范索引、`trellis mem` 和完整项目检查；
11. 完成本次 bootstrap/迁移任务并归档；
12. 分批提交生成框架、规范迁移、文档删除和最终验证，避免一个不可审查的大提交。

## 8. 本机文件与 Git 边界

初始化后必须逐项判断生成文件是否属于：

- 团队共享模板、任务、spec、脚本和 Codex 集成：提交；
- 开发者身份、当前会话指针、缓存、临时日志或机器状态：加入 `.gitignore`，不得提交；
- Trellis 管理文件的本地定制：记录原因，并使用 `trellis update --dry-run` 验证未来升级行为。

不得假设 `.developer`、`.current-task` 或其他点文件天然适合提交，以 Trellis 生成说明和实际
用途为准。

## 9. 冲突与失败处理

- 初始化前不删除任何旧文档；
- 只有目标 spec 已写入、检查并建立索引后，才删除对应旧文件；
- 如果 Trellis 初始化覆盖现有文件，立即停止，不在覆盖结果上继续迁移；
- 如果 `task.py validate`、`trellis update --dry-run` 或上下文恢复验证失败，保留旧文档并修复
  框架配置；
- 任何安全规则迁移都必须逐项比对，不能用“已迁移”替代内容核验；
- 不向 Trellis 任务、spec、会话记忆或日志写入真实 API Key；
- Git 提交提供回滚边界，不使用破坏性 reset 清理失败迁移。

## 10. 验证与验收

至少验证：

1. `trellis --version` 和 `trellis update --dry-run` 成功；
2. `.trellis/workflow.md` 使用 `tdd` 模板；
3. `.trellis/config.yaml` 使用 Codex inline dispatch；
4. 当前 bootstrap 任务可以被 create/start/current/validate/finish/archive 流程识别；
5. `implement.jsonl` 和 `check.jsonl` 只引用存在的相关规范；
6. 所有 spec 分层都有 `index.md`，索引内容和文件实际存在性一致；
7. 从任务文件可以在不读取旧聊天全文的情况下说明当前目标、已完成、关键决策和下一步；
8. `trellis mem projects` 和当前项目范围的查询可用，但不暴露密钥；
9. `AGENTS.md` 精简后仍保留全部不可违反的安全红线；
10. `docs/` 已删除，README 不含失效链接；
11. 仓库不提交本机身份、缓存、当前会话状态或临时文件；
12. `npm run check` 全部通过；
13. `git diff --check` 和暂存差异检查通过；
14. 独立审计能把每条旧安全规则映射到 `AGENTS.md` 或一个具体 Trellis spec。

本次只改变开发流程、规则组织和文档位置，不改变 Codex Relay 产品运行行为，因此除非迁移
意外触及构建配置，不要求重新生成 NSIS 安装包。
