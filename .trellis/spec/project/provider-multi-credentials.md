# Provider 多命名地址与密钥契约

## 1. 范围与触发条件

本契约适用于以下任一改动：

- 修改 Provider 的列表顺序、Base URL、API Key、模型偏好、当前选择或保持身份的连接覆盖；
- 修改 `providers.json`、`provider-preferences.json`、`config.toml` 或 `auth.json` 的读取、写入、迁移或投影；
- 新增或调整 Provider typed command、普通 DTO、管理 DTO、详情页快速选择或管理对话框；
- 修改外部值纳管、Provider 应用/恢复、API/Codex 可用性测试或配置完整性门禁。

Base URL 与 API Key 是两组独立的有序命名列表，不得持久化为可复用的成对连接配置，也不得实现
自动轮询、故障转移或负载均衡。连接覆盖只记录某次实际应用的两个条目 ID 与首次恢复点，不改变
两组列表的独立选择语义。所有写入继续遵循[配置事务安全](../security/transaction-safety.md)。

## 2. 签名

### Rust / Tauri 命令

```text
list_providers() -> ProviderListState
get_provider_api_keys_for_management(providerId: String)
  -> ProviderApiKeyManagementState
create_provider(input: CreateProviderInput) -> ProviderMutationOutcome
update_provider(input: UpdateProviderInput) -> ProviderMutationOutcome
save_provider_base_urls(input: SaveProviderBaseUrlsInput) -> ProviderMutationOutcome
select_provider_base_url(input: SelectProviderBaseUrlInput) -> ProviderMutationOutcome
save_provider_api_keys(input: SaveProviderApiKeysInput) -> ProviderMutationOutcome
select_provider_api_key(input: SelectProviderApiKeyInput) -> ProviderMutationOutcome
update_provider_preference(input: UpdateProviderPreferenceInput) -> ProviderMutationOutcome
update_provider_fast(input: UpdateProviderFastInput) -> ProviderMutationOutcome
import_current_auth_key(input: ImportCurrentApiKeyInput) -> ProviderMutationOutcome
reorder_providers(input: ReorderProvidersInput) -> ProviderMutationOutcome
apply_provider_connection(input: ApplyProviderConnectionInput) -> ProviderMutationOutcome
restore_provider_connection(input: RestoreProviderConnectionInput) -> ProviderMutationOutcome
```

所有结构使用 `camelCase` IPC 字段。核心输入：

```ts
interface CreateProviderInput {
  id: string
  name: string
  baseUrlName: string
  baseUrl: string
  wireApi: 'responses'
  models: string[]
  fastEnabled: boolean
  apiKeyName: string
  apiKey: string
  activateAfterSave: boolean
  expectedFiles: FileSetFingerprint
}

interface UpdateProviderInput {
  id: string
  name: string
  wireApi: 'responses'
  models: string[]
  fastEnabled: boolean
  syncIfActive: boolean
  expectedFiles: FileSetFingerprint
}

interface ProviderBaseUrlDraft {
  id: string | null
  name: string
  url: string
}

interface ProviderApiKeyDraft {
  id: string | null
  name: string
  apiKey: string
}

interface ReorderProvidersInput {
  providerIds: string[]
  expectedFiles: FileSetFingerprint
}

interface UpdateProviderFastInput {
  providerId: string
  enabled: boolean
  expectedFiles: FileSetFingerprint
}

interface ApplyProviderConnectionInput {
  sourceProviderId: string
  expectedFiles: FileSetFingerprint
}

interface RestoreProviderConnectionInput {
  expectedFiles: FileSetFingerprint
}
```

`id=null` 只表示新增条目；现有 ID 必须来自最近权威快照，客户端不得伪造稳定 ID。

### 普通 DTO 与管理 DTO

普通 `ProviderProfile` 只允许包含：

- `baseUrls: Array<{ id, name, url }>`、`selectedBaseUrlId`、`baseUrlStatus`；
- `apiKeys: Array<{ id, name }>`、`selectedApiKeyId`、`apiKeyStatus`；
- `configurationComplete` 与安全的 `disabledReason`；
- `fastEnabled`、Provider、模型、验证和当前状态字段。
- `connection: { role, status, action, disabledReason, targetProviderId, sourceProviderName, appliedBaseUrlName,
  appliedApiKeyName, restoreBaseUrlName, restoreApiKeyName }` 安全投影。

