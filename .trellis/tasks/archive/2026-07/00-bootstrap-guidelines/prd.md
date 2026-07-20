# Codex Relay 全量 Trellis 迁移 PRD

## 背景

Kai 主要使用 Codex 独立开发 Codex Relay。复杂任务在长对话自动压缩后，关键决策和下一步不能只存在于聊天中；同时，项目规则不能继续全部堆入每轮都会加载的 `AGENTS.md`。

## 目标

1. 使用 Trellis `tdd` 工作流管理非平凡任务的创建、规划、实施、检查、规范更新、完成和归档。
2. 使用任务文件持续保存需求、设计、实施进度、验证证据和未解决问题，使上下文压缩后可从仓库恢复。
3. 将长期项目知识迁入按领域拆分的 `.trellis/spec/`，按任务选择加载。
4. 将 `AGENTS.md` 精简为每轮必须加载的最高优先级红线。
5. 将有用的 `docs/` 内容迁入 Trellis，删除一次性、过期或已被取代的文档，最终删除整个 `docs/`。
6. 保持 README 面向用户和开发者的启动、测试、构建及发布说明准确，并把进一步阅读入口改到 `.trellis/spec/`。

## 非目标

- 不改变 Codex Relay 产品运行行为、配置格式或安装行为。
- 不重新生成 Debug、Release 或 NSIS 产物。
- 不启用 Trellis channel 或 sub-agent dispatch。
- 不把真实 API Key、认证文件或真实用户配置写入 Trellis 任务、规范、日志或 Git。
- 不触碰 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。

## 必须满足的约束

- Trellis CLI 使用已验证的 0.6.7；初始化必须使用 `--skip-existing`，禁止 `--force`。
- Codex 使用 `dispatch_mode: inline`。
- 所有项目文档与 Trellis 任务材料默认使用简体中文。
- 配置写入、TOML 保留、回滚真实性、测试路径隔离、密钥保护和卸载数据保留等安全红线必须完整迁移。
- 本机身份、当前会话指针、缓存、日志和临时文件不得提交。
- 只有目标规范已建立并完成映射检查后，才能删除对应旧文档。

## 验收标准

1. `.trellis/workflow.md` 为 `tdd` 模板，`.trellis/config.yaml` 明确使用 Codex inline。
2. 当前任务包含 `task.json`、`prd.md`、`design.md`、`implement.md`、`implement.jsonl`、`check.jsonl` 和 `research/`。
3. `.trellis/spec/` 包含 project、security、backend、frontend、testing、release、workflow 七个领域及各自 `index.md`。
4. 仅通过当前任务文件和引用规范即可说明目标、已完成、关键决策、验证证据、下一步和未解决问题。
5. `AGENTS.md` 只保留最高优先级规则，README 不再引用 `docs/`。
6. `docs/` 完全删除，旧长期规则均能映射到新的 `AGENTS.md` 或具体 spec。
7. `task.py validate`、`trellis update --dry-run`、上下文恢复演练和 `trellis mem` 项目查询完成并记录真实结果。
8. `npm run check` 与 `git diff --check` 通过，Git 不包含机器私有状态或秘密。
