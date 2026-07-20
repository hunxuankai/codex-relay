# Codex Relay 开发规则

以下规则适用于仓库全部目录和所有开发、测试、评审、构建工作。

1. 开发和自动化测试严禁读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。测试必须使用 `tempfile`、`AppPaths::for_test` 或同时设置 `CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR`。
2. 任何测试构造器都不得在覆盖变量缺失时回退到生产路径。路径保护失败必须立即停止测试。
3. 真实 API Key、Authorization Header、完整 `auth.json`、完整 `providers.json` 不得提交到 Git，不得进入日志、通知、快照、测试输出或普通前端状态。fixture 只能使用明确的 `test-key-*-not-real`。
4. 所有配置写入必须经过应用级事务锁、外部修改指纹检查、事务备份、临时文件、解析验证、原子替换和写后验证。不得绕过 `TransactionService` 直接修改受管文件。
5. `config.toml` 必须使用 `toml_edit` 局部修改。严禁把整个 TOML 反序列化后重新生成，必须保留注释、未知字段、其他 Provider 和功能配置。
6. 不得绕过回滚或夸大回滚结果。只有全部触及文件恢复并验证成功时，用户消息才能声明“原配置已恢复”；否则必须报告恢复未完全成功并引导使用备份。
7. `providers.json` 损坏时不得静默覆盖；先保留损坏副本，再返回安全错误。
8. 备份元数据不得包含密钥，但备份文件快照可能含明文密钥，文档和界面必须如实说明。
9. 前端只能通过 `src/services/tauri.ts` 调用 `invoke`。组件使用 Vue 3 Composition API、`<script setup lang="ts">`、显式 typed props/emits、只读 composable 状态和可访问语义。
10. 新功能和缺陷修复必须先写能因预期原因失败的测试，再做最小实现。修改后运行与风险相称的 typecheck、测试、格式、Clippy 和构建。
11. 不得声称测试、构建、回滚、签名、安装或手动行为已成功，除非本轮有对应命令或人工观察证据。报告必须保留真实失败、限制和未完成项。
12. 卸载逻辑不得擅自删除 Codex 配置、应用数据、API Key、日志或备份。
13. 项目文档、设计文档、实施计划、验证报告和 README 默认使用简体中文；命令、代码标识符、文件名以及没有通行中文译名的技术专有名词可以保留原文。
