# 同步当前 Codex Provider 连接实施计划

> **面向执行 Agent：** 使用 `superpowers:executing-plans`，按行为切片执行。每一项均以失败测试开始，确认失败后才写最小实现；本任务使用 Codex inline 模式，不派发实现或检查子 Agent。

**目标：** 在不改变顶层 `model_provider` 的前提下，将一个已选 Relay Provider 的 Base URL 和 API Key 事务性应用到当前 Codex Provider 身份，并提供安全、可验证的显式恢复。

**架构：** `provider-preferences.json` v4 保存不含秘密的连接关系与首次覆盖恢复点；`ProviderService` 在同一四文件事务内解析、应用、恢复和投影该关系。Rust 负责全部资格判定、密钥解析、关系失效判定和写后验证；Vue 只消费安全 DTO，展示确认和触发 typed command。

**技术栈：** Rust、Tauri 2、`toml_edit`、serde、Tokio、Vue 3 Composition API、TypeScript、Element Plus、Vitest。

---

## 当前进度

任务已于 2026-07-31 激活；Task 1 至 Task 7 的实现、用户文档、全量 Rust/前端、路径、秘密与提交前门禁均已完成，正在同步代码规范并准备精确提交。

## 已完成

- 用户已审阅并批准 PRD、技术设计和本实施计划。
- 已加载后端、前端、安全、测试、工作流、跨层和复用规范，以及 Vue Composition API 参考。
- 已确认现有 `TransactionService` 已覆盖四个受管文件的锁、指纹、备份、原子写入、解析和回滚，不新增直接写路径。
- 已完成 v3 → v4 的只读迁移：旧文件加载到内存 v4 且 `needs_upgrade=true`，不在读取路径写盘。
- 已完成 v4 `connectionOverride` 的可选 JSON 往返，关系只持有目标/来源 Provider 与四个条目 ID。
- 已完成 v4 结构校验：目标/来源必须不同，四个引用 ID 必须非空且已规范化；跨文件归属和值匹配仍由后续 `ProviderService` 负责。
- 已完成 `ProviderProfile.connection` 默认安全投影与 `routed` URL/Key 状态枚举；当前 resolver 尚未产生路由状态。
- 已完成有效 A<-B 关系的只读投影：目标身份显示 `routed` 与恢复动作，来源显示已应用或其选择变更后的更新动作，且列表读取不改写四个受管文件。
- 已完成外部认证改写后的失效投影：目标身份保持可恢复的 `stale` 关系，来源显示安全失效状态；读取路径不修复、不迁移、不清除关系。
- 已完成无覆盖时普通完整非当前 Provider 的 `apply` action；有效关系会覆盖来源为 `applied`/`update`，失效关系阻止新的应用动作。
- 已完成引用条目缺失时的 `stale` 投影：关系保留，恢复点完整时仍可恢复；其他完整来源显示禁用的 `apply` 入口与安全恢复提示。
- 已完成首次应用连接事务：公开输入只含来源 ID 与四文件指纹，事务仅修改当前目标的 `base_url`、当前认证和 v4 关系；写后验证三者一致且顶层身份未变。
- 已完成显式恢复事务：恢复点完整时在同一事务恢复目标 Provider 的 URL、当前认证并清除关系；新增备份审计 `restore_current_provider` 操作名。
- 已完成连接更新事务：有效关系下更新来源或来源已选条目时沿用首次目标恢复 ID，只替换来源与已应用 ID；失效关系拒绝更新。
- 已完成普通切换复原：`SwitchProvider` 事务复用统一恢复点解析，先复原旧目标块，再应用新 Provider 及认证并清除关系。
- 已验证应用连接在 preferences 前向写失败时由 `TransactionService` 恢复四个受管文件的原始字节，并返回 `TRANSACTION_FAILED_ROLLED_BACK`。
- 已完成覆盖目标的 Base URL/API Key 独立选择锁定，两条入口均在事务前返回 `PROVIDER_CONNECTION_TARGET_LOCKED` 且四文件不变。
- 已完成覆盖目标的 URL/Key 批量管理和当前认证导入锁定；来源已应用 URL/Key 按稳定 ID 保护值、允许重命名，来源 Provider 禁止删除。
- 已覆盖外部顶层切换后的仅目标块恢复、恢复条目缺失时保留关系、相同连接重复提交零写入，以及完整 ProviderService 回归。
- 已完成 routed 当前身份的可用性目标与失效关系拒绝：目标模型保持身份自身偏好，URL/Key 来自实际连接来源，失效关系在网络前返回稳定错误。
- 已完成扩展自检：有效 routed 连接沿用稳定检查 ID 报告来源地址/认证已纳管，任一失效连接关系追加安全错误，两个读取路径均逐字节保持四个受管文件不变。
- 已完成 apply/restore 两个 Tauri typed command，command 只做一次服务委托并复用应用写入 guard、托盘刷新和统一安全 `CommandResult`；两个 handler 均已注册。
- 已完成 TypeScript 的 `routed` 状态、无密连接投影与两个安全输入类型；typed service 精确使用 `apply_provider_connection` / `restore_provider_connection` 和 `{ input }` 包装。
- 已完成 `useProviders.applyConnection` / `restoreConnection`，复用现有 busy、四文件指纹、稳定错误、成功文案和 mutation 后权威刷新；普通 composable 状态不持有 URL/Key 值。
- 已为前端测试 fixture 提供共享 `providerConnection` 工厂，使必填连接 DTO 在类型检查中保持完整，并支持后续 active/stale/action 场景。
- 已完成 Provider 卡片四种连接动作、文字状态与可访问名称；动作资格和禁用原因只消费 Rust 安全投影。
- 已完成应用、更新和恢复专用结构化确认框；摘要只含 Provider/条目安全名称，默认聚焦取消，取消不发 IPC。
- 已完成 routed 当前身份的详情投影：显示来源 Provider 与已应用条目名称，锁定自身 URL/Key 选择和管理入口，并提供显式恢复提示。
- 已修复窄窗口下全局 `ElButton` 100% 宽度把拖动手柄撑满并将 Provider 名称压缩为 0 宽的问题；列表头紧凑按钮和 36px 拖动手柄现在保留稳定尺寸。
- 已补充确认框父级卸载时的焦点回归；当前 Element Plus `ElFocusTrap` 已负责把焦点还给触发入口，因此没有叠加第二套手工焦点恢复。
- 已修复来源 Key 与新目标某个 Key 值相同的普通切换：新目标继续保存自己的 `selectedApiKeyId`，不按值误写来源条目 ID。
- 已修复 stale 关系下从托盘/普通入口切回原目标时的顺序：先恢复目标 URL，再按恢复后的状态验证和应用，避免对覆盖 URL 提前失败。
- 已把静默回滚后的四文件逐字节及存在状态验证纳入 `TransactionService`；验证失败返回 `ROLLBACK_INCOMPLETE` 并保留事务标记。
- 已禁止删除仍被关系引用的来源或目标 Provider，列表与详情页同步禁用对应入口。
- 已修复 routed 身份执行 `update_provider(syncIfActive=true)` 时把目标自身 Key 写回当前认证的问题；有效来源认证和关系保持不变。
- 已修复“创建 Provider 并立即启用”流程：同一事务先恢复原覆盖目标并清除关系，再应用新 Provider。

