# 实施计划

## 会话交接

- 2026-07-29：用户已审阅并确认 `prd.md`、`design.md`、`implement.md` 与
  `research.md`。
- 用户明确要求当前会话不要实施；任务保持 `planning`，由下一 Codex 会话完成启动
  门禁后再运行 `task.py start`。

## 工作方式

- 当前任务使用 Codex inline TDD；运行 `task.py start` 前不修改业务代码。
- 每个行为切片严格执行：新增失败测试 -> 确认因目标行为缺失而失败 -> 最小实现 ->
  专项绿色 -> 重构。
- Rust 文件测试只使用 `tempfile` / `AppPaths::for_test`；如使用环境覆盖，必须成对
  设置 Relay 路径。禁止访问真实 `%USERPROFILE%\.codex` 和
  `%LOCALAPPDATA%\CodexRelay`。
- 前端只 mock typed Tauri/composable 边界；fixture 密钥只使用
  `test-key-*-not-real`。

## 当前进度

- 2026-07-29：启动门禁通过，任务状态已由 `planning` 切换为 `in_progress`。
- 实施、规范更新与 Phase 2.2 全范围质量检查已完成，准备执行提交前审计与精确提交。

## 已完成

- 完整复核 `prd.md`、`design.md`、`implement.md` 与 `research.md`。
- `task.py validate` 与 `git diff --check` 已通过；inline 模式按规则跳过 JSONL。
- 已加载阶段 1.4、2.1 细则和相关 project/backend/frontend/security/testing 规范。
- v1/v2 只读迁移到 v3，且 `fastEnabled` 默认关闭；v3 序列化稳定往返。
- v3 对“不支持模型 + Fast 开启”的损坏组合失败关闭。
- `ProviderPreference::set_fast` 只允许受支持模型开启，失败返回 `MODEL_FAST_UNSUPPORTED`。
- 详情模型选择到不支持模型时保留模型切换并自动关闭 Fast。
- 编辑模型集合回退到不支持模型时自动关闭 Fast。
- Fast 开启写入顶层 `service_tier = "fast"` 并单向确保 `[features].fast_mode = true`。
- 修复同一投影路径替换顶层标量时 `toml_edit::Table::insert` 清空 key decor 的注释丢失问题。
- Fast 关闭只删除顶层 `service_tier`，不读取或修改 `features.fast_mode`。
- 新建并立即启用 Fast Provider 时，偏好、config、auth 和 secrets 在同一事务中一致。
- `ProviderProfile.fastEnabled` 与 `ModelCatalogItem.supportsFast` 从 Rust 权威状态 typed 投影。
- 创建 Fast 但不启用只保存偏好；不支持模型请求 Fast 时零写入。
- 编辑当前 Provider 且勾选同步时，Fast 偏好和全局投影同事务更新。
- 编辑回退到不支持模型时原子关闭 Fast、保留 feature gate 并返回明确原因。
- 编辑未同步只保存偏好；不支持组合在事务前失败且四文件不变。
- 独立 Fast 动作覆盖当前/非当前、开启/关闭和不支持模型，统一经过 TransactionService。
- 详情模型动作保持合法 Fast；切到不支持模型时按当前/非当前语义原子关闭并提示原因。
- Provider 切换按目标偏好双向投影 Fast/non-Fast、模型、强度与认证。
- 独立 Fast Tauri command 只做 typed 委托、应用写守卫、通知刷新和 handler 注册。
- typed 前端 service 使用精确 `update_provider_fast` command 与 camelCase 参数委托。
- `useProviders.updateFast` 自动携带当前四文件指纹，并在 mutation 后刷新后端权威状态。
- 前端 DTO 与所有完整测试工厂已增加 `fastEnabled` / `supportsFast` 必填字段。
- 详情控件从模型目录派生 Fast 能力，支持费用提示、不支持提示、busy 禁用和单一事件上送。
- 编辑器覆盖创建默认值、编辑回填、实际偏好模型、模型回退自动关闭和现有同步选项。
- `ProvidersView` 只把详情 Fast 事件转发到 composable，不保存第二份 Fast 状态。
- v1/v2 fixture、self-check 与事务验证保持只读兼容；下一次成功用户事务写出 v3。
- Fast 专用 stale fingerprint、偏好写故障逐字节回滚和稳定备份 operation 已覆盖。
- README、About 与 project/backend/security/frontend 规范已同步 v3、Fast 投影、模型和费用契约。

## 关键决策

- 现有 `implement.md` 继续作为唯一实施清单；核心实施、检查和任务状态均由主会话 inline 完成。
- 每个行为切片保留 RED 失败证据，再进行最小实现和专项 GREEN 验证。

## 验证证据

