# Provider 多命名地址与密钥契约

## 1. 范围与触发条件

本契约适用于以下任一改动：

- 修改 Provider 的列表顺序、Base URL、API Key、模型偏好或当前选择；
- 修改 `providers.json`、`provider-preferences.json`、`config.toml` 或 `auth.json` 的读取、写入、迁移或投影；
- 新增或调整 Provider typed command、普通 DTO、管理 DTO、详情页快速选择或管理对话框；
- 修改外部值纳管、Provider 应用、API/Codex 可用性测试或配置完整性门禁。

Base URL 与 API Key 是两组独立的有序命名列表，不得配对成连接配置，也不得实现自动轮询、
故障转移或负载均衡。所有写入继续遵循[配置事务安全](../security/transaction-safety.md)。

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
import_current_auth_key(input: ImportCurrentApiKeyInput) -> ProviderMutationOutcome
reorder_providers(input: ReorderProvidersInput) -> ProviderMutationOutcome
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
```

`id=null` 只表示新增条目；现有 ID 必须来自最近权威快照，客户端不得伪造稳定 ID。

### 普通 DTO 与管理 DTO

普通 `ProviderProfile` 只允许包含：

- `baseUrls: Array<{ id, name, url }>`、`selectedBaseUrlId`、`baseUrlStatus`；
- `apiKeys: Array<{ id, name }>`、`selectedApiKeyId`、`apiKeyStatus`；
- `configurationComplete` 与安全的 `disabledReason`；
- Provider、模型、验证和当前状态字段。

普通 DTO 不得包含 `apiKey`。完整密钥只允许出现在
`ProviderApiKeyManagementState.entries[].apiKey`，并且 Rust `Debug` 必须脱敏。

## 3. 契约

### 3.1 文件所有权

| 文件 | 权威内容 | 禁止行为 |
|---|---|---|
| `config.toml` | Provider 官方字段与每个 Provider 当前实际 `base_url`；顶层当前 Provider/模型/推理强度 | 不写 Relay 私有数组；不保存第二份 URL 选择游标 |
| `provider-preferences.json` v2 | Provider 显示顺序、命名 Base URL 列表与可选模型偏好 | 不保存 API Key；不覆盖未知 Provider 记录；不重排 Codex 配置 |
| `providers.json` v2 | 命名 API Key 列表与 `selectedApiKeyId` | 不作为 Provider 官方定义来源；损坏时不覆盖为空 |
| `auth.json` | 当前生效 Provider 的实际密钥 | 不作为非当前 Provider 的密钥预选存储 |

四个文件仍是同一个 `TransactionService` 快照和指纹集合；不得新增绕过事务的直接写路径。

### 3.2 私有存储格式

`provider-preferences.json` v2：

```json
{
  "version": 2,
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
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    }
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
- 应用非当前 Provider 时，使用其预选 URL、密钥、模型和推理强度，并在同一事务内写入生效配置。
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
- 启动、列表读取、自检和文件监控不得为迁移而写文件；下一次成功用户事务按需写 v2。
- 列表、密钥管理查询和可用性目标解析遇到缺失的私有文件时，只能使用内存空存储；不得创建
  `providers.json` 或 `provider-preferences.json`，也不得使调用方持有的文件指纹自行过期。
- 未知版本失败关闭；临时文件解析、备份恢复和自检必须同时接受 v1/v2。

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
| 测试外部 URL、外部密钥或缺失密钥 | `PROVIDER_TEST_BASE_URL_UNMANAGED` / `PROVIDER_TEST_KEY_UNMANAGED` / `PROVIDER_TEST_KEY_MISSING` |
| `expectedFiles` 过期 | `EXTERNAL_MODIFICATION_CONFLICT`，不得强制覆盖 |
| 写入、解析或写后验证失败 | 回滚全部已触及文件；回滚不完整返回 `ROLLBACK_INCOMPLETE` |

## 5. 良好、基线与错误用例

- 良好：当前 Provider 点击“备用地址”只更新 URL；随后点击“备用密钥”只更新密钥，
  两次事务后 `config.toml` 与 `auth.json` 分别匹配所选项。
- 良好：非当前 Provider 预选密钥后，当前 `auth.json` 字节不变；显式应用时预选配置一起生效。
- 良好：打开密钥管理器立即看到全部假密钥，关闭后 manager `entries=[]`，晚返回的旧请求被丢弃。
- 良好：拖动 `provider-a` 到 `provider-b` 之后，列表立即投影新顺序；事务完成后刷新和重启仍
  使用该顺序，而 `config.toml`、当前 Provider 和 `auth.json` 字节不变。
- 基线：v1 文件启动后只在内存显示“默认地址/默认密钥”，磁盘字节不变。
- 基线：外部不完整 Provider 仍可进入详情与补齐入口，但“使用”和测试按钮显示禁用原因。
- 错误：把 Base URL 与 API Key 绑定成成对连接，导致用户不能独立切换。
- 错误：读取到外部值后自动追加命名项，或列表刷新时自动升级文件。
- 错误：把管理 DTO 放进 `useProviders`、Pinia、localStorage、通知或日志。
- 错误：通过重排 `config.toml.model_providers` 或 localStorage 保存 Provider 顺序。

## 6. 必需测试

- Rust 纯逻辑：名称/值唯一、稳定 ID、Provider/条目顺序、选中项、最后项、v1/v2、未知版本与脱敏 `Debug`。
- ProviderService 临时目录测试：创建、URL/Key 批量管理、独立选择、当前/非当前语义、
  应用、命名导入、外部状态、缺失私有文件只读、外部 Provider 常规编辑不纳管地址、指纹冲突和
  Provider 重排、非法排列、指纹冲突和故障回滚。
- TransactionService / Backup / SelfCheck：v1/v2 临时解析、新 operation 名、旧备份恢复、
  四文件写后验证和回滚失败保真。
- typed service：精确命令名、`camelCase` 参数和 `{ input }` 包装；断言前脱敏 API Key 字段。
- composable：排序乐观投影/失败恢复、mutation 后权威刷新、busy 防重、Provider 事件晚响应、密钥 manager 的
  load/save/clear/scope dispose/晚响应丢弃。
- Vue：Provider 拖动/方向键排序与 busy 门禁、创建四个初始字段、编辑模式无 URL/Key 修改、两行 `ElSegmented` 独立事件、
  760px 横向滚动、外部/缺失状态、两个批量对话框、默认明文、统一隐藏/显示、复制和关闭清空。
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