`ProviderListState.modelCatalog[]` 必须包含 `supportsFast: boolean`；组件只能消费该字段，不能维护
另一份支持模型 ID 常量。

普通 DTO 不得包含 `apiKey`。完整密钥只允许出现在
`ProviderApiKeyManagementState.entries[].apiKey`，并且 Rust `Debug` 必须脱敏。
`baseUrlStatus` / `apiKeyStatus` 的 `routed` 只表示当前身份正使用有效来源连接；前端不得据此
反推来源或恢复点，动作、角色、名称和禁用原因以 `connection` 投影为准。

## 3. 契约

### 3.1 文件所有权

| 文件 | 权威内容 | 禁止行为 |
|---|---|---|
| `config.toml` | Provider 官方字段与每个 Provider 当前实际 `base_url`；顶层当前 Provider/模型/推理强度/Fast 投影 | 不写 Relay 私有数组；不保存第二份 URL 选择游标 |
| `provider-preferences.json` v4 | Provider 显示顺序、命名 Base URL 列表、模型偏好、`fastEnabled` 与可选 `connectionOverride` | 不保存 API Key 或 URL/Key 值副本；不覆盖未知 Provider 记录；不重排 Codex 配置 |
| `providers.json` v2 | 命名 API Key 列表与 `selectedApiKeyId` | 不作为 Provider 官方定义来源；损坏时不覆盖为空 |
| `auth.json` | 当前生效 Provider 的实际密钥 | 不作为非当前 Provider 的密钥预选存储 |

四个文件仍是同一个 `TransactionService` 快照和指纹集合；不得新增绕过事务的直接写路径。

### 3.2 私有存储格式

`provider-preferences.json` v4：

```json
{
  "version": 4,
  "providerOrder": ["provider-a"],
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "url-primary",
          "name": "主用地址",
          "url": "https://provider-a.example.test/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" },
        "fastEnabled": false
      }
    }
  },
  "connectionOverride": {
    "targetProviderId": "provider-a",
    "sourceProviderId": "provider-b",
    "appliedBaseUrlId": "url-b-primary",
    "appliedApiKeyId": "key-b-primary",
    "restoreBaseUrlId": "url-primary",
    "restoreApiKeyId": "key-primary"
  }
}
```

`providers.json` v2：

```json
{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "key-primary",
          "name": "主用密钥",
          "apiKey": "test-key-primary-not-real"
        }
      ],
      "selectedApiKeyId": "key-primary"
    }
  }
}
```

`providerOrder` 是左侧 Provider 列表的完整 ID 排列，只影响 Relay 展示。字段缺失时按
`config.toml` 原顺序展示；外部新增或尚未记录的 Provider 按配置顺序追加。Base URL/API Key
数组顺序分别是条目展示顺序；重命名、替换值和切换不改变顺序，新增项只追加到末尾。
`connectionOverride` 只保存稳定 ID：应用条目属于来源 Provider，恢复条目属于目标 Provider；
目标与来源必须不同。字段缺失表示没有覆盖，不得通过跨 Provider 的 URL/Key 值匹配补造关系。

### 3.3 Provider 列表排序

- 用户通过拖动手柄排序；键盘用户聚焦手柄后使用上下方向键产生同一完整排列。
- 前端放开后可做短期乐观投影，但持久化失败必须恢复先前顺序；最终真相始终来自
  `list_providers`，不得在组件或 localStorage 维护第二份长期顺序。
- command 输入必须是当前 `config.toml` Provider ID 的精确排列：ID 合法、无重复、无缺失、
  无未知项，并携带最近 `FileSetFingerprint`。失败返回安全错误且不写文件。
- 成功事务只写 `provider-preferences.json`；`config.toml`、`auth.json`、`providers.json`、
  当前 Provider、托盘应用语义和 Provider 内部 URL/Key 条目顺序均保持不变。
- 新建 Provider 追加到规范化顺序末尾；删除 Provider 同时移除排序记录；旧/外部缺失记录在
  读取时只做内存 fallback，不因启动、列表或文件监控而写盘。

