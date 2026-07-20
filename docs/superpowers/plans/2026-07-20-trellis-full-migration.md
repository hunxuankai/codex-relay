# Codex Relay 全量 Trellis 迁移实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**目标：** 将 Codex Relay 的非平凡开发生命周期、上下文检查点和分层项目规范完整迁入 Trellis，并在验证通过后删除旧 `docs/` 文档树。

**架构：** 使用 Trellis 0.6.7 的 Codex + `tdd` 模板生成共享工作流，由 bootstrap 任务承载本次迁移；长期规则按 project、security、backend、frontend、testing、release、workflow 七个领域写入 `.trellis/spec/`。`AGENTS.md` 只保留每轮都必须加载的红线，README 只保留面向使用者的说明和规范入口，本机身份、当前任务指针及缓存不进入 Git。

**技术栈：** Trellis CLI 0.6.7、Python 3.14、PowerShell、Git、Markdown、JSONL、Tauri 2、Vue 3、Rust、npm。

---

### 任务 1：记录干净基线与工具版本

**文件：**
- 检查：`package.json`
- 检查：`AGENTS.md`
- 检查：`docs/superpowers/specs/2026-07-20-trellis-full-migration-design.md`

- [ ] **步骤 1：确认仓库状态和当前分支**

运行：

```powershell
git status --short --branch
git branch --show-current
```

预期：分支为 `master`，除本实施计划外没有未说明的改动。

- [ ] **步骤 2：记录本机工具版本**

运行：

```powershell
trellis --version
python --version
node --version
npm --version
```

预期：Trellis `0.6.7`、Python `3.14.0`、Node.js `22.20.0`、npm `10.9.3`，或明确记录实际无破坏性差异。

- [ ] **步骤 3：运行迁移前完整检查**

运行：

```powershell
npm run check
```

预期：前端 typecheck、Vitest、Rust fmt、Clippy 和 Rust tests 全部通过。若失败，停止迁移并先说明基线失败。

- [ ] **步骤 4：提交过渡实施计划**

```powershell
git add docs/superpowers/plans/2026-07-20-trellis-full-migration.md
git commit -m "docs: plan full Trellis migration"
```

### 任务 2：安全初始化 Trellis 并审计生成文件

**文件：**
- 创建：`.trellis/`
- 可能修改：`.gitignore`
- 检查：`AGENTS.md`

- [ ] **步骤 1：保存初始化前关键文件哈希**

运行：

```powershell
Get-FileHash AGENTS.md, README.md, package.json -Algorithm SHA256
```

预期：输出三个文件的 SHA256，供初始化后比对。

- [ ] **步骤 2：使用非覆盖模式初始化**

运行：

```powershell
trellis init -u kai --codex --workflow tdd --skip-existing
```

预期：初始化成功；命令中不得出现 `--force`。

- [ ] **步骤 3：确认现有关键文件未被覆盖**

运行：

```powershell
Get-FileHash AGENTS.md, README.md, package.json -Algorithm SHA256
git status --short
rg --files .trellis
```

预期：三个哈希与初始化前一致；所有新增 Trellis 文件均可枚举。

- [ ] **步骤 4：审计 Git 边界**

逐项检查 `.developer`、`.current-task`、缓存、日志、临时文件和 `.trellis/.template-hashes.json` 的用途。共享模板、脚本、spec 和任务材料保留；本机身份、当前会话指针、缓存及临时状态写入 `.gitignore`。运行：

```powershell
git status --short --ignored
git check-ignore -v .trellis/.developer .trellis/.current-task 2>$null
```

预期：机器私有状态被忽略，共享工作流文件未被误忽略。

### 任务 3：建立 bootstrap 迁移任务与 Codex inline 配置

