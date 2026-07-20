# 设计：复核内部文档中文化任务完成度

## 审计边界

本任务先执行独立完成度审计。用户于 2026-07-21 明确授权后，任务范围扩展为修复审计发现的任务脚手架中文化与持久测试缺口；原审计结论和红色证据必须保留，不改写已归档任务历史。

审计覆盖五个维度：

1. 原 PRD 的 8 条需求与 6 条验收标准。
2. `.agents`、`.codex`、`.trellis` 和 `AGENTS.md` 的当前受管内容。
3. 三笔工作提交及后续归档、会话提交的实际差异和顺序。
4. workspace、PRD 和自动提交等生成式行为。
5. 原任务验证声明的可重复性、安全边界和限制。

## 证据优先级

结论按以下顺序取证：

1. 当前文件、当前 Git 对象和本轮可重复命令。
2. 在安全临时目录或 mocked Git 边界得到的行为观察。
3. 原任务提交差异与归档材料。
4. 原任务中的文字声明和验收勾选。

较低级证据不能覆盖较高级证据。原任务声称通过、但本轮失败的项目按本轮结果定级，并同时保留旧声明与新失败。

## 结论模型

每个原需求和验收项使用以下状态：

- **通过**：当前证据完整覆盖要求，没有实质性缺口。
- **部分通过**：主体完成，但存在可复现遗漏、范围缺口或证据不足。
- **未通过**：核心行为仍不满足。
- **无法验证**：安全边界、环境或缺少可观察入口导致无法取得可信证据。

发现按严重程度排序：

- **高**：长期规则会持续被生成器或运行行为破坏，或存在安全/密钥风险。
- **中**：明确违反原需求，但影响局限或有人工绕行方式。
- **低**：不影响核心目标的一致性、可维护性或证据缺口。

总体完成度不是简单按文件数量计算：任何“后续生成会重新产生英文”的可复现缺陷都会使“防止回退”相关要求至少降为部分通过。

## 验证方法

- 从 Git 生成受管文件清单，扫描 fenced block、inline code、URL 和链接目标之外的高置信度英文候选，再人工分类。
- 对 Skill frontmatter、Markdown 围栏/相对链接、TOML/YAML/JSON/Python 和工作流阶段解析入口做结构验证。
- 从干净临时目录调用公开或稳定的生成边界，验证首次生成、连续更新和旧英文标签兼容；禁止使用真实用户路径。
- 用 mocked Git 或已产生的真实提交对象验证会话日志与归档中文主题，不执行 push、amend 或历史重写。
- 运行 `task.py validate`、`git diff --check` 和 `npm run check`，并把测试数量与限制写入报告。

## 安全与回滚

- 不调用会读取真实 `~/.codex/sessions` 的 `trellis mem`。
- 不启动普通 `npm run dev`，不读取真实认证文件，不枚举真实 Codex/Relay 数据目录。
- 临时测试只使用 `TemporaryDirectory`；如需环境覆盖，Relay 的两个路径必须成对设置。
- 实施只修改 `.trellis/scripts/common/task_store.py`、`.trellis/scripts/add_session.py`、新增的 `.trellis/scripts/tests/`、`package.json`、相关 workflow spec 和当前任务目录。
- 回滚时按行为切片分别回退测试接入、测试文件、两个生成文本常量和 session 所有权 helper；不得操作并行任务文件。

## 修复设计

### 任务脚手架契约

- `_default_prd_content(title, description)` 输出 `目标 / 需求 / 验收标准 / 说明` 四个中文二级标题。
- `title` 与非空 `description` 原样进入输出；空描述仍使用结构占位符 `TBD`，避免改变调用契约。
- `_SEED_EXAMPLE` 的解释性文字使用中文；JSON 示例中的 `file`、`reason`、`<path>`、`<why>` 和 `get_context.py --mode packages` 命令保持不变。

### 持久测试边界

- 使用标准库 `unittest`，不增加第三方 Python 依赖。
- 任务创建通过 `task.py create` 的公开 CLI 在 `TemporaryDirectory` 中验证；临时根目录内创建最小 `.trellis/.developer` 与配置，不访问真实用户路径。
- workspace 生成/更新直接调用稳定模块边界并写入 `TemporaryDirectory`。
- Git 提交只 mock `run_git` / `safe_git_add` 等系统边界，断言提交主题和传入的 Trellis 所属路径；不操作真实 Git index。
- 在 `package.json` 增加 `test:trellis`，并纳入 `npm run check`，使回归测试成为完成门禁。

### 并行会话提交隔离

- `session-fallback` 继续服务缺少会话 ID 的只读上下文恢复，不修改共享解析器语义。
- `add_session.py` 的任务元数据推断和任务目录暂存属于写入/提交边界，只接受 `source_type == "session"` 的精确会话任务。
- 当前 context 文件不存在、解析结果来自 `session-fallback` 或任务已 stale 时，按“当前任务未知”处理：package/branch 不从该任务推断，自动提交只暂存 workspace 日志和索引。
- 回归测试在 `TemporaryDirectory` 创建两个逻辑 session：当前已归档 session 没有状态文件，另一 session 指向脏任务；断言后者不进入 `safe_git_add`。
