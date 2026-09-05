# 实施与验证记录

## 阶段与计划

- [x] 加载 Trellis、后端、前端、测试、安全及发布规范；确认用户实施与发布授权。
- [x] 完成 PRD、设计和计划审查；目录单一来源、真实路径禁用、无 tag push。
- [x] 行为切片：先运行当前 Provider 编辑为 `gpt-6-astra` 的失败回归，再更新内置目录并通过专项测试。
- [x] 同步 README、补丁版本和最终简体中文发布说明；核对公开 Latest。
- [x] `trellis-check` inline 审查、`npm run check`、移除签名环境变量后的 `npm run build`、实际产物枚举。
- [ ] 规范更新判断、精确暂存提交、按 `master` 已配置上游普通推送候选并核验 SHA。
- [ ] 触发并监控唯一 GitHub 发布 Run，核验 Draft 与签名资产后公开。
- [ ] 核验 Latest、公开资产、tag、历史清理；更新验收证据。
- [ ] 任务归档、会话日志、最终普通 push，确认远程跟踪分支与 HEAD 相等。

## 验证命令

- `npm run test:rust:provider-workflow`
- `npm run test:rust:lib -- provider_preference_service`
- `npx vitest run src/components/ProviderEditor.test.ts src/components/ProviderPreferenceControls.test.ts src/release-config.test.ts`
- `npm run check`
- 移除当前命令进程的两个 `TAURI_SIGNING_*` 环境变量后运行 `npm run build`。
- `git diff --check`、精确暂存差异和秘密/路径审计、发布 API 与资产哈希核验。

所有本地测试/构建命令使用临时目录下成对 Relay 覆盖，复用 `src-tauri/target`。

## 当前检查点

2026-09-06：inline 质量检查与普通 Release/NSIS 构建通过，准备提交候选。

- 红色：`npm run test:rust:provider-workflow -- editing_provider_to_gpt_6_astra_saves_and_applies_model_preferences` 退出 1；唯一失败为编辑目录缺少 `gpt-6-astra`，符合预期，编译约 63 秒。
- 绿色：`npm run test:rust:provider-workflow` 退出 0，2 项通过；新模型目录、编辑保存、默认 low、Fast、ultra 投影与未知 TOML/认证保留通过，编译约 30 秒、测试约 1 秒。
- GitHub 只读核验：Latest `v0.5.0`，Release ID `364697150`；两项 updater Secret 名称存在，未读取值。辅助研究确认 `v0.5.1` 未占用且无运行中的发布流程。
- npm 和两个 Cargo manifest/lock 已同步 `0.5.1`；前端结构测试使用动态版本，无需改写固定断言。
- 前端专项：3 个文件、27 项通过（编辑页、详情偏好、发布配置），退出 0。
- 中断记录：首次 `test:rust:lib -- provider_preference_service` 执行因会话中断失去终态输出，恢复后未声称通过；后续完整检查已重新覆盖整个 core 测试模块。
- 完整检查：`npm run check` 退出 0；Trellis 8 项，根 Vitest 60 文件/338 项，发布控制台 Vitest 17 文件/89 项，Rust 463 项通过、1 项既有完整检查嵌套探针按配置 ignored；两个前端 typecheck、Rust fmt/Clippy 和依赖图检查通过。`path_safety` 3 项、`provider_workflow` 2 项通过。
- 安全审查：改动只含固定模型元数据、假 key 测试、README 与版本/发布说明；Git 跟踪文件无真实认证文件、开发数据或构建产物。使用系统临时目录成对覆盖，未访问真实 Codex/Relay 数据。
- 规范更新判断：只扩展既有内置模型目录，没有新增 API、存储格式、事务、安装或发布契约；README 已同步实际能力，无需重复扩写长期 spec。
- 普通构建：进程内移除两个 updater 签名变量后 `npm run build` 退出 0；Rust Release 编译 7 分 46 秒，实际生成主程序与 NSIS，版本均为 `0.5.1`。Vite 保留第三方 PURE 注解和约 504 kB chunk 提示，不影响成功状态；未生成本次 `.sig`，Authenticode 实测 `NotSigned`。
- 构建产物与后续线上证据见 `release-evidence.md`。`git fetch --no-tags origin main` 后本地与上游差异为 `0/0`。
- 当前无安装、应用内升级、UAC、重启或卸载证据。