**文件：**
- 修改：`.trellis/config.yaml`
- 修改：`.trellis/workflow.md`
- 创建或修改：`.trellis/tasks/<bootstrap-task>/task.json`
- 创建或修改：`.trellis/tasks/<bootstrap-task>/prd.md`
- 创建或修改：`.trellis/tasks/<bootstrap-task>/design.md`
- 创建或修改：`.trellis/tasks/<bootstrap-task>/implement.md`
- 创建或修改：`.trellis/tasks/<bootstrap-task>/implement.jsonl`
- 创建或修改：`.trellis/tasks/<bootstrap-task>/check.jsonl`

- [ ] **步骤 1：检查 Trellis 实际命令和生成的 bootstrap 任务**

运行：

```powershell
trellis --help
python .trellis/scripts/task.py --help
python .trellis/scripts/task.py current --source
```

预期：确认生成版本的真实 create/start/current/validate/finish/archive 接口；若初始化已创建 bootstrap 任务，复用它，不创建第二个迁移任务。

- [ ] **步骤 2：设置 Codex inline dispatch**

在 `.trellis/config.yaml` 中确保存在且仅存在以下 Codex 派发配置：

```yaml
codex:
  dispatch_mode: inline
```

不得启用 channel 或 sub-agent dispatch。

- [ ] **步骤 3：确认工作流为 TDD 模板**

运行：

```powershell
Select-String -Path .trellis/workflow.md -Pattern 'tdd|test|red|green' -CaseSensitive:$false
```

预期：工作流内容明确包含测试先行和验证阶段，不是默认或其他工作流模板。

- [ ] **步骤 4：迁入 PRD、设计与实施检查点**

将已批准设计全文迁入任务 `design.md`，在 `prd.md` 明确目标、非目标、安全边界和验收条件；`implement.md` 必须包含并持续更新：

```markdown
## 当前进度

## 已完成

## 关键决策

## 验证证据

## 下一步

## 尚未解决的问题
```

`implement.jsonl` 与 `check.jsonl` 只引用本任务实际需要且已经存在的规范文件。

- [ ] **步骤 5：验证当前任务结构**

运行生成版本支持的 validate 命令，并再次运行：

```powershell
python .trellis/scripts/task.py current --source
```

预期：当前任务可定位，任务文件完整，所有 JSONL 行可解析且引用存在。

- [ ] **步骤 6：提交 Trellis 框架和 bootstrap 任务**

```powershell
git add .trellis .gitignore
git diff --cached --check
git commit -m "chore: initialize Trellis workflow"
```

### 任务 4：迁移项目与后端规范

**文件：**
- 创建：`.trellis/spec/project/index.md`
- 创建：`.trellis/spec/project/product-contract.md`
- 创建：`.trellis/spec/project/architecture.md`
- 创建：`.trellis/spec/backend/index.md`
- 创建：`.trellis/spec/backend/rust-guidelines.md`
- 创建：`.trellis/spec/backend/service-boundaries.md`
- 创建：`.trellis/spec/backend/error-and-logging.md`
- 读取：`docs/architecture.md`
- 读取：`docs/superpowers/specs/2026-07-20-codex-relay-design.md`

- [ ] **步骤 1：提取产品契约**

在 `product-contract.md` 记录 Windows 10/11、单用户本机 Provider 管理、`responses` Wire API、无远程密钥验证、无自动更新/云同步/团队权限、卸载不删除用户数据等长期边界。

- [ ] **步骤 2：提取总体架构**

在 `architecture.md` 记录 Vue/Tauri/Rust 边界、前端 invoke 适配层、Rust command/service/repository 分层、配置文件职责、事务服务和托盘/单实例/自检关系。

- [ ] **步骤 3：拆分后端规则**

在后端三个文件分别记录 Rust 代码约定、command/service/repository 边界、稳定错误码与脱敏日志要求；索引文件只描述何时加载各规范。

- [ ] **步骤 4：执行内容覆盖检查**

运行：

```powershell
rg -n "Windows 10|Windows 11|responses|TransactionService|providers.json|auth.json|config.toml|脱敏|错误码" .trellis/spec/project .trellis/spec/backend
```

预期：每类长期约束至少映射到一个明确文件，索引中的链接都存在。

