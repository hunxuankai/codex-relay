# 实施与验证记录

## 状态

实施与完整质量门禁已完成，准备按授权提交、归档和推送。核心实施与检查采用 inline，无辅助代理。

## 有序清单

- [x] 切片一：修改既有公共流程测试，复现目录多出 `ultra`；收窄目录并验证五档可用、非法值拒绝。
- [x] 切片二：复现旧 v4 Astra `ultra` 无法读取；添加只读兼容映射，验证事务保存与备份恢复。
- [x] 补充兼容范围和严格序列化检查；同步 README 与可执行规范。
- [x] 完成 Trellis check、路径安全及完整项目门禁。
- [ ] 精确暂存并提交，归档任务、记录会话，完成普通推送及远端一致性验证。

## 验证命令

所有测试命令使用成对的安全临时 `CODEX_RELAY_CODEX_HOME` / `CODEX_RELAY_APP_DATA_DIR`，不改变 Cargo target 位置。

```powershell
npm run test:rust:provider-workflow -- gpt_6_astra
npm run test:rust:lib -- provider_preference_service
npm run test:rust:path-safety
npm run check
git diff --check
```

## 风险与边界

- `provider_preference_service.rs`：只在读取当前 v4 的精确旧值组合上兼容；新输入和序列化保持严格。
- 旧原始文件必须可从事务备份恢复；不能通过读取偷偷迁移磁盘文件。
- 本任务不构建、发布、安装或操作真实用户数据。

## 本轮证据

- 红色：`npm run test:rust:provider-workflow -- editing_provider_to_gpt_6_astra_saves_and_applies_model_preferences` 退出 101；目录实际六档、预期五档（1 失败）。
- 绿色：移除目录中的 `ultra` 后，同命令退出 0（1 通过），覆盖五档逐项保存、TOML 投影以及 `ultra`/`none` 拒绝后的四文件字节不变。
- 红色：旧偏好专项退出 101（1 失败），在 `list_providers` 返回 `INVALID_MODEL_REASONING_EFFORT`。
- 绿色：添加 v4 读取兼容后，`npm run test:rust:provider-workflow -- gpt_6_astra` 退出 0（2 通过），覆盖只读加载、事务保存为 `max`、认证保留和备份恢复原始四文件。
- `npm run test:rust:lib -- provider_preference_service` 退出 0，21 项通过；编译/链接 1 分 26 秒、执行 0.02 秒。
- `npm run test:rust:path-safety` 退出 0，3 项通过，证明默认路径哨兵不变。
- 首次 `npm run check` 退出 1：Trellis 8 项和前端类型检查通过；前端 338 项中 336 通过，`release-request` 与 `release-console-structure` 的两个 PowerShell 用例分别约 5.88/5.61 秒，超过 Vitest 默认 5 秒预算。此轮尚未执行后续发布控制台独立检查和 Rust 全套。
- 诊断：`npx vitest run src/release-request.test.ts src/release-console-structure.test.ts --maxWorkers=1` 退出 0（7 项通过）；两个 PowerShell 用例分别约 3.14/2.61 秒，支持并发启动耗时造成超时的判断。本任务未修改测试超时、worker 配置或发布脚本。
- 最终 `npm run check` 按原有配置完整重跑退出 0，覆盖 Trellis、两套前端类型检查与测试、Rust 依赖图、格式、严格 Clippy 和 workspace 测试。此前超时的两个用例分别约 2.51/2.03 秒通过；`local_verification` 中递归执行整个项目检查的既有用例仍按配置忽略，未声称运行该用例。
- `git diff --check` 通过；高置信度密钥前缀扫描无命中，Git 未跟踪真实认证文件、密钥存储或 Cargo 产物（开发数据只有 `.gitkeep`）。

## 审查结论

- 生产改动只涉及唯一模型目录与 v4 读取分支；UI 继续消费后端目录，无重复模型白名单。
- 新选择和序列化仍严格拒绝非法强度；兼容标记复用既有事务、指纹与备份机制。
- 已同步 README、产品契约和后端可执行规范；“关于”页面没有具体模型/强度清单，无需修改。
- 未发现本任务外的未提交改动；未执行构建、安装、发布或真实用户配置操作。
