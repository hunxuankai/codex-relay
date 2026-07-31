# 同步当前 Codex Provider 连接技术设计

## 1. 背景与目标

Codex 线程包含 `modelProvider` 身份，官方 App Server 文档也允许按 `modelProviders` 过滤线程；直接切换顶层 `model_provider` 会改变会话所属 Provider 身份。Codex 配置参考确认自定义 Provider 的地址字段是 `[model_providers.<id>].base_url`。

本功能把“Codex 当前 Provider 身份”与“实际连接来源”分离：用户可以保持顶层 `model_provider` 不变，只把另一个 Relay Provider 已选中的 Base URL 和 API Key 应用到当前身份，从而继续使用原线程身份。官方依据：

- <https://developers.openai.com/codex/config-reference/#configtoml>
- <https://learn.chatgpt.com/docs/app-server#api-overview>

本功能不修改 Codex 会话文件，不迁移线程，不同步来源 Provider 的模型、推理强度或 Fast，也不自动联网验证连接。

## 2. 架构边界

沿用现有单一写入链路：

```text
ProviderList
  -> ProvidersView 确认编排
  -> useProviders 显式动作与 busy/指纹
  -> src/services/tauri.ts typed command
  -> provider_commands 单次服务委托
  -> ProviderService 解析连接与恢复语义
  -> TransactionService 锁、备份、原子写、验证、回滚
  -> config.toml / auth.json / providers.json / provider-preferences.json
```

前端不读取文件，不接收 API Key。command 不解析业务文件，只做参数转换、一次服务调用、结果映射与现有托盘/事件刷新。`ProviderService` 是应用、更新、恢复和普通切换整合的唯一业务入口。

## 3. 私有数据格式与迁移

`provider-preferences.json` 从 v3 升级为 v4，在顶层增加可选 `connectionOverride`：

```json
{
  "version": 4,
  "providerOrder": ["provider-a", "provider-b"],
  "providers": {},
  "connectionOverride": {
    "targetProviderId": "provider-a",
    "sourceProviderId": "provider-b",
    "appliedBaseUrlId": "url-b-primary",
    "appliedApiKeyId": "key-b-primary",
    "restoreBaseUrlId": "url-a-primary",
    "restoreApiKeyId": "key-a-primary"
  }
}
```

字段含义：

- `targetProviderId`：顶层 `model_provider` 所指向、其 Provider 块被覆盖的身份。
- `sourceProviderId`：提供实际连接的 Relay Provider。
- `appliedBaseUrlId` / `appliedApiKeyId`：本次真实写入的来源条目，不随来源之后的预选变化。
- `restoreBaseUrlId` / `restoreApiKeyId`：第一次覆盖前目标自身的受管条目，后续更新来源时保持不变。

关系只存稳定 ID，不复制 URL 或密钥。结构校验要求目标与来源不同、Provider ID 规范化、六个 ID 非空；跨文件存在性和值匹配由 `ProviderService` 在一致磁盘快照上验证。

新增独立 v3 解析结构；v1/v2/v3 在内存中升级为 v4 且 `connectionOverride=null`，启动、列表、自检和文件监控保持只读。只有下一次成功用户事务才写出 v4。未知版本继续失败关闭。旧备份恢复后仍可按旧版本读取，不会因恢复动作立即迁移写盘。

## 4. 公开命令与 DTO

新增命令：

```ts
interface ApplyProviderConnectionInput {
  sourceProviderId: string
  expectedFiles: FileSetFingerprint
}

interface RestoreProviderConnectionInput {
  expectedFiles: FileSetFingerprint
}

applyProviderConnection(input): Promise<ProviderMutationOutcome>
restoreProviderConnection(input): Promise<ProviderMutationOutcome>
```

应用命令不接收目标 Provider、条目 ID、URL 或 Key；恢复命令不接收 Provider ID。Rust 从事务前最新快照重新解析当前目标、已选来源和恢复关系，确认框期间的任何文件变化都由 `expectedFiles` 拦截。

后端在普通 `ProviderProfile` 中增加不含秘密的连接投影：