### 任务 5：迁移安全规范

**文件：**
- 创建：`.trellis/spec/security/index.md`
- 创建：`.trellis/spec/security/path-and-secret-safety.md`
- 创建：`.trellis/spec/security/transaction-safety.md`
- 创建：`.trellis/spec/security/data-retention.md`
- 读取：`docs/config-transaction.md`
- 读取：`docs/security-notes.md`
- 读取：`AGENTS.md`

- [ ] **步骤 1：迁移路径与密钥红线**

明确真实 `%USERPROFILE%\.codex`、`%LOCALAPPDATA%\CodexRelay` 禁止被开发和自动化测试触碰；覆盖变量必须成对存在且不得指向真实目录；密钥不得进入 Git、日志、通知、快照、普通前端状态或 Trellis 资料。

- [ ] **步骤 2：迁移事务完整性规则**

明确应用级事务锁、外部修改指纹、事务备份、临时文件、解析验证、原子替换、写后验证、回滚验证、`toml_edit` 局部修改、损坏 `providers.json` 保留副本等不可省略步骤。

- [ ] **步骤 3：迁移数据保留规则**

明确备份快照可能含明文密钥、元数据不得含密钥、最多保留 20 份、卸载不删除 Codex 配置和应用数据、用户自行清理前应确认恢复需求。

- [ ] **步骤 4：逐条映射旧安全规则**

为旧 `AGENTS.md` 第 1–8、11–12 条和安全文档中的每条强制要求建立映射表，确认它们进入精简 `AGENTS.md` 或具体 security/testing/release spec 后再允许删除源文件。

### 任务 6：迁移前端、测试和发布规范

**文件：**
- 创建：`.trellis/spec/frontend/index.md`
- 创建：`.trellis/spec/frontend/vue-guidelines.md`
- 创建：`.trellis/spec/frontend/state-management.md`
- 创建：`.trellis/spec/frontend/accessibility.md`
- 创建：`.trellis/spec/testing/index.md`
- 创建：`.trellis/spec/testing/tdd-and-isolation.md`
- 创建：`.trellis/spec/testing/verification.md`
- 创建：`.trellis/spec/release/index.md`
- 创建：`.trellis/spec/release/tauri-nsis.md`
- 创建：`.trellis/spec/release/signing.md`
- 读取：`docs/verification-report.md`
- 读取：`docs/superpowers/specs/2026-07-20-codex-relay-design.md`
- 读取：`docs/superpowers/specs/2026-07-20-conditional-nsis-install-directory-design.md`

- [ ] **步骤 1：迁移 Vue 与状态边界**

记录 Vue 3 Composition API、`<script setup lang="ts">`、typed props/emits、只读 composable 状态、唯一 `src/services/tauri.ts` invoke 边界、可访问语义和键盘焦点要求。

- [ ] **步骤 2：迁移 TDD、隔离和验证原则**

记录测试必须先因预期原因失败、测试路径双覆盖、`tempfile`/`AppPaths::for_test`、默认路径哨兵、按风险选择 typecheck/Vitest/fmt/Clippy/Rust tests/build，以及完成声明需要本轮证据。只提取 `verification-report.md` 中可复用原则，不复制过期成功结果。

- [ ] **步骤 3：迁移 NSIS 与签名规则**

记录 per-machine 安装、管理员权限、固定 D 盘存在时默认 `D:\Program Files\Codex Relay`、无 D 盘时使用系统 Program Files、升级沿用既有目录、卸载保留数据、产物路径需实测枚举，以及无证书时不得声称已签名。

- [ ] **步骤 4：检查索引和关键约束**

运行：

```powershell
rg -n "script setup|src/services/tauri.ts|AppPaths::for_test|CODEX_RELAY_CODEX_HOME|D:\\Program Files|per-machine|签名" .trellis/spec/frontend .trellis/spec/testing .trellis/spec/release
```

预期：所有关键词均有清晰、无冲突的长期规范。

