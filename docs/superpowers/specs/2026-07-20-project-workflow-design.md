# Codex Relay 项目工作流整理设计

日期：2026-07-20

## 1. 背景

Codex Relay 当前由 Kai 个人使用 Codex 持续开发。仓库已有约 170 个受 Git 管理文件，
其中约 97 个源码或工程配置文件，Rust、TypeScript、Vue、NSIS 等源码和配置合计约
1.3 万行。项目已经进入中等规模，但现有职责边界、测试体系和开发约束已经形成：

- `AGENTS.md` 记录全仓库必须遵守的安全、配置写入和验证规则；
- `docs/` 记录架构、事务、安全和验证事实；
- `docs/superpowers/specs/` 保存已批准设计；
- `docs/superpowers/plans/` 保存实施计划；
- Superpowers 负责需求澄清、设计、计划、TDD、调试、评审和完成前验证；
- Git 提交记录保存变更历史。

仓库根目录的 `项目提示词.txt` 是创建项目时使用的一次性总需求。应用运行、测试、构建
和打包都不读取它；其中仍然有效的长期规则已经进入 `AGENTS.md`、正式文档、自动化测试
和实现代码。该文件继续保留会形成第二份容易过期的需求来源。

## 2. 目标

1. 删除不再作为权威来源的 `项目提示词.txt`。
2. 清理活动设计、计划和验证文档中对该文件的残留引用。
3. 评估 Trellis 是否能为当前个人 Codex 开发模式带来净收益。
4. 保持开发规则只有一个清晰来源，避免 Trellis 与 Superpowers 重复控制同一阶段。
5. 记录未来重新评估 Trellis 的客观触发条件。

## 3. 方案比较

### 3.1 保持现有工作流，不引入 Trellis（采用）

继续使用 `AGENTS.md + Superpowers + docs + Git`。删除旧提示词，并将本设计作为工作流
决策记录。

优点：

- 没有新的 CLI、生成目录、钩子或任务状态需要维护；
- 不会出现 Superpowers 与 Trellis 同时要求头脑风暴、设计、计划和最终检查；
- 对 Kai 个人使用 Codex 的实际模式最直接；
- 当前安全规则、测试门禁和项目知识已经有稳定载体。

缺点：

- 不提供 Trellis 的任务目录、归档日志和跨 AI 平台自动注入能力；
- 长期任务仍依靠当前 Goal、设计文档、计划文档和 Git 记录衔接。

### 3.2 部分引入 Trellis

只使用 Trellis 的任务登记、状态和 `.trellis/spec/` 项目记忆；Codex 使用
`dispatch_mode: inline`。Superpowers 继续负责设计、计划、TDD、调试和完成验证。

该方案需要额外维护协作契约：Trellis 不启动重复的 brainstorm/check 流程，Trellis PRD
只能引用已批准的 Superpowers 设计，Trellis 检查只能补充而不能替代 Superpowers 的验证。
对于当前单人、单 AI 主流程，这些协调成本高于任务登记带来的收益。

### 3.3 完全采用 Trellis

由 Trellis 接管 create、plan、start、check、finish 和 archive 生命周期，Superpowers 仅保留
TDD、系统化调试等专项能力。该方案适合多人、多 AI 平台和频繁任务交接，但会迁移现有
设计/计划流程，并引入两套技能的优先级治理，不适合当前项目。

## 4. 最终设计

### 4.1 权威信息来源

- `AGENTS.md`：不可违反的全仓库开发和安全规则；
- `docs/architecture.md`、`docs/config-transaction.md`、`docs/security-notes.md`：当前系统设计；
- `docs/superpowers/specs/`：具体变更的已批准设计；
- `docs/superpowers/plans/`：具体变更的可执行计划；
- 自动化测试和实际命令输出：行为与完成状态的证据；
- Git 历史：已经执行的变更记录。

`项目提示词.txt` 不再属于权威信息来源，实施阶段直接删除，不迁移到其他新文件。

### 4.2 Trellis 决策

本轮不安装 Trellis CLI，不运行 `trellis init`，不创建 `.trellis/`、`.agents/skills/` 或新的
Codex 平台集成文件。项目规模本身不足以证明需要新增流程框架；采用框架应由协作和任务
交接问题驱动，而不是单纯由代码行数驱动。

### 4.3 Superpowers 职责

Superpowers 继续作为 Codex Relay 的变更流程：

1. 创造性变更先澄清需求并形成设计；
2. 用户批准设计后写入 spec；
3. 多步骤变更形成 implementation plan；
4. 功能与缺陷遵守 TDD；
5. 异常行为使用系统化调试；
6. 提交或宣布完成前运行与风险相称的验证。

项目安全约束仍以 `AGENTS.md` 为准，不复制到新的工作流配置中。

### 4.4 重新评估 Trellis 的条件

满足以下任一条件时，可以创建新的设计重新评估 Trellis：

- Kai 开始长期同时使用 Codex、Claude Code 等多个 AI 平台；
- 有其他开发者加入并需要共享任务状态和项目记忆；
- 经常并行维护三个以上持续多轮的开发任务；
- 多次发生跨会话上下文丢失，现有 spec、plan 和 Git 记录不足以恢复；
- 需要统一的任务归档、开发日志或自动规范注入；
- 现有 Superpowers 流程被证明无法有效支持项目维护。

如果未来引入，必须先定义明确边界：Trellis 负责任务状态、项目记忆和跨平台注入；
Superpowers 负责设计质量、TDD、调试和完成验证；Codex 默认采用 inline dispatch，禁止
两个框架同时驱动同一阶段。

## 5. 实施范围

实施只包含文档和仓库整理：

1. 删除根目录 `项目提示词.txt`；
2. 修改 `docs/superpowers/specs/2026-07-20-codex-relay-design.md`，将旧提示词引用改为
   已批准设计和项目验收要求；
3. 修改 `docs/superpowers/plans/2026-07-20-codex-relay-implementation.md`，从历史暂存命令中
   移除已删除文件；
4. 修改 `docs/verification-report.md`，将旧提示词引用改为正式发布要求；
5. 不修改应用源码、依赖、构建配置或运行行为。

## 6. 验证

实施完成后运行：

```powershell
Test-Path -LiteralPath '项目提示词.txt'
rg -n '项目提示词\.txt|项目提示词' `
  docs/verification-report.md `
  docs/superpowers/specs/2026-07-20-codex-relay-design.md `
  docs/superpowers/plans/2026-07-20-codex-relay-implementation.md
git diff --check
git status --short
```

验收结果：文件不存在；三个被清理的旧文档不再引用该文件；本决策记录和对应实施计划
可以为了保留审计依据而提及被删除文件；Git diff 没有空白错误；改动范围仅为设计文档、
历史文档引用和文件删除。由于不改变可执行代码，不要求重新运行前后端测试或 Release
构建。