- `python ./.trellis/scripts/task.py validate .trellis/tasks/07-29-provider-fast-service-tier`：退出码 0。
- `git diff --check`：退出码 0，无输出。
- RED：`legacy_v1_and_v2_upgrade_to_v3_with_fast_disabled` 在版本仍为 2 时按预期失败。
- GREEN：`provider_preference_service` 模块 13 项测试全部通过。
- RED：`v3_rejects_fast_for_an_unsupported_selected_model` 证明非法组合此前会被接受。
- GREEN：`provider_preference_service` 模块 14 项测试全部通过。
- RED：`fast_can_only_be_enabled_for_supported_catalog_models` 因缺少 `set_fast` 按预期编译失败。
- GREEN：该专项测试通过。
- RED：`src/services/tauri.test.ts` 因缺少 `updateProviderFast` typed service 方法按预期失败。
- GREEN：`src/services/tauri.test.ts` 共 9 项测试全部通过。
- RED：`useProviders` 新用例因缺少 `updateFast` 公开动作按预期失败，其他 15 项保持通过。
- GREEN：`src/composables/useProviders.test.ts` 共 16 项测试全部通过。
- RED/GREEN：详情 Fast 开关先因 `ElSwitch` 缺失失败；支持与不支持模型用例完成后 5 项通过。
- RED/GREEN：编辑器依次暴露控件缺失、编辑未回填、能力未按实际偏好模型、回退未关闭和 Fast 未触发同步；完成后 8 项通过。
- RED/GREEN：route view 未转发 `update-fast`；补齐无状态编排后 15 项通过。
- GREEN：前端 Fast 专项 6 个文件共 55 项测试通过；`npm run typecheck` 退出码 0。
- RED/GREEN：旧升级断言实际得到版本 3、仍期待版本 2；更新为 v3 并断言 Fast 默认关闭后通过。
- GREEN：v2 用户事务升级、事务 v1/v2 验证与根 crate self-check 只读兼容各 1 项通过。
- GREEN：`update_fast_` 6 项、备份 operation 1 项、事务序列化 1 项通过。
- GREEN：完整 `provider_service` 模块 58 项测试全部通过。
- RED/GREEN：About 测试先因缺少 v3/Fast 说明失败；补齐用户可见契约后 1 项通过。
- RED：`reconciling_to_an_unsupported_selected_model_disables_fast` 证明回退后 Fast 此前仍开启。
- GREEN：重构后的 `provider_preference_service` 模块 17 项测试全部通过。
- RED：Fast 投影测试先因函数缺少布尔参数编译失败；首次 GREEN 尝试暴露前导注释丢失。
- 调试：确认 `toml_edit::Table::insert` 对已存在 key 调用 `Key::fmt()` 并清空 decor；改用原地 Item 替换后专项通过。
- RED/GREEN：关闭 Fast 的专项测试先证明旧 tier 未删除，恢复单行删除后通过。
- GREEN：`config_service` 模块 12 项测试全部通过。
- RED：创建 Fast Provider 测试因 `CreateProviderInput.fast_enabled` 缺失而编译失败。
- GREEN：`create_and_activate_fast_projects_preference_and_global_config_atomically` 通过。
- RED：列表投影测试因缺少 `fast_enabled` / `supports_fast` DTO 字段编译失败；补齐映射后通过。
- GREEN：创建相关过滤共 10 项测试通过。
- RED：编辑同步测试因 `UpdateProviderInput.fast_enabled` 缺失而编译失败；补齐字段与服务映射后通过。
- RED/GREEN：编辑自动关闭测试先只缺原因消息，补齐后通过；编辑过滤共 10 项测试通过。
- RED：独立 Fast 测试因 DTO/方法缺失编译失败；补齐事务操作、写后验证和消息后，操作矩阵 4 项通过。
- RED/GREEN：详情合法 Fast 先触发写后验证回滚，改传真实偏好后通过；自动关闭矩阵 2 项通过。
- RED/GREEN：Fast Provider 切换先触发写后验证回滚，改传目标偏好后双向切换通过。
- RED：command adapter 缺失；一次根 crate 编译超时后重跑得到有效 RED，补齐后专项 1 项通过。
- RED：`selecting_an_unsupported_model_automatically_disables_fast` 暴露旧返回类型和缺少自动关闭。
- GREEN：该专项测试通过。
- 首次 `npm run test:rust:lib`：205 项中 204 项通过；旧事务测试仍把偏好 v3 当作未知
  版本，按预期暴露过期断言，未掩盖该失败。
- GREEN：修正版本矩阵为偏好 v1/v2/v3 可读、v4 拒绝后，事务专项 1 项通过；重新运行
  `npm run test:rust:lib`，205 项全部通过。