### 任务 7：建立 Trellis 工作流与上下文恢复规范

**文件：**
- 创建：`.trellis/spec/workflow/index.md`
- 创建：`.trellis/spec/workflow/trellis-superpowers.md`
- 创建：`.trellis/spec/workflow/context-recovery.md`
- 创建：`.trellis/spec/workflow/documentation.md`

- [ ] **步骤 1：定义唯一生命周期所有权**

在 `trellis-superpowers.md` 明确 Trellis 负责 create、PRD、design、implement plan、start、TDD、check、spec update、finish、archive；保留 Superpowers 的能力发现、系统调试、审查反馈验证和完成前验证，不在 Trellis 任务中再次运行 brainstorming、writing-plans、test-driven-development、executing-plans、subagent-driven-development 或 finishing-a-development-branch。

- [ ] **步骤 2：定义强制检查点和恢复顺序**

在 `context-recovery.md` 写入 `implement.md` 六个固定标题、六类必须更新时机，以及压缩后先运行：

```powershell
python .trellis/scripts/task.py current --source
```

随后读取任务文件、校验 JSONL 引用、按需加载 spec，最后才使用：

```powershell
trellis mem search "<关键词>"
trellis mem context <session-id>
```

- [ ] **步骤 3：定义中文文档和知识归档规则**

在 `documentation.md` 规定项目文档与任务材料默认简体中文；长期规则进入 spec，任务过程进入 task，一次性运行结果进入任务检查点或 Git 历史；索引只导航，不全量注入规范树。

- [ ] **步骤 4：提交分层规范**

```powershell
git add .trellis/spec .trellis/tasks
git diff --cached --check
git commit -m "docs: migrate project knowledge to Trellis specs"
```

### 任务 8：精简 AGENTS 并更新 README

**文件：**
- 修改：`AGENTS.md`
- 修改：`README.md`

- [ ] **步骤 1：将 AGENTS 精简为最高优先级规则**

最终 `AGENTS.md` 只保留以下八类要求：非平凡开发使用当前 Trellis 任务；Trellis `tdd` 工作流拥有生命周期；严禁触碰真实用户路径；严禁泄漏真实密钥；配置写入必须经过事务服务并保留未知 TOML；完成声明需要新鲜证据；按任务从 `.trellis/spec/` 选择加载详细规则；项目文档默认简体中文。

- [ ] **步骤 2：更新 README 目录和工作流说明**

把 `docs/` 目录项替换为 `.trellis/`；增加 Trellis 0.6.7、非平凡任务入口、当前任务恢复命令和规范索引说明，同时保留所有最终用户、开发启动、测试和打包说明。

- [ ] **步骤 3：替换进一步阅读链接**

将 README 的四个旧 `docs/` 链接替换为：

```markdown
- [项目与产品规范](.trellis/spec/project/index.md)
- [安全规范](.trellis/spec/security/index.md)
- [测试与验证规范](.trellis/spec/testing/index.md)
- [发布与 NSIS 规范](.trellis/spec/release/index.md)
- [Trellis 工作流与上下文恢复](.trellis/spec/workflow/index.md)
```

- [ ] **步骤 4：验证 README 不含失效旧入口**

运行：

```powershell
rg -n "docs/|docs\\" README.md AGENTS.md
```

预期：无匹配。

### 任务 9：删除旧 docs 并完成迁移审计

**文件：**
- 删除：`docs/architecture.md`
- 删除：`docs/config-transaction.md`
- 删除：`docs/security-notes.md`
- 删除：`docs/verification-report.md`
- 删除：`docs/superpowers/specs/*.md`
- 删除：`docs/superpowers/plans/*.md`

- [ ] **步骤 1：确认所有源文档已有目标映射**

运行：

```powershell
rg --files docs
rg --files .trellis/spec
```

逐项对照批准设计第 6 节；任一长期约束无目标文件时停止删除并先补齐。

- [ ] **步骤 2：删除完整 docs 目录**