### 3.4 选择与生效

- 点击 Base URL 只改变 URL 选择；点击 API Key 只改变密钥选择。
- 当前 Provider 切换 URL 时写对应 `config.toml.base_url`；切换密钥时写
  `providers.json.selectedApiKeyId` 和当前 `auth.json`。
- 非当前 Provider 的 URL 点击只修改其 `config.toml.base_url` 预选，不改变顶层当前 Provider；
  密钥点击只修改 `providers.json.selectedApiKeyId`，不得写当前 `auth.json`。
- 应用非当前 Provider 时，使用其预选 URL、密钥、模型、推理强度和 Fast，并在同一事务内写入生效配置。
- 当前选中项不能直接删除，最后一项不能删除；系统不得自动按顺序回退选择。
- 管理对话框提交完整草稿，一次事务全部成功或全部回滚；取消不写文件。

### 3.5 外部值与配置完整性

- `config.toml.base_url` 匹配命名 URL 时投影为 `managed`；未匹配时投影为 `external`，
  但不得自动追加到私有列表。
- 当前 `auth.json` 匹配命名密钥时投影为 `managed`；未匹配时投影为 `external`，
  普通 DTO 只返回状态。
- 缺少命名 URL、命名密钥或模型偏好时，`configurationComplete=false`，并返回安全的
  `disabledReason`；该 Provider 可以查看和补齐，但不能应用或测试。
- 外部地址必须在 URL 管理器中显式命名保存；外部密钥必须通过命名导入入口纳管。
- 常规编辑外部 Provider 可以写入模型偏好，但其 `baseUrls` 必须保持为空，直到用户在地址
  管理器中显式保存命名地址。

### 3.6 前端密钥生命周期

- `useProviders` 只持有普通脱敏 Provider 状态，不得保存完整密钥集合。
- 用户打开 API Key 管理器后，`useProviderApiKeyManager.load(providerId)` 才调用管理查询命令。
- 管理器打开后默认明文显示全部密钥；支持统一隐藏/显示和逐项复制，不增加“查看”二次点击。
- `clear()`、对话框关闭、Vue scope dispose 或请求序列失效时，把密钥数组替换为空；
  晚响应不得重新填充已关闭管理器。
- 密钥不得进入 localStorage、普通通知、日志、事件、测试快照或 Provider 列表。

### 3.7 兼容读取

- `providers.json` v1 单密钥在内存中规范为 `legacy-default` / “默认密钥”。
- `provider-preferences.json` v1 模型记录与受管 Provider 的当前 URL在内存中规范为
  `legacy-default` / “默认地址”。
- `provider-preferences.json` v2 缺少 `providerOrder` 时按空排列读取；不得把缺失字段视为损坏。
- 启动、列表读取、自检和文件监控不得为迁移而写文件；v1/v2 的 Fast 在内存中默认关闭，v1/v2/v3 均在下一次成功用户事务按需写 v4，且默认没有连接覆盖。
- 列表、密钥管理查询和可用性目标解析遇到缺失的私有文件时，只能使用内存空存储；不得创建
  `providers.json` 或 `provider-preferences.json`，也不得使调用方持有的文件指纹自行过期。
- 未知版本失败关闭；临时文件解析、备份恢复和自检必须同时接受 v1/v2/v3/v4。

### 3.8 Provider Fast 与 Codex 全局投影

- `fastEnabled` 是 Provider 私有布尔偏好，默认 `false`；不是 Provider 块字段，也不是通用
  `service_tier` 枚举编辑器。
- 内置模型目录是能力唯一事实来源：GPT-5.6 Sol/Terra/Luna、GPT-5.5、GPT-5.4 支持，
  GPT-5.4 Mini 不支持。UI 只读取 `supportsFast`，Rust 在所有写入口再次校验。
- 开启 Fast 时用 `toml_edit` 写顶层 `service_tier = "fast"`，并单向确保
  `[features].fast_mode = true`；已有标准表、inline table、注释和其他 feature 必须保留。