## 关键决策

- 连接关系升级为 `provider-preferences.json` v4 的可选 `connectionOverride`，只存 Provider 与条目稳定 ID，不复制 URL 或 API Key。
- 应用复用现有 `SyncCurrentProvider` 事务审计操作；显式恢复新增 `RestoreCurrentProvider` 操作。
- Rust 产生完整的连接 action/status/禁用原因/安全名称投影；Vue 不从 URL、Key 状态或 Provider ID 组合推导动作。
- 当前 `model_provider` 保持不变；正常切换会在同一事务先复原被覆盖目标。

## 验证证据

- 已观察 v3 迁移测试在旧实现上以 `left: 3, right: 4` 失败；修正测试原始字符串后确认该红灯属于目标行为。
- `npm run test:rust:lib -- provider_preference_service::tests::v3_loads_as_pending_v4_upgrade_without_writing`：1 通过。
- `npm run test:rust:lib -- provider_preference_service::tests::v4_connection_override_round_trips_stable_reference_ids`：1 通过。
- `npm run test:rust:lib -- provider_preference_service::tests::v4_rejects_structurally_invalid_connection_overrides`：1 通过。
- `npm run test:rust:lib -- provider_preference_service::tests`：20 通过。
- `npm run test:rust:lib -- provider::tests::routed_connection_statuses_round_trip_as_safe_enum_values`：1 通过。
- `npm run test:rust:lib -- provider::tests`：3 通过。
- 已观察来源选择变化测试在旧投影上以 `Applied != Update` 失败；`npm run test:rust:lib -- provider_service::tests::list_marks_changed_source_selection_as_connection_update_without_writing`：1 通过。
- 已观察外部认证改写测试在旧 resolver 上以 `None != Identity` 失败；`npm run test:rust:lib -- provider_service::tests::list_marks_external_auth_connection_as_stale_without_writing`：1 通过。
- 已观察普通来源 action 测试在旧投影上以 `None != Apply` 失败；`npm run test:rust:lib -- provider_service::tests::list_projects_complete_non_current_provider_as_connection_apply_source`：1 通过。
- 已观察来源已应用密钥缺失测试在旧 resolver 上以 `None != Identity` 失败；`npm run test:rust:lib -- provider_service::tests::list_marks_missing_applied_source_key_as_stale_without_writing`：1 通过。
- 已观察失效关系下其他来源测试在旧投影上以 `None != Apply` 失败；`npm run test:rust:lib -- provider_service::tests::list_disables_other_connection_sources_while_a_connection_is_stale`：1 通过。
- `npm run test:rust:lib -- provider_service::tests::list_`：8 通过。
- `npm run test:rust:lib -- provider_service::tests::v2_projection`：1 通过。
- 已观察首次应用连接测试因缺少公开输入和服务方法而编译失败；`npm run test:rust:lib -- provider_service::tests::apply_connection_keeps_model_provider_and_records_first_restore_point`：1 通过。
- 已观察显式恢复测试因缺少公开输入和服务方法而编译失败；`npm run test:rust:lib -- provider_service::tests::restore_connection_restores_first_target_selection_and_clears_relation`：1 通过。
- 修正测试夹具顺序后，已观察 B→C 更新测试以 `PROVIDER_CONNECTION_OVERRIDE_STALE` 失败；`npm run test:rust:lib -- provider_service::tests::update_connection_preserves_first_restore_point_across_b_then_c`：1 通过。
- 已观察覆盖期间普通切换测试以旧目标仍保留 B URL 失败；`npm run test:rust:lib -- provider_service::tests::switch_restores_overridden_target_before_selecting_new_provider`：1 通过。
- `npm run test:rust:lib -- provider_service::tests::apply_preference_write_failure_rolls_back_all_managed_files`：1 通过。
- 已观察目标 URL 与 Key 选择测试在旧实现上分别成功写入并使关系失效；对应 `routed_target_*_selection_is_locked_without_writing` 专项测试各 1 通过。
- 目标批量管理与认证导入三个专项测试均先观察到旧实现成功写入，再以 `PROVIDER_CONNECTION_TARGET_LOCKED` 转绿。
- 来源 URL/Key 值替换测试均先观察到旧实现写入并使关系失效，再以 `PROVIDER_CONNECTION_ENTRY_IN_USE` 转绿；来源删除测试先观察到旧实现删除 B，再以 `PROVIDER_CONNECTION_SOURCE_DELETE_FORBIDDEN` 转绿。
- `npm run test:rust:lib -- provider_service::tests::applied_source_entries_can_be_renamed_without_breaking_connection`：1 通过。
- `npm run test:rust:lib -- provider_service::tests::restore_after_external_provider_switch_preserves_current_auth`：1 通过。
- `npm run test:rust:lib -- provider_service::tests::restore_keeps_relation_when_restore_entry_is_unavailable`：1 通过。
- 完整回归首次发现 2 个旧测试仍期待 preferences v3；核对 v4 常量与迁移契约后仅更新断言，两个专项转绿。
- `npm run test:rust:lib -- provider_service::tests`：81 通过。
- `npm run test:rust:provider-workflow`：1 通过。
- 已观察有效 routed 自检测试因 `ProviderBaseUrlStatus::Routed` 与 `ProviderApiKeyStatus::Routed` 未被穷尽处理而编译失败；补充来源纳管分支后专项 1 通过。
- 已观察外部切换顶层 Provider 后的失效关系自检仍返回 `Normal`；增加全局 stale 投影检查后 routed/stale 两个自检专项均通过。
- 已观察 apply/restore command 测试分别因缺少 `apply_provider_connection_inner` / `restore_provider_connection_inner` 而编译失败；接入薄 command 边界后两个专项分别通过。
- `npm run test:rust:lib -- provider_service::tests::availability`：7 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay --lib commands::tests`：13 通过。
- `cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay --lib services::self_check_service::tests`：9 通过。
- 已观察 typed service 测试以 `applyProviderConnection is not a function` 失败；接入两个安全 `{ input }` 调用后 `src/services/tauri.test.ts` 10 项通过。
- 已观察 composable 测试以 `providers.applyConnection is not a function` 失败；接入共享 mutation 后 `src/composables/useProviders.test.ts` 18 项通过。
- 首次 `npm run typecheck` 精确发现 9 个旧 `ProviderProfile` fixture 缺少必填 `connection`；复用测试工厂补齐后 `vue-tsc` 通过。
- `npm test -- --run src/services/tauri.test.ts src/composables/useProviders.test.ts`：2 个文件、28 项通过。
- `npm run typecheck`：通过。
- 已观察窄窗口样式契约在修复前以缺少紧凑控件覆盖失败；`npm test -- --run src/element-plus-layout.test.ts` 修复后 11 项通过。
- `npm test -- --run src/element-plus-layout.test.ts src/components/ProviderConnectionConfirmDialog.test.ts src/components/ProviderList.test.ts src/components/ProviderStatus.test.ts src/components/ProviderEndpointControls.test.ts src/components/ProviderCredentialControls.test.ts src/views/ProvidersView.test.ts`：7 个文件、49 项通过。
- Task 6 修改后再次运行 `npm run typecheck`：通过。
- 在仅存在于页面内存的安全 IPC 桩上实际观察 900×620 和 560×720：routed 目标/来源状态、锁定控件、应用与恢复摘要均可读，页面无横向溢出，按钮文本无裁切；560×720 首次观察到卡片标题裁切并按红绿循环修复，复查时拖动手柄 36px、名称按钮 464px 且二者不重叠。
- 恢复确认后实际观察目标自身 URL/Key 恢复、管理控件解锁和恢复动作消失；页面内存桩没有完整重置来源 role，因此不把桩中的来源状态字样作为后端恢复证据，关系清除仍以 Rust/组件专项回归为准。
- 本机 Windows TCP 排除范围包含默认 Tauri 开发端口 1420-1421，未调整系统端口、未启动或操作已安装的 `CodexRelay.exe`；人工界面检查使用 Vite `http://127.0.0.1:4173/` 与安全页面内存桩完成。
- 已观察 AboutView 文档测试在旧内容上同时因 `provider-preferences.json` 仍显示版本 3、缺少“保持 model_provider”而 2 项失败；更新用户契约后 `npm test -- --run src/views/AboutView.test.ts`：2 项通过。
- Task 7 文档修改后运行 `npm run typecheck`：通过。
- 实际观察关于页新文案包含 v4、来源已选组合、首次恢复点、普通切换复原和明文边界；900×620 与 560×720 的 DOM 几何检查均无横向溢出或视口外元素。900×620 整页截图因页面较长触发浏览器 5 秒截屏超时，改用当前视口截图与几何检查，不将该次整页截图记为成功。
- `npm run test:rust:provider-workflow`：1 项通过，验证未知 TOML 保留和原始字节恢复。
- `npm run test:rust:path-safety`：3 项通过，默认 `.codex` 与 CodexRelay 路径哨兵保持不变。
- `npm run check:rust`：退出 0；依赖图、`cargo fmt --check`、Clippy `-D warnings` 通过，根 crate 47 项、core 243 项、路径 3 项和 Provider workflow 1 项通过。
- `npm run check`：退出 0；Trellis 8 项、前端 typecheck 与 40 个文件/230 项、完整 Rust 门禁全部通过。
- 仓库高置信度密钥前缀扫描无命中；`test-key-` 命中均符合 `test-key-*-not-real` fixture/断言约定，未跟踪真实 `auth.json`、`providers.json` 或备份数据。
- `git diff --check`：退出 0，仅报告预期的 LF→CRLF 工作树提示，无空白错误。
- 尚未声明构建、安装、签名或真实用户配置行为成功。