使用补丁删除所有已迁移或已过期文档，确保 `docs/` 不再存在。Git 历史继续保留原文。

- [ ] **步骤 3：检查引用完整性和安全规则映射**

运行：

```powershell
rg -n "docs/|docs\\" -g '!node_modules/**' -g '!src-tauri/target/**' .
rg -n "真实|API Key|TransactionService|toml_edit|回滚|卸载|新鲜|本轮" AGENTS.md .trellis/spec
```

预期：没有活动文件指向旧 `docs/`；每条旧安全红线都能定位到 `AGENTS.md` 或具体 spec。

- [ ] **步骤 4：提交入口调整和旧文档删除**

```powershell
git add AGENTS.md README.md .trellis docs
git diff --cached --check
git commit -m "docs: complete Trellis documentation migration"
```

### 任务 10：演练恢复、升级和记忆能力

**文件：**
- 修改：`.trellis/tasks/<bootstrap-task>/implement.md`
- 修改：`.trellis/tasks/<bootstrap-task>/check.jsonl`

- [ ] **步骤 1：验证任务和规范结构**

运行生成版本实际支持的 task validate 命令，并检查所有 JSONL：

```powershell
Get-ChildItem .trellis/tasks -Recurse -Filter *.jsonl | ForEach-Object {
  Get-Content $_.FullName | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json | Out-Null }
}
```

预期：任务验证通过，所有非空 JSONL 行均为合法 JSON。

- [ ] **步骤 2：执行上下文恢复演练**

仅通过以下命令输出定位当前任务：

```powershell
python .trellis/scripts/task.py current --source
```

随后只读取 `task.json`、`prd.md`、`design.md`、`implement.md` 和引用的 spec，确认可以回答目标、已完成、关键决策、验证证据、下一步和未解决问题。

- [ ] **步骤 3：验证 Trellis 升级预览**

运行：

```powershell
trellis update --dry-run
```

预期：命令成功，输出不会静默覆盖本地定制；任何差异都写入任务验证证据。

- [ ] **步骤 4：验证记忆检索但不记录密钥**

运行：

```powershell
trellis mem projects
trellis mem search "Trellis 迁移"
```

若搜索返回 session id，再运行 `trellis mem context <session-id>`。预期：项目查询可用；输出检查不得包含真实 API Key、Authorization Header 或完整认证文件。

### 任务 11：完整验证、完成任务和归档

**文件：**
- 修改：`.trellis/tasks/<bootstrap-task>/implement.md`
- 修改：`.trellis/tasks/<bootstrap-task>/task.json`
- 可能移动：`.trellis/tasks/<bootstrap-task>/`

- [ ] **步骤 1：运行项目完整检查**

运行：

```powershell
npm run check
```

预期：typecheck、Vitest、Rust fmt、Clippy 和 Rust tests 全部通过。

- [ ] **步骤 2：运行仓库静态收尾检查**

运行：

```powershell
git diff --check
git status --short --ignored
rg -n "docs/|docs\\" -g '!node_modules/**' -g '!src-tauri/target/**' .
```

预期：无空白错误、无旧文档引用、无待解释的私有或缓存文件进入 Git。

- [ ] **步骤 3：更新最终检查点**

在 `implement.md` 写入实际完成项、所有命令及结果、关键决策、下一步和真实限制；不得把未运行的签名、安装或手工验证写成成功。

- [ ] **步骤 4：按生成版本真实接口完成并归档任务**

先再次运行 validate，再使用 task.py 或 Trellis CLI 的实际 finish/archive 命令。完成后运行：

```powershell
python .trellis/scripts/task.py current --source
git status --short --branch
```

预期：迁移任务状态为已完成并归档；不存在悬空当前任务指针。

- [ ] **步骤 5：提交最终验证与归档状态**

```powershell
git add .trellis .gitignore
git diff --cached --check
git commit -m "chore: finish Trellis migration"
git status --short --branch
```

预期：提交成功，工作树干净，分支仍为 `master`。
