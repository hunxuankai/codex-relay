# Codex Relay 开发规则

以下规则适用于仓库全部目录和所有开发、测试、评审、构建工作。

1. 非平凡开发必须使用当前 Trellis 任务保存 PRD、设计、实施进度、验证证据和下一步；没有当前任务时先按 `.trellis/workflow.md` 创建并启动任务。
2. Trellis `tdd` 工作流是任务生命周期的唯一负责人，Codex 使用 inline 模式；不要另建重复的规划、TDD、子代理派发或分支收尾流程，具体边界见 `.trellis/spec/workflow/trellis-superpowers.md`。
3. 开发和自动化测试严禁读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`；必须使用安全临时路径或成对 Relay 覆盖，路径保护失败立即停止。
4. 真实 API Key、Authorization Header、完整认证文件和密钥存储不得进入 Git、日志、通知、快照、测试输出、普通前端状态、Trellis 任务或规范；fixture 只能使用明确的 `test-key-*-not-real`。
5. 所有受管配置写入必须经过 `TransactionService` 的锁、指纹、备份、临时文件、解析、原子替换、写后验证和可验证回滚；`config.toml` 必须用 `toml_edit` 局部修改并保留未知内容。
6. 不得声称测试、构建、回滚、签名、安装或人工行为成功，除非本轮有对应命令或观察证据；必须保留真实失败、限制和未完成项。
7. 开始实施和检查前，根据当前任务从 `.trellis/spec/` 的相关 `index.md` 选择加载详细规则；安全、测试、发布和卸载边界不得因 `AGENTS.md` 精简而省略。
8. 项目文档、README、设计、实施计划、验证记录、Trellis 任务材料、`.agents` Skill/参考资料、`.codex` Agent/提示词及 Git 提交主题和正文默认使用简体中文；Conventional Commits 的 `type(scope):` 可保留英文标识符，命令、代码标识符、文件名和无通行中文译名的技术名词可保留原文。