## 下一步

1. 完成 v4 连接覆盖、恢复和事务易错点的代码规范同步。
2. 重新运行最终 diff/秘密扫描并核对精确暂存清单。
3. 提交功能改动，随后归档任务并记录会话日志。

## 尚未解决的问题

无阻塞性产品决策；实现中遇到的现有契约冲突将先记录并返回规划阶段修订。

## 文件边界

| 文件 | 职责 |
| --- | --- |
| `src-tauri/crates/codex-relay-core/src/services/provider_preference_service.rs` | v4 `connectionOverride` 的解析、校验、延迟升级与序列化。 |
| `src-tauri/crates/codex-relay-core/src/models/provider.rs` | 无密 DTO、连接 action/status/role、应用与恢复输入。 |
| `src-tauri/crates/codex-relay-core/src/models/transaction.rs`、`services/backup_service.rs` | 恢复操作审计名和备份元数据映射。 |
| `src-tauri/crates/codex-relay-core/src/services/provider_service.rs` | 关系解析、投影、应用/恢复事务、普通切换复原、引用条目保护。 |
| `src-tauri/crates/codex-relay-core/src/services/provider_availability_service.rs`、`src-tauri/src/services/self_check_service.rs` | 有效路由的测试目标和自检状态。 |
| `src-tauri/src/commands/provider_commands.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/commands/mod.rs` | 两个 Tauri command、应用写锁、注册和安全边界测试。 |
| `src/types/provider.ts`、`src/services/tauri.ts`、`src/composables/useProviders.ts` | TypeScript DTO、typed IPC 和共享 mutation/fingerprint 刷新。 |
| `src/components/ProviderConnectionConfirmDialog.vue` | 仅显示安全摘要的专用连接确认框。 |
| `src/components/ProviderList.vue`、`ProviderStatus.vue`、`ProviderEndpointControls.vue`、`ProviderCredentialControls.vue`、`src/views/ProvidersView.vue` | 卡片动作、状态文字、路由详情、目标控件锁定与确认编排。 |
| 对应 Rust/Vitest 测试、`README.md`、`src/views/AboutView.vue` | 行为回归、用户契约与 v4 文件说明。 |