- `npm run test:rust:provider-workflow`：1 项通过；未知 TOML 保留与原字节回滚通过。
- `npm run test:rust:path-safety`：3 项通过；真实默认路径哨兵保持不变。
- `npm run check`：退出码 0；Trellis 8 项、前端 39 个文件 207 项、Rust 根 crate
  43 项、core 205 项、路径安全 3 项、Provider workflow 1 项全部通过；Rust 依赖图、
  `cargo fmt --check` 与 `clippy -D warnings` 通过。
- `npm run build:frontend`：退出码 0，转换 1742 个模块并生成生产前端；Rollup 对依赖
  `@vueuse/core` 的两个 PURE 注释位置给出非阻塞警告。
- `python ./.trellis/scripts/task.py validate .trellis/tasks/07-29-provider-fast-service-tier`：
  退出码 0；inline 模式没有 JSONL，验证器按契约跳过。
- `git diff --check`：退出码 0；仅报告工作树 LF 将按 Git 设置转换为 CRLF，没有空白错误。
- `git status --short --ignored`、`git ls-files` 与仓库扫描已人工复核：只有本任务目录和
  `fixtures/provider-preferences-v2.json` 是待纳入的未跟踪文件；没有跟踪真实 `auth.json`、
  `providers.json` 或 `provider-preferences.json`，高置信度密钥模式无命中。
- `OPENAI_API_KEY` / `Authorization` / `Bearer` / `apiKey` 差异复核仅命中字段名和
  `test-key-*-not-real`；新增直接 `fs::write` 只位于 `#[cfg(test)]` 或集成测试的安全路径。

## 下一步

- 完成任务校验、差异/路径/密钥审计，精确暂存本任务改动并提交；随后按
  `trellis-finish-work` 归档并记录会话。

## 尚未解决的问题

- 未执行真实 Windows 桌面人工观察；本轮只把 Vitest、Rust 测试与前端生产构建作为自动化
  证据，不据此声称桌面窗口、托盘或真实 Tauri 交互已经人工验证。

## 1. 偏好 v3 与模型能力

- [x] RED：为 `ProviderPreferenceService` 添加 v1/v2 -> v3 Fast 默认关闭、v3 round
  trip、支持模型可开 Fast、不支持模型拒绝、非法 v3 文件失败关闭测试。
- [x] GREEN：升级 `PROVIDER_PREFERENCE_VERSION`，增加显式 v2 decoder、
  `fast_enabled`、`supports_fast`、校验和自动关闭返回值。
- [x] 更新 preference fixture 和 transaction/self-check 解析测试，保留至少一个 v1 与
  v2 兼容样本。
- 公开被测边界：`parse_store`、`serialize_store`、`ProviderPreference::{from_models,
  reconcile_models,select,set_fast}`、`model_catalog`。
- 专项命令：
  `cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay-core --lib provider_preference_service`。

## 2. `config.toml` Fast 投影

- [x] RED：添加 Fast 开/关、已有/缺少 `[features]`、其他 feature/注释/未知内容保留、
  inline table、异常 features 类型返回 `INVALID_FEATURES_CONFIG` 的测试。
- [x] GREEN：扩展 `select_provider_with_preference`，建立唯一 Fast TOML 投影辅助函数。
- [x] 验证 Fast true 同时写 `service_tier = "fast"` 与 `fast_mode = true`；Fast false
  只移除 `service_tier` 且 features 字节语义不变。
- 公开被测边界：`config_service::select_provider_with_preference`。
- 专项命令：
  `cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay-core --lib config_service`。

## 3. ProviderService 行为切片

- [x] RED/GREEN：创建 Provider 默认 Fast 关闭；Fast 开启但首选模型不支持时零写入。
- [x] RED/GREEN：编辑 Provider 保存 Fast；当前 Provider 分别覆盖 sync=false/true。
- [x] RED/GREEN：新增 `update_provider_fast`，覆盖当前/非当前、开启/关闭、不支持模型、
  指纹冲突和事务失败回滚。
- [x] RED/GREEN：详情模型从支持切到不支持时自动关 Fast；验证当前/非当前文件矩阵和
  成功消息。
- [x] RED/GREEN：Fast/非 Fast Provider 双向切换，验证 Provider、模型、强度、认证、
  service tier 和 feature gate 的完整投影。
- [x] 扩展写后验证器，检查 v3 Fast 偏好以及当前 config 的 `service_tier` /
  `features.fast_mode` 不变量。
- 公开被测边界：`ProviderService::{create_provider,update_provider,
  update_provider_preference,update_provider_fast,switch_provider}`，使用真实
  `TransactionService` 与安全临时路径。
- 专项命令：
  `cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay-core --lib provider_service`；
  `npm run test:rust:provider-workflow`；`npm run test:rust:path-safety`。

## 4. Tauri command 与 typed 前端服务