```ts
type ProviderConnectionAction = 'apply' | 'applied' | 'update' | 'restore' | null
type ProviderConnectionRole = 'identity' | 'source' | null
type ProviderConnectionStatus = 'none' | 'active' | 'stale'

interface ProviderConnectionProjection {
  role: ProviderConnectionRole
  status: ProviderConnectionStatus
  action: ProviderConnectionAction
  disabledReason: string | null
  sourceProviderName: string | null
  appliedBaseUrlName: string | null
  appliedApiKeyName: string | null
}
```

所有业务资格和 action 枚举由 Rust 投影。Vue 只把枚举映射为按钮文案和事件，不通过 URL、Key 状态或 Provider ID 重新推导连接关系。

`ProviderBaseUrlStatus` 和 `ProviderApiKeyStatus` 新增 `routed`：仅当当前身份的有效关系把实际 URL/Key 精确匹配到来源已应用条目时使用。此时目标自身的 `selectedBaseUrlId` / `selectedApiKeyId` 不冒充为当前实际选择；详情页显示来源与已应用条目名称，URL/Key 控件进入只读锁定状态。`configurationComplete` 把有效 `routed` 视为完整，失效关系仍按安全失败投影。

## 5. 应用与更新事务

`ProviderService::apply_provider_connection` 执行：

1. 一致读取四个受管文件和指纹，解析顶层目标 Provider。
2. 验证来源与目标不同，二者 Provider、模型偏好、受管 URL 和受管 Key 均完整。
3. 来源 URL 使用其 `config.toml.base_url` 匹配到的命名条目；来源 Key 使用 `providers.json.selectedApiKeyId`。
4. 首次覆盖时，目标恢复 URL 由目标当前 `base_url` 匹配，恢复 Key 由当前 `auth.json` 匹配目标命名密钥；必要时把目标 `selectedApiKeyId` 规范为该实际认证条目。
5. 已有有效覆盖时保留 `restore*Id`，只更新来源与 `applied*Id`；失效关系阻止新覆盖。
6. 使用 `toml_edit` 只设置目标 Provider 块的 `base_url`；顶层 Provider、模型、推理强度、Fast 和未知内容保持原值。
7. 用来源 Key 渲染只含 `OPENAI_API_KEY` 的规范 `auth.json`，写出 v4 关系及必要的旧版本升级。
8. 通过 `TransactionService` 以 `SyncCurrentProvider` operation 写入并执行写后业务验证。

写后验证要求顶层 `model_provider` 未变、目标实际 URL 与来源条目一致、当前认证与来源 Key 一致、六个关系 ID 可解析，且恢复条目仍属于目标。相同来源和相同条目的重复直接调用返回 `PROVIDER_CONNECTION_ALREADY_APPLIED`，不创建无意义事务。

## 6. 显式恢复与普通切换

`ProviderService::restore_provider_connection` 使用新的 `RestoreCurrentProvider` operation：

- 当前顶层 Provider 仍是关系目标时，同一事务写回目标恢复 URL、目标恢复 Key 并删除关系。
- 顶层 Provider 已被外部改为其他 Provider 时，只恢复旧目标 Provider 块的 URL，不改属于新当前 Provider 的 `auth.json`，然后删除关系。
- 关系存在但恢复条目缺失或损坏时禁止部分恢复，保留关系和备份入口。

普通 `switch_provider` 在有效覆盖期间不能只删除关系。它必须在原切换事务中：

1. 恢复旧目标 Provider 块的自身 URL。
2. 应用新 Provider 的顶层 ID、模型、推理强度、Fast 和认证。
3. 删除连接关系。
4. 同时验证旧目标块、新当前 Provider 和认证三者正确。

这样切换到 C 后，A 不会遗留 B 的 URL，也不会丢失唯一恢复点。切换失败时所有文件按原始字节回滚。

关系已失效时，普通切换仍只能在恢复条目完整的前提下继续：同一事务先恢复旧目标 Provider 块，再应用新 Provider 并清除关系。恢复条目不可用时返回 `PROVIDER_CONNECTION_RESTORE_UNAVAILABLE`，不得为了完成切换丢弃失效关系。