## Task 1：持久关系与公开类型

**文件：**
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_preference_service.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/models/provider.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/models/transaction.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/services/backup_service.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_preference_service.rs` 的单元测试
- 修改：`src-tauri/crates/codex-relay-core/src/models/provider.rs` 的单元测试
- 创建：`fixtures/provider-preferences-v1.json`
- 创建：`fixtures/provider-preferences-v3.json`

- [x] **Step 1: 先写 v4 迁移和无密 DTO 的失败测试。**

```rust
#[test]
fn v1_v2_v3_are_read_only_upgraded_to_v4_without_connection_override() {
    for fixture in [
        include_bytes!("../../../../../fixtures/provider-preferences-v1.json").as_slice(),
        include_bytes!("../../../../../fixtures/provider-preferences-v2.json").as_slice(),
        include_bytes!("../../../../../fixtures/provider-preferences-v3.json").as_slice(),
    ] {
        let loaded = parse_store(fixture).unwrap();
        assert_eq!(loaded.store.version, 4);
        assert!(loaded.store.connection_override.is_none());
        assert!(loaded.needs_upgrade);
    }
}

#[test]
fn v4_rejects_same_target_source_and_empty_reference_ids() {
    let error = parse_store(br#"{\"version\":4,\"providers\":{},\"connectionOverride\":{\"targetProviderId\":\"provider-a\",\"sourceProviderId\":\"provider-a\",\"appliedBaseUrlId\":\"\",\"appliedApiKeyId\":\"key-b\",\"restoreBaseUrlId\":\"url-a\",\"restoreApiKeyId\":\"key-a\"}}"#).unwrap_err();
    assert_eq!(error.code(), "INVALID_PROVIDER_PREFERENCES");
}

#[test]
fn provider_profile_serialization_excludes_api_key_when_connection_is_present() {
    let json = serde_json::to_string(&profile_with_active_connection()).unwrap();
    assert!(json.contains("\"connection\""));
    assert!(!json.contains("test-key-a-not-real"));
}
```

运行：

```powershell
npm run test:rust:lib -- provider_preference_service::tests provider::tests
```

预期：新 v4 类型、`routed` 状态和连接 DTO 尚不存在，测试失败。

- [x] **Step 2: 实现 v4 store 和公开无密契约。**

```rust
pub struct ProviderConnectionOverride {
    pub target_provider_id: String,
    pub source_provider_id: String,
    pub applied_base_url_id: String,
    pub applied_api_key_id: String,
    pub restore_base_url_id: String,
    pub restore_api_key_id: String,
}

pub struct ApplyProviderConnectionInput {
    pub source_provider_id: String,
    pub expected_files: FileSetFingerprint,
}

pub struct RestoreProviderConnectionInput {
    pub expected_files: FileSetFingerprint,
}
```

将偏好版本升至 4，加入可选 `connectionOverride`、v3 解析结构和规范化校验；旧版本在内存升级后保持 `needs_upgrade=true`，仅成功用户事务写出 v4。新增 `ProviderConnectionProjection` 及 `apply | applied | update | restore | null` 枚举，把 `ProviderBaseUrlStatus`、`ProviderApiKeyStatus` 扩展为 `Routed`。保留现有 `SyncCurrentProvider` 审计操作作为应用操作，并新增 `RestoreCurrentProvider` 和对应备份 operation name。

- [x] **Step 3: 运行 Task 1 测试并检查秘密边界。**

```powershell
npm run test:rust:lib -- provider_preference_service::tests provider::tests backup_service::tests
```

预期：v1/v2/v3 延迟迁移、v4 往返和 DTO 序列化通过；断言文本只使用 `test-key-*-not-real`。

## Task 2：连接关系投影与读路径

**文件：**
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_service.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/models/provider.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_service.rs` 的单元测试

- [x] **Step 1: 写入 active、stale 与普通来源卡片 action 的失败测试。**

```rust
#[test]
fn list_projects_active_connection_as_routed_identity_and_source() {
    let state = service_with_active_a_routed_to_b().list_providers().unwrap();
    assert_eq!(state.providers[0].base_url_status, ProviderBaseUrlStatus::Routed);
    assert_eq!(state.providers[0].connection.as_ref().unwrap().action, Some(ProviderConnectionAction::Restore));
    assert_eq!(state.providers[1].connection.as_ref().unwrap().action, Some(ProviderConnectionAction::Applied));
}

#[test]
fn list_marks_changed_source_selection_as_update_without_rewriting_files() {
    let (paths, service) = service_with_active_a_routed_to_b_and_new_b_selection();
    let before = four_managed_file_bytes(&paths);
    let state = service.list_providers().unwrap();
    assert_eq!(state.providers[1].connection.as_ref().unwrap().action, Some(ProviderConnectionAction::Update));
    assert_eq!(four_managed_file_bytes(&paths), before);
}

#[test]
fn list_marks_external_url_auth_or_top_level_change_as_stale_without_writing() {
    let (paths, service) = service_with_stale_connection_after_external_auth_change();
    let before = four_managed_file_bytes(&paths);
    assert_eq!(service.list_providers().unwrap().providers[0].connection.as_ref().unwrap().status, ProviderConnectionStatus::Stale);
    assert_eq!(four_managed_file_bytes(&paths), before);
}
```

运行：

```powershell
npm run test:rust:lib -- provider_service::tests::list_
```

预期：连接 resolver、`routed` 和 `connection` 投影尚不存在，测试失败。

- [x] **Step 2: 以单一 resolver 实现关系判定。**

在 `ProviderService` 内从一致磁盘快照解析 `ProviderConnectionOverride`，校验：目标仍在配置中、来源与目标不同、六个条目 ID 仍属于正确 Provider、当前顶层身份、目标实际 URL、当前认证与应用来源值均一致。将结果建模为无关系、有效关系、失效但可恢复关系，且读取路径绝不修复、升级或清除文件。

`list_state_from_disk` 必须把有效目标投影为 `routed`，显示来源名称和应用条目名称，给目标 `restore` action、给来源 `applied` 或 `update` action、给完整其他来源 `apply` action。所有禁用原因由 Rust 生成；关系失效时阻止新应用并保留恢复状态与安全原因。

- [x] **Step 3: 运行投影回归。**

```powershell
npm run test:rust:lib -- provider_service::tests::list_ provider_service::tests::v2_projection
```

预期：状态刷新不写四个文件，普通 DTO JSON 不包含 API Key。

## Task 3：应用、更新、恢复与普通切换事务

**文件：**
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_service.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/services/transaction_service.rs` 的测试辅助代码（仅在现有注入点不足时）
- 修改：`src-tauri/crates/codex-relay-core/tests/provider_workflow.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_service.rs` 的单元测试

- [x] **Step 1: 写应用/恢复的失败行为测试。**

```rust
#[tokio::test]
async fn apply_connection_keeps_model_provider_and_restores_first_target_selection() {
    let (paths, service) = service_with_a_and_b();
    service.apply_provider_connection(apply_input(&service, "provider-b")).await.unwrap();
    assert_eq!(current_provider_id(&paths), "provider-a");
    assert_eq!(current_auth_key(&paths), "test-key-b-not-real");
    service.restore_provider_connection(restore_input(&service)).await.unwrap();
    assert_eq!(current_auth_key(&paths), "test-key-a-not-real");
    assert!(load_preferences(&paths).connection_override.is_none());
}

#[tokio::test]
async fn update_connection_keeps_first_restore_point_across_b_then_c() {
    let (paths, service) = service_with_a_b_and_c();
    service.apply_provider_connection(apply_input(&service, "provider-b")).await.unwrap();
    service.apply_provider_connection(apply_input(&service, "provider-c")).await.unwrap();
    service.restore_provider_connection(restore_input(&service)).await.unwrap();
    assert_eq!(current_base_url(&paths, "provider-a"), "https://provider-a.example.test/v1");
    assert_eq!(current_auth_key(&paths), "test-key-a-not-real");
}

#[tokio::test]
async fn switch_restores_overridden_target_before_selecting_new_provider() {
    let (paths, service) = service_with_a_b_and_c();
    service.apply_provider_connection(apply_input(&service, "provider-b")).await.unwrap();
    service.switch_provider("provider-c").await.unwrap();
    assert_eq!(current_base_url(&paths, "provider-a"), "https://provider-a.example.test/v1");
    assert_eq!(current_provider_id(&paths), "provider-c");
    assert!(load_preferences(&paths).connection_override.is_none());
}

#[tokio::test]
async fn preference_write_failure_rolls_back_config_auth_and_connection_relation() {
    let (paths, service) = service_with_preference_write_failure();
    let before = four_managed_file_bytes(&paths);
    let error = service.apply_provider_connection(apply_input(&service, "provider-b")).await.unwrap_err();
    assert_eq!(error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
    assert_eq!(four_managed_file_bytes(&paths), before);
}
```

运行：

```powershell
npm run test:rust:lib -- provider_service::tests::apply_ provider_service::tests::restore_ provider_service::tests::switch_
```

预期：公开方法和关系写后验证不存在，测试失败。

- [x] **Step 2: 实现两个公共 ProviderService 方法和写后验证。**

```rust
pub async fn apply_provider_connection(
    &self,
    input: ApplyProviderConnectionInput,
) -> Result<ProviderMutationOutcome, AppError>;

pub async fn restore_provider_connection(
    &self,
    input: RestoreProviderConnectionInput,
) -> Result<ProviderMutationOutcome, AppError>;
```

应用时后端仅接受来源 ID 和四文件指纹：从最新快照解析当前顶层目标、来源已选 URL/Key、首次覆盖恢复条目；使用 `config_service::set_provider_base_url` 局部改写目标块、`render_auth_json` 改写认证，并将 v4 关系同一事务写入。重复相同来源/条目返回 `PROVIDER_CONNECTION_ALREADY_APPLIED`；更新连接保留首次恢复点。

恢复时若当前顶层仍是目标，写回目标 URL 和原 Key 并清除关系；若外部已改顶层，只写回旧目标 URL 并保留当前认证。恢复条目缺失时返回 `PROVIDER_CONNECTION_RESTORE_UNAVAILABLE`，不做部分写入。所有 validator 重新读取磁盘并验证顶层身份、URL、认证、关系条目和恢复点。

- [x] **Step 3: 将关系复原编入 `switch_provider`。**

正常切换前先解析任何有效或可恢复的关系，要求恢复 URL/Key 条目存在；在同一 `SwitchProvider` 事务中写回旧目标 Provider 块、应用新 Provider 顶层偏好和认证、清除关系。无论前向写入、临时校验或写后验证在哪一步失败，四文件必须恢复为原始字节。

- [x] **Step 4: 保护关系引用，写入失败测试后实现。**

在 `save_provider_base_urls`、`select_provider_base_url`、`save_provider_api_keys`、`select_provider_api_key`、`import_current_auth_key` 和 `delete_provider` 前调用统一保护函数：目标 URL/Key 控件返回 `PROVIDER_CONNECTION_TARGET_LOCKED`；来源应用条目、目标恢复条目不可删除或替换值，返回 `PROVIDER_CONNECTION_ENTRY_IN_USE`；当前来源不可删除，返回 `PROVIDER_CONNECTION_SOURCE_DELETE_FORBIDDEN`。重命名和非引用来源条目管理继续允许。

- [x] **Step 5: 运行事务与集成回归。**

```powershell
npm run test:rust:lib -- provider_service::tests
npm run test:rust:provider-workflow
```

预期：TOML 注释/未知字段保留，`auth.json` 只含 `OPENAI_API_KEY`，事务失败后四个文件与事务前字节一致。

## Task 4：可用性测试、自检与 Tauri 命令边界

**文件：**
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_service.rs`
- 修改：`src-tauri/src/services/self_check_service.rs`
- 修改：`src-tauri/src/commands/provider_commands.rs`
- 修改：`src-tauri/src/lib.rs`
- 修改：`src-tauri/src/commands/mod.rs`
- 修改：`src-tauri/crates/codex-relay-core/src/services/provider_availability_service.rs` 的测试或现有 ProviderService 测试
- 修改：`src-tauri/src/services/self_check_service.rs` 的测试
- 修改：`src-tauri/src/commands/mod.rs` 的测试

- [x] **Step 1: 写 routed 可用性、自检与 command 的失败测试。**

```rust
#[test]
fn availability_target_for_routed_identity_uses_source_key_and_identity_model() {
    let target = service_with_active_a_routed_to_b().resolve_availability_target("provider-a").unwrap();
    assert_eq!(target.base_url, "https://provider-b.example.test/v1");
    assert_eq!(target.api_key, "test-key-b-not-real");
    assert_eq!(target.model, "gpt-5.6-sol");
}

#[test]
fn extended_self_check_accepts_active_routed_connection() {
    let report = self_check_for_active_a_routed_to_b().run_extended();
    assert!(report.checks.iter().any(|check| check.id == "managed-base-url" && check.level == HealthLevel::Normal));
    assert!(report.checks.iter().any(|check| check.id == "auth-key-match" && check.level == HealthLevel::Normal));
}

#[tokio::test]
async fn apply_connection_command_accepts_only_source_and_fingerprints_without_secret_json() {
    let result = apply_provider_connection_inner(&state, ApplyProviderConnectionInput { source_provider_id: "provider-b".into(), expected_files }).await;
    let json = serde_json::to_string(&result).unwrap();
    assert!(result.success);
    assert!(!json.contains("test-key-b-not-real"));
    assert!(!json.contains("\"apiKey\":"));
}
```

运行：

```powershell
npm run test:rust:lib -- provider_service::tests::availability
cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay --lib commands::tests self_check_service::tests
```

预期：routed resolver、typed commands 和自检分支尚未存在，测试失败。

- [x] **Step 2: 接入运行时边界。**

有效 `routed` 当前身份的 `resolve_availability_target` 使用实际路由 URL、来源 Key 和目标自身模型；失效关系返回 `PROVIDER_CONNECTION_OVERRIDE_STALE`，不发网络请求。自检把有效路由报告为“来源条目已纳管/认证一致”，失效关系报告安全错误且绝不写盘。

新增 `apply_provider_connection` 与 `restore_provider_connection` command，沿用现有应用写锁、托盘刷新和统一 `CommandResult`。在 `lib.rs` 注册；在命令单元测试序列化结果，确认 API Key、`apiKey` 字段、完整文件内容和路径均不泄漏。

- [x] **Step 3: 运行边界测试。**

```powershell
npm run test:rust:lib -- provider_service::tests::availability
cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay --lib commands::tests self_check_service::tests
```

预期：失效关系测试不触发网络 runtime，命令采用 `{ input }` 包装。

## Task 5：TypeScript IPC 与共享 mutation

**文件：**
- 修改：`src/types/provider.ts`
- 修改：`src/services/tauri.ts`
- 修改：`src/composables/useProviders.ts`
- 修改：`src/services/tauri.test.ts`
- 修改：`src/composables/useProviders.test.ts`

- [x] **Step 1: 写 typed client 和 composable 的失败测试。**

```ts
it('sends only sourceProviderId and expectedFiles when applying a connection', async () => {
  await applyProviderConnection({ sourceProviderId: 'provider-b', expectedFiles: fingerprints })
  expect(invoke).toHaveBeenCalledWith('apply_provider_connection', {
    input: { sourceProviderId: 'provider-b', expectedFiles: fingerprints },
  })
})

it('uses shared busy state, one fingerprint snapshot, then refreshes after restore', async () => {
  const outcome = await providers.restoreConnection()
  expect(outcome?.message).toBe('当前 Provider 连接已恢复。')
  expect(client.restoreProviderConnection).toHaveBeenCalledWith({ expectedFiles: fingerprints })
  expect(client.listProviders).toHaveBeenCalledTimes(2)
})
```

运行：

```powershell
npm test -- --run src/services/tauri.test.ts src/composables/useProviders.test.ts
```

预期：`applyProviderConnection`、`restoreProviderConnection` 与 composable methods 不存在，测试失败。

- [x] **Step 2: 添加安全输入、连接 DTO 与操作方法。**

把 Rust 的 `connection` 投影完整映射为 TypeScript discriminated unions；输入仅包含 `sourceProviderId` 和 `expectedFiles`，恢复仅包含 `expectedFiles`。`useProviders` 将两者接到已有 `currentExpectedFiles`、`mutate`、busy、错误和权威刷新，不在 refs 中保存 Key、URL 值或自推导连接状态。

- [x] **Step 3: 运行前端边界测试。**

```powershell
npm test -- --run src/services/tauri.test.ts src/composables/useProviders.test.ts
npm run typecheck
```

预期：IPC 名称、`{ input }` 参数形状、busy 互斥和刷新行为均通过。

## Task 6：卡片动作、确认框与详情锁定

**文件：**
- 创建：`src/components/ProviderConnectionConfirmDialog.vue`
- 创建：`src/components/ProviderConnectionConfirmDialog.test.ts`
- 修改：`src/components/ProviderList.vue`
- 修改：`src/components/ProviderStatus.vue`
- 修改：`src/components/ProviderEndpointControls.vue`
- 修改：`src/components/ProviderCredentialControls.vue`
- 修改：`src/views/ProvidersView.vue`
- 修改：`src/components/ProviderList.test.ts`
- 修改：`src/components/ProviderStatus.test.ts`
- 修改：`src/components/ProviderEndpointControls.test.ts`
- 修改：`src/components/ProviderCredentialControls.test.ts`
- 修改：`src/views/ProvidersView.test.ts`

- [x] **Step 1: 写卡片状态和确认取消的失败测试。**

```ts
it.each([
  ['apply', '仅应用连接'],
  ['applied', '已应用'],
  ['update', '更新连接'],
  ['restore', '恢复自身连接'],
])('renders the %s action with its accessible Provider name', (action, label) => {
  const wrapper = mount(ProviderList, { props: { providers: [provider({ connection: connection(action) })], selectedProviderId: 'provider-a', busy: false } })
  expect(wrapper.get(`[aria-label="${label} Provider A"]`).text()).toBe(label)
})

it('opens a safe connection confirmation and does not invoke on cancel', async () => {
  await wrapper.get('[aria-label="仅应用连接 Provider B"]').trigger('click')
  expect(wrapper.text()).toContain('顶层 model_provider 不变')
  expect(wrapper.text()).not.toContain('test-key-b-not-real')
  await wrapper.get('[aria-label="取消确认"]').trigger('click')
  expect(state.applyConnection).not.toHaveBeenCalled()
})
```

运行：

```powershell
npm test -- --run src/components/ProviderList.test.ts src/components/ProviderStatus.test.ts src/views/ProvidersView.test.ts
```

预期：连接 action、专用确认对话框和锁定描述尚不存在，测试失败。

- [x] **Step 2: 实现卡片与确认编排。**

在 Provider 卡片按 `编辑 | 使用 | 连接动作 | 删除` 放置 action。按钮由后端 enum 映射为“仅应用连接”“已应用”“更新连接”“恢复自身连接”，可访问名称包含 Provider 名称；禁用原因可见且非颜色唯一表达。`ProviderConnectionConfirmDialog` 接收安全摘要字段，分别显示来源/目标/条目名称和“顶层 model_provider 不变”，绝不接收或拼接 URL 全文/API Key 值。`ProvidersView` 只保留短生命周期待确认摘要；取消绝不发 IPC，确认调用 composable 后刷新。

- [x] **Step 3: 实现 routed 详情与后端锁定对应的 UI。**

`ProviderStatus` 同时表达“当前身份”“当前连接”与“选择已变化”。`ProviderEndpointControls` 与 `ProviderCredentialControls` 在 routed 目标显示来源 Provider 和安全条目名称，禁用 segmented、管理按钮和对应事件，并呈现“恢复自身连接后可管理”的可见提示；非目标 Provider 继续按既有操作。所有文本在 900x620 和窄窗口下可换行，不改变现有布局层级。

- [x] **Step 4: 运行 UI 回归和人工截图检查。**

```powershell
npm test -- --run src/components/ProviderConnectionConfirmDialog.test.ts src/components/ProviderList.test.ts src/components/ProviderStatus.test.ts src/components/ProviderEndpointControls.test.ts src/components/ProviderCredentialControls.test.ts src/views/ProvidersView.test.ts
npm run typecheck
```

预期：四种卡片状态、确认/取消、受限控件、键盘焦点、深浅主题与窄窗口文本均无重叠；截图观察另记为实际证据，不以测试替代。

## Task 7：文档、全量检查与提交前门禁

**文件：**
- 修改：`README.md`
- 修改：`src/views/AboutView.vue`
- 修改：`src/views/AboutView.test.ts`
- 修改：必要的 fixture、测试与本任务验证记录

- [x] **Step 1: 写文档界面说明的失败测试。**

```ts
it('describes routed connection recovery without claiming key encryption or automatic validation', () => {
  const text = mount(AboutView, { props: { appVersion: '0.3.0', configDirectory: 'C:\\test' } }).text()
  expect(text).toContain('保持 model_provider')
  expect(text).toContain('恢复自身连接')
  expect(text).not.toContain('自动验证 API Key')
  expect(text).not.toContain('加密 API Key')
})
```

运行：

```powershell
npm test -- --run src/views/AboutView.test.ts
```

预期：尚未说明 v4 关系、显式恢复和普通切换复原，测试失败。

- [x] **Step 2: 更新用户契约。**

README 与关于页说明：连接覆盖不改变 `model_provider`、仅采用来源的已选 URL/Key、当前与来源必须受管、恢复回首次覆盖前条目、普通切换会先复原旧目标、v4 只保存稳定条目 ID。保留“密钥明文只在现有本地文件和备份中”的事实，不宣称加密、自动恢复或自动联网验证。

- [x] **Step 3: 运行完整质量门禁和路径/秘密检查。**

```powershell
npm run test:rust:lib
npm run test:rust:provider-workflow
npm run test:rust:path-safety
npm run check:frontend
npm run check:rust
rg -n --glob '!node_modules/**' --glob '!src-tauri/target/**' 'test-key-' .
git diff --check
```

预期：所有命令退出为 0；若密钥扫描命中真实值、路径哨兵失败、格式/类型/Clippy 失败或截图出现布局问题，停止提交并按实际错误修复。

- [x] **Step 4: 提交本任务改动。**

```powershell
git add -- README.md fixtures/provider-preferences-v1.json fixtures/provider-preferences-v3.json src/types/provider.ts src/services/tauri.ts src/composables/useProviders.ts src/components/ProviderConnectionConfirmDialog.vue src/components/ProviderConnectionConfirmDialog.test.ts src/components/ProviderList.vue src/components/ProviderList.test.ts src/components/ProviderStatus.vue src/components/ProviderStatus.test.ts src/components/ProviderEndpointControls.vue src/components/ProviderEndpointControls.test.ts src/components/ProviderCredentialControls.vue src/components/ProviderCredentialControls.test.ts src/views/ProvidersView.vue src/views/ProvidersView.test.ts src/views/AboutView.vue src/views/AboutView.test.ts src-tauri/crates/codex-relay-core/src/models/provider.rs src-tauri/crates/codex-relay-core/src/models/transaction.rs src-tauri/crates/codex-relay-core/src/services/provider_preference_service.rs src-tauri/crates/codex-relay-core/src/services/provider_service.rs src-tauri/crates/codex-relay-core/src/services/backup_service.rs src-tauri/crates/codex-relay-core/tests/provider_workflow.rs src-tauri/src/commands/provider_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/src/services/self_check_service.rs .trellis/tasks/07-30-sync-current-codex-provider-credentials
git commit -m "feat(provider): 支持保持身份的连接同步"
```

提交前精确检查暂存清单，只包含本任务相关文件；不要暂存用户的无关改动。提交后记录哈希、实际运行的验证命令和未执行的人工检查。

## 风险与回滚点

- 任意连接写入必须经 `TransactionService`，不得直接写真实用户目录或跳过四文件指纹、备份、原子替换、解析和写后验证。
- 产品恢复依赖 v4 关系及受保护稳定 ID；回滚产品版本前先在 UI 显式恢复活动连接，或保留支持 v4 的版本完成恢复。
- 外部修改只会使关系投影失效；读取、watcher、自检和可用性测试不得自动清理或修复文件。
- 所有测试使用 `AppPaths::for_test` / 成对 Relay 覆盖和 `test-key-*-not-real`；不读取、写入或递归扫描真实 `%USERPROFILE%\\.codex` 或 `%LOCALAPPDATA%\\CodexRelay`。