- [x] RED：command adapter、`src/services/tauri.test.ts` 和 `useProviders.test.ts` 先覆盖
  新 DTO、精确 command 名、错误透传与 mutation 后权威刷新。
- [x] GREEN：增加 Rust/TypeScript `UpdateProviderFastInput`、
  `TransactionOperation::UpdateProviderFast`、command 注册、typed service 与
  `useProviders.updateFast`。
- [x] 更新所有 ProviderProfile/ModelCatalogItem 测试工厂，禁止用类型断言掩盖缺字段。
- mock 边界：Tauri command 测试使用测试 `AppState`；前端 service/composable 测试只
  mock `invoke` 或 typed relay client。

## 5. Vue 详情与编辑交互

- [x] RED：`ProviderPreferenceControls.test.ts` 覆盖支持模型、费用提示、不支持提示、
  `aria-describedby`、busy、toggle emit 和模型切换后的后端权威结果。
- [x] GREEN：在现有偏好组件增加 `ElSwitch` 与单一 `update-fast` emit，不在组件中
  硬编码模型 ID。
- [x] RED：`ProviderEditor.test.ts` 覆盖 create 默认关闭、edit 回填、实际偏好模型计算、
  不支持时自动关闭/禁用、Fast 变化触发现有 sync 选项和提交 DTO。
- [x] GREEN：增加 draft/computed/watch 和表单控件；保持 props down/events up。
- [x] RED/GREEN：`ProvidersView.test.ts` 覆盖详情动作编排和编辑提交，不让 route view
  持有第二份 Fast 状态。
- 专项命令：
  `npm run test -- src/components/ProviderPreferenceControls.test.ts src/components/ProviderEditor.test.ts src/views/ProvidersView.test.ts src/composables/useProviders.test.ts src/services/tauri.test.ts`；
  `npm run typecheck`。

## 6. 文档、规范与关于页

- [x] 更新 `README.md` 的 `config.toml`、provider-preferences v3、Provider 操作和事务
  说明。
- [x] 更新 `AboutView.vue` / `AboutView.test.ts`，说明 Fast 私有偏好、全局投影、模型
  限制与费用影响。
- [x] 更新 `.trellis/spec/project/{product-contract,architecture,
  provider-multi-credentials}.md`、backend service 边界、security 事务和 frontend
  状态/交互契约；保留官方来源链接或指向任务 research。
- [x] 搜索残留的 `provider-preferences.json v2`、Provider DTO 工厂和旧投影字段列表，
  区分需要升级的当前契约与必须保留的历史资料。

## 7. 质量门禁

按风险从专项扩大到完整检查；保留首次失败、超时和重试证据：

```powershell
npm run check:rust-dev-env
npm run test -- src/components/ProviderPreferenceControls.test.ts src/components/ProviderEditor.test.ts src/views/ProvidersView.test.ts src/composables/useProviders.test.ts src/services/tauri.test.ts src/views/AboutView.test.ts
npm run typecheck
npm run test:rust:lib
npm run test:rust:provider-workflow
npm run test:rust:path-safety
npm run check:frontend
npm run check:rust
npm run check
npm run build:frontend
python ./.trellis/scripts/task.py validate .trellis/tasks/07-29-provider-fast-service-tier
git diff --check
```

提交前另做：

- `git status --short --ignored` 与暂存差异审计。
- 搜索真实认证文件名、高置信度密钥、`Authorization` / `Bearer` 命中并人工复核。
- 确认测试路径未回退到真实 `.codex` 或 Relay 应用数据。
- 如未执行安全 Tauri 桌面人工观察，明确报告；不得把 Vitest 或 frontend build 说成
  Windows 桌面交互已验证。

## 风险与回滚点

- **最高风险：偏好 v3。** v1/v2 只读迁移必须先有 fixture 回归；开始用户事务前保留
  完整备份。旧版降级会拒绝 v3，需要在 README 说明。
- **高风险：全局 TOML 投影。** 所有入口复用一个投影函数，专项验证注释/未知内容
  和 `fast_mode` 单向语义；不得在组件或 command 重复拼 TOML。
- **高风险：模型切换自动关闭。** 必须由后端在同一事务决定，前端禁用只是提示层，
  不能成为唯一校验。
- **回滚。** 运行时失败由 TransactionService 恢复精确原字节/存在状态；代码回滚按
  行为切片提交边界撤销，不手工降写用户 v3 文件。

## 启动前门禁

- [x] 用户已审阅最终 `prd.md`、`design.md`、`implement.md`。
- [x] `task.py validate` 与 `git diff --check` 通过。
- [x] 运行 `task.py start` 后再加载 `trellis-before-dev` 和相关 Vue/Rust/安全规范。
- [x] 不创建重复的 Superpowers 计划、TDD、写入型子代理或分支收尾流程。