## 7. 条目保护与失效关系

Relay 内部写操作必须保护：

- 当前已应用的来源 URL/Key 条目不得删除或替换值。
- 目标恢复 URL/Key 条目不得删除或替换值。
- 条目可重命名，稳定 ID 与关系保持有效。
- 当前连接来源 Provider 不得删除；目标已由现有“当前 Provider 不能删除”规则保护。
- 来源改选其他 URL/Key 只改变来源预选；当前连接保持旧条目，投影 action 变为 `update`。
- 有效或可恢复的关系存在时，目标 Provider 的 URL/Key 分段选择和管理入口锁定；后端也拒绝会改变目标恢复点或造成 URL/认证半恢复的直接 command。来源未被引用的条目仍按原有规则可管理。

每次列表读取把持久关系分类为 `active` 或 `stale`：

- 目标仍是顶层当前 Provider，且实际 URL/认证匹配来源条目时为 `active`。
- 顶层 Provider、实际值、来源条目或恢复条目任一不匹配时为 `stale`。

读取路径不修复、不迁移、不清除关系。存在 `stale` 关系时阻止新的应用和更新，以免覆盖恢复点；恢复条目完整时仍允许显式恢复。恢复条目不可用时只提供稳定错误与备份引导。

## 8. 前端交互

连接按钮位于左侧 Provider 卡片，顺序为：

```text
编辑 | 使用 | 连接动作 | 删除
```

状态映射：

| 场景 | 状态文字 | 连接动作 |
|---|---|---|
| 无覆盖的当前 Provider | 当前 | 不显示 |
| 普通完整来源 | 无 | 仅应用连接 |
| 当前身份正在使用外部来源 | 当前身份 | 恢复自身连接 |
| 来源选择仍等于已应用条目 | 当前连接 | 已应用（禁用） |
| 来源预选已改变 | 选择已变化 | 更新连接 |
| 配置不完整或关系失效 | 可见原因 | 禁用或恢复 |

状态不能只靠颜色。按钮具有与可见文案一致并包含 Provider 名称的可访问名称；动作区域允许换行，保持现有双栏和窄窗口布局。

新增 `ProviderConnectionConfirmDialog`：

- 应用/更新显示来源 Provider、已选 URL 名称、已选 Key 名称、目标 Provider ID，以及“顶层 `model_provider` 不变”。
- 恢复显示目标 Provider 和恢复 URL/Key 名称。
- 不显示 API Key 值，不把结构化摘要拼成无法扫描的单段文本。
- 取消不发 IPC；确认按钮分别为“应用连接”“更新连接”“恢复连接”。

`ProviderList` 只发事件，`ProvidersView` 保存短生命周期待确认摘要，`useProviders` 复用共享 mutation busy、文件指纹、稳定错误和成功后权威刷新。连接操作与编辑、删除、排序、选择、切换及 Provider 测试互斥。

`ProviderEndpointControls` 和 `ProviderCredentialControls` 在 `routed` 目标详情中显示当前连接的来源 Provider 名称及安全条目名称，而不是“外部地址/密钥”；分段选择和管理按钮禁用并关联可见恢复提示。

## 9. 自检、文件监控与可用性测试

`ProviderService::list_providers` 是连接关系的唯一投影点。`self_check_service` 必须消费该投影：有效 `routed` 状态分别报告“当前连接来源已纳管”和“当前认证与来源条目一致”，不再把目标自身列表未匹配的实际 URL/Key 当作外部错误；`stale` 关系报告稳定安全错误且不修复文件。

`resolve_availability_target` 对当前身份的有效 `routed` 状态使用实际路由 URL、来源已应用 Key 和目标自身模型偏好。失效关系返回 `PROVIDER_CONNECTION_OVERRIDE_STALE`，不发起网络请求。非当前 Provider 仍使用既有独立选择语义。文件监控只触发权威刷新；关系状态随刷新重新投影，不在 watcher 中写入或清除。

## 10. 文档与用户可见契约