- 关闭 Fast 只删除顶层 `service_tier`，不得删除 `fast_mode` 或写 `fast_mode = false`。
- 详情页对当前 Provider 的独立 Fast 修改立即写偏好与 `config.toml`；非当前 Provider 只写偏好。
  编辑页继续由 `syncIfActive` 决定是否立即投影。模型选择或编辑回退到不支持模型时，在同一事务把 Fast 关闭。
- `features` 是标量且需要开启 Fast 时返回配置错误；关闭 Fast 不读取或重写 `features`。
- 官方值与费用语义依据 [Codex 配置参考](https://developers.openai.com/codex/config-reference/#configtoml)
  和 [Speed 文档](https://learn.chatgpt.com/docs/agent-configuration/speed)。不使用未定义的关闭值，
  不在运行时执行 `codex debug models` 或网络能力测试。

### 3.9 保持身份的连接覆盖与恢复

- `apply_provider_connection` 只消费来源 Provider 当前已选的受管 URL/Key；后端从最新四文件快照
  解析顶层目标与条目，应用后顶层 `model_provider`、模型、推理强度和 Fast 保持不变。
- 首次应用把目标当时实际 URL 与当前认证匹配到的受管条目固定为 `restore*Id`；后续更新来源只
  改 `sourceProviderId` 和 `applied*Id`，不得把上一来源变成恢复点。
- 有效目标投影为 `routed` 并锁定 URL/Key 选择、批量管理和认证导入；已应用来源/恢复条目的值
  不可替换或删除，关系两端 Provider 不可删除，重命名因稳定 ID 不变而允许。
- `restore_provider_connection` 在顶层仍为目标时恢复目标 URL 和认证，否则只恢复旧目标 URL 并
  保留新当前 Provider 的认证；恢复点不完整时禁止部分恢复并保留关系。
- 普通切换与“创建并立即启用”在同一事务先恢复旧目标块，再应用新 Provider 并清除关系。
  来源 Key 与目标某个 Key 值相同时仍必须保留新目标自己的 `selectedApiKeyId`，不能按值把来源 ID
  写进目标记录。
- 列表、自检、watcher 和可用性解析只读校验关系。实际 URL、认证、顶层身份或引用条目不一致时
  投影 `stale`，禁止新应用/更新和联网测试；恢复点完整时仍可显式恢复或经普通切换安全清除。

## 4. 验证与错误矩阵

| 条件 | 必需结果 / 稳定错误 |
|---|---|
| 名称 trim 后为空或过长 | `INVALID_BASE_URL_NAME` / `INVALID_API_KEY_NAME`，整批不写 |
| 同类名称大小写不敏感重复 | `DUPLICATE_BASE_URL_NAME` / `DUPLICATE_API_KEY_NAME`，整批不写 |
| URL 非 HTTP(S) 或无主机 | `INVALID_BASE_URL`，整批不写 |
| URL 规范值重复 | `DUPLICATE_BASE_URL_VALUE`，整批不写 |
| API Key 去除首尾 CR/LF 后为空或过长 | `EMPTY_API_KEY` / `API_KEY_TOO_LONG`，整批不写 |
| API Key 规范值重复 | `DUPLICATE_API_KEY_VALUE`，整批不写且不泄漏值 |
| 地址或密钥列表为空 | `PROVIDER_BASE_URLS_REQUIRED` / `PROVIDER_API_KEYS_REQUIRED` |
| 客户端提交未知现有 ID | `INVALID_BASE_URL_ID` / `INVALID_API_KEY_ID` |
| Provider 排列重复、缺失、未知或不是当前精确集合 | `INVALID_PROVIDER_ORDER`，不写任何文件 |
| 删除当前 URL / 密钥 | `SELECTED_BASE_URL_DELETE_FORBIDDEN` / `SELECTED_API_KEY_DELETE_FORBIDDEN` |
| 删除最后 URL / 密钥 | `LAST_BASE_URL_DELETE_FORBIDDEN` / `LAST_API_KEY_DELETE_FORBIDDEN` |
| 应用外部 URL、外部密钥或缺失密钥 | `PROVIDER_BASE_URL_UNMANAGED` / `PROVIDER_API_KEY_MISSING` |
| 当前目标 URL/认证未纳管或来源配置不完整 | `CURRENT_PROVIDER_CONNECTION_UNMANAGED` / `PROVIDER_CONNECTION_SOURCE_INCOMPLETE`，不写文件 |
| 关系与顶层身份、实际值或引用条目不一致 | `PROVIDER_CONNECTION_OVERRIDE_STALE`，保留关系且不联网 |
| 恢复条目缺失或损坏 | `PROVIDER_CONNECTION_RESTORE_UNAVAILABLE`，不得部分恢复或清除关系 |
| 删除/替换关系引用条目或删除关系任一 Provider | `PROVIDER_CONNECTION_ENTRY_IN_USE` / `PROVIDER_CONNECTION_SOURCE_DELETE_FORBIDDEN` / `PROVIDER_CONNECTION_TARGET_DELETE_FORBIDDEN`，不写文件 |
| 覆盖期间修改目标 URL/Key | `PROVIDER_CONNECTION_TARGET_LOCKED`，不写文件 |
| 相同来源与已应用条目重复提交 | `PROVIDER_CONNECTION_ALREADY_APPLIED`，不创建事务 |
| 测试外部 URL、外部密钥或缺失密钥 | `PROVIDER_TEST_BASE_URL_UNMANAGED` / `PROVIDER_TEST_KEY_UNMANAGED` / `PROVIDER_TEST_KEY_MISSING` |
| 不支持模型请求开启 Fast | `MODEL_FAST_UNSUPPORTED`，事务前失败且四文件不变 |
| 开启 Fast 时 `[features]` 不是 table/inline table | `INVALID_FEATURES_CONFIG`，不覆盖未知 TOML |
| `expectedFiles` 过期 | `EXTERNAL_MODIFICATION_CONFLICT`，不得强制覆盖 |
| 写入、解析或写后验证失败 | 回滚全部已触及文件；回滚不完整返回 `ROLLBACK_INCOMPLETE` |

## 5. 良好、基线与错误用例

- 良好：当前 Provider 点击“备用地址”只更新 URL；随后点击“备用密钥”只更新密钥，
  两次事务后 `config.toml` 与 `auth.json` 分别匹配所选项。
- 良好：非当前 Provider 预选密钥后，当前 `auth.json` 字节不变；显式应用时预选配置一起生效。
- 良好：打开密钥管理器立即看到全部假密钥，关闭后 manager `entries=[]`，晚返回的旧请求被丢弃。
- 良好：拖动 `provider-a` 到 `provider-b` 之后，列表立即投影新顺序；事务完成后刷新和重启仍
  使用该顺序，而 `config.toml`、当前 Provider 和 `auth.json` 字节不变。
- 良好：当前 Provider 开启 Fast 后同时得到 `service_tier = "fast"` 与 `fast_mode = true`；关闭后
  tier 消失而 feature gate 保留。非当前 Provider 修改 Fast 时当前 `config.toml` 字节不变。
- 良好：A 应用 B 的已选连接后仍保持顶层身份 A；再更新到 C 并显式恢复时回到第一次覆盖前的
  A URL/Key，而不是回到 B。普通切换到 C 会先复原 A 块并清除关系。
- 基线：外部把顶层身份从 A 改为 C 后，A 的关系投影为 `stale`；显式恢复只复原 A 的 URL，
  不覆盖属于 C 的当前认证。
- 基线：v1 文件启动后只在内存显示“默认地址/默认密钥”，磁盘字节不变。
- 基线：v2/v3 偏好启动与 self-check 保持原字节，下一次成功用户事务写 v4，Fast 保持既有语义且连接关系为空。
- 基线：外部不完整 Provider 仍可进入详情与补齐入口，但“使用”和测试按钮显示禁用原因。
- 错误：把 Base URL 与 API Key 绑定成成对连接，导致用户不能独立切换。
- 错误：读取到外部值后自动追加命名项，或列表刷新时自动升级文件。
- 错误：把管理 DTO 放进 `useProviders`、Pinia、localStorage、通知或日志。
- 错误：通过重排 `config.toml.model_providers` 或 localStorage 保存 Provider 顺序。
- 错误：用 `service_tier = "off"` 关闭 Fast，或关闭时把全局 `fast_mode` 写成 false。
- 错误：让前端提交目标 Provider、URL、Key 或恢复条目，按跨 Provider 值相等推断来源，或在
  stale 读取路径自动删除关系。

## 6. 必需测试

- Rust 纯逻辑：名称/值唯一、稳定 ID、Provider/条目顺序、选中项、最后项、v1/v2/v3/v4、连接关系结构、模型 Fast 能力、非法组合、未知版本与脱敏 `Debug`。
- ProviderService 临时目录测试：创建、URL/Key 批量管理、独立选择、当前/非当前语义、
  应用、命名导入、外部状态、缺失私有文件只读、外部 Provider 常规编辑不纳管地址、指纹冲突和
  Provider 重排、非法排列、Fast 当前/非当前矩阵、连接应用/更新/恢复、stale、条目/Provider 保护、
  普通切换与创建后启用复原、同值 Key ID 归属、模型自动关闭、非法模型、指纹冲突和故障回滚。
- TransactionService / Backup / SelfCheck：v1/v2/v3/v4 临时解析、`update_provider_fast` /
  `restore_current_provider` operation 名、旧备份恢复、四文件写后验证、逐字节及存在状态回滚验证和
  `ROLLBACK_INCOMPLETE` 标记保留。
- typed service：精确命令名、`camelCase` 参数和 `{ input }` 包装；断言前脱敏 API Key 字段。
- composable：排序乐观投影/失败恢复、Fast mutation 指纹与权威刷新、busy 防重、Provider 事件晚响应、密钥 manager 的
  load/save/clear/scope dispose/晚响应丢弃，以及连接 apply/restore 的单次指纹与权威刷新。
- Vue：Provider 拖动/方向键排序与 busy 门禁、创建/编辑 Fast 默认与回填、实际偏好模型、费用/不支持提示、模型回退自动关闭、同步选项、两行 `ElSegmented` 独立事件、
  760px 横向滚动、外部/缺失状态、两个批量对话框、默认明文、统一隐藏/显示、复制和关闭清空；
  连接四种卡片动作、安全确认/取消、目标详情锁定、对话框卸载焦点回归和窄窗口无横向溢出。
- 路径与密钥审计：只使用 `test-key-*-not-real`，真实默认目录前后递归快照完全不变，
  Git、任务、规范、日志和快照无真实密钥。

## 7. 错误与正确做法

### 错误：把完整密钥放进普通 Provider 状态

```ts
// 错误：列表刷新、事件和通知都会扩大秘密暴露面。
const providers = ref<Array<ProviderProfile & { apiKeys: ProviderApiKeyDraft[] }>>([])
```

### 正确：普通投影与短生命周期管理器分离

```ts
const providerState = useProviders() // 只有密钥 ID、名称和状态
const keyManager = useProviderApiKeyManager()

await keyManager.load(providerId) // 仅在用户打开管理器后调用
keyManager.clear() // 关闭、卸载或作废请求时立即清空
```

### 错误：维护第二份 URL 选择游标

```json
{
  "selectedBaseUrlId": "url-primary"
}
```

### 正确：由实际配置推导选择

```text
normalize(config.toml.base_url) == normalize(baseUrls[i].url)
  -> selectedBaseUrlId = baseUrls[i].id
  -> 未匹配则 baseUrlStatus = external
```

### 错误：在组件复制 Fast 支持列表并直接改 Provider

```ts
const supportsFast = ['gpt-5.6-sol', 'gpt-5.5'].includes(provider.selectedModel ?? '')
provider.fastEnabled = true
```

### 正确：目录能力下发，事件交给 composable 和后端事务

```ts
const fastSupported = computed(() =>
  modelCatalog.some((model) => model.id === provider.selectedModel && model.supportsFast),
)
if (fastSupported.value) emit('update-fast', true)

// 父级只编排动作，最终状态来自 mutation 后刷新。
await providerState.updateFast(provider.id, true)
```

### 错误：让前端决定连接目标和值

```ts
await applyProviderConnection({ targetProviderId, baseUrl, apiKey, restoreApiKeyId })
```

### 正确：只提交来源和并发基线

```ts
await applyProviderConnection({
  sourceProviderId: provider.id,
  expectedFiles: currentExpectedFiles,
})
// Rust 从事务前最新快照解析目标、应用条目和固定恢复点。
```