更新 `README.md`、`AboutView.vue` 和对应 Vue 测试：说明连接覆盖保持 `model_provider`、需要当前/来源均纳管、提供显式恢复、普通切换会恢复旧目标块，以及 `provider-preferences.json` v4 保存不含秘密的关系 ID。不得暗示自动联网验证、密钥加密或会话文件迁移。

## 11. 错误契约

| 条件 | 稳定错误 |
|---|---|
| 当前目标 URL/Key 未纳管 | `CURRENT_PROVIDER_CONNECTION_UNMANAGED` |
| 来源配置不完整 | `PROVIDER_CONNECTION_SOURCE_INCOMPLETE` |
| 持久关系与当前文件不一致 | `PROVIDER_CONNECTION_OVERRIDE_STALE` |
| 恢复条目缺失或损坏 | `PROVIDER_CONNECTION_RESTORE_UNAVAILABLE` |
| 删除或替换被引用条目 | `PROVIDER_CONNECTION_ENTRY_IN_USE` |
| 删除当前连接来源 Provider | `PROVIDER_CONNECTION_SOURCE_DELETE_FORBIDDEN` |
| 相同连接重复提交 | `PROVIDER_CONNECTION_ALREADY_APPLIED` |
| 覆盖期间直接修改目标 URL/Key | `PROVIDER_CONNECTION_TARGET_LOCKED` |

现有 `EXTERNAL_MODIFICATION_CONFLICT`、配置/密钥损坏、事务写入失败和 `ROLLBACK_INCOMPLETE` 语义保持不变。公开错误与成功消息只包含 Provider/条目名称和稳定 code，不包含 URL、Key、文件正文或内部路径。

## 12. 测试设计

TDD 按以下公开行为切片实施：

1. v4 结构往返、ID 校验、v1/v2/v3 延迟迁移和未知版本失败关闭。
2. 应用 B 到 A 保持顶层 Provider、模型、推理强度和 Fast，只改变 A URL、认证与关系。
3. B -> C 更新保留首次恢复点，显式恢复回到 A。
4. 覆盖期间普通切换 C 自动恢复 A 块并完整应用 C。
5. 来源选择变化、条目/Provider 保护、有效/失效投影和只读路径零写入。
6. config/auth/providers/preferences 各写入失败、临时解析失败、写后验证失败和回滚失败。
7. v4 备份恢复、v1/v2/v3 旧备份兼容、外部顶层切换后的仅目标块恢复。
8. typed service 命令名与 `{ input }` 包装、composable busy/指纹/刷新、卡片状态、确认与取消行为。
9. 有效 `routed` 投影、自检正常结果、失效自检错误、当前身份可用性测试使用路由 URL/Key且不在失效关系下联网。
10. 覆盖期间目标详情 URL/Key 控件锁定、来源未引用条目仍可管理、900x620 和窄窗口布局、键盘焦点、可见禁用原因、明暗主题与文本换行。

Rust 测试使用 `tempfile` / `AppPaths::for_test`、真实解析器、ProviderService 和 TransactionService；只在文件故障阶段替换 `FileOps`。前端只 mock typed Tauri client 或 composable。fixture 仅使用 `test-key-*-not-real`，不运行真实 Codex、不联网、不访问真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`。

完成前至少运行专项测试、`npm run check`、路径安全哨兵、密钥扫描、`git diff --check`，并实际观察两个窗口宽度。未执行的构建、安装或人工行为不得声明成功。

## 13. 兼容性、回滚与权衡

- 选择显式 v4 关系而非跨 Provider 值推断，避免重复 URL/Key 导致来源歧义，并能保存确定恢复点。
- 选择稳定条目 ID 而非原始值快照，避免第三份密钥副本；代价是覆盖前目标必须完全纳管，并需要保护引用条目。
- 不依赖某个事务备份作为产品恢复点，避免备份保留清理和无关文件恢复影响一键恢复；事务备份仍承担故障与人工恢复职责。
- 回滚产品代码前应先通过 UI 恢复任何活动连接覆盖。若直接降级到不理解 v4 的旧版 Relay，旧版会按未知版本失败关闭；因此发布说明必须提示先恢复连接并保留备份，或继续使用支持 v4 的版本完成恢复。
