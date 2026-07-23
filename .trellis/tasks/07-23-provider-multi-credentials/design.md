# 技术设计

## 方案选择

采用“扩展现有两个 Relay 私有存储”的方案，不新增第五个受管文件：

- `provider-preferences.json`：保存命名 Base URL 目录和可选模型偏好。
- `providers.json`：保存命名 API Key、密钥值和非当前 Provider 的密钥预选。
- `config.toml`：继续保存 Codex 官方 Provider 定义，其中实际 `base_url` 是该 Provider
  当前 URL 选择的唯一真相。
- `auth.json`：继续保存当前生效 Provider 的实际认证。

备选方案及放弃原因：

1. **把 URL、模型和密钥全部合并进 `providers.json`**：文件数量少，但任何非秘密 URL
   或模型修改都会重写包含全部密钥的文件，扩大秘密处理面并混合职责。
2. **新增 `provider-endpoints.json`**：职责最纯，但会引入第五个路径、指纹、备份、恢复、
   监控、自检和回滚分支；现有 `provider-preferences.json` 已是合适的非秘密私有数据边界。
3. **在 `config.toml` Provider 块写 Relay 私有数组**：污染外部配置契约，违反既有项目规范，
   并可能被 Codex 或其他工具误解，明确不采用。

## 架构与数据流

```text
ProviderEditor / Provider detail / management dialogs
  → useProviders / useProviderApiKeyManager 显式动作
  → src/services/tauri.ts typed commands
  → Tauri provider command adapters
  → ProviderService
     ├→ ConfigService（官方 TOML 与实际 Base URL）
     ├→ ProviderPreferenceService（命名 URL + 模型偏好）
     ├→ ProviderSecretService（命名密钥 + 密钥预选）
     ├→ AuthService（当前实际认证）
     └→ TransactionService（锁、指纹、备份、原子写、验证、回滚）
```

普通列表和详情只经过脱敏投影。只有用户打开 API Key 管理对话框时，专用 command 才返回
目标 Provider 的完整密钥集合；该 DTO 不进入 `useProviders` 的长期资源状态。

## 磁盘结构

### `provider-preferences.json` 版本 2

把模型偏好嵌入可选子结构，使只有外部 Provider 配置、尚未完成模型配置的记录也能保存
命名 Base URL。

```json
{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "主用地址",
          "url": "https://provider-a.example.test/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": {
          "gpt-5.6-sol": "medium"
        }
      }
    }
  }
}
```

不保存 `selectedBaseUrlId`。`config.toml` 中经过规范化的实际 `base_url` 与 `baseUrls`
唯一值匹配后即可得到当前命名项；外部修改为未知 URL 时自然得到“外部地址”状态，避免
维护第二份选择游标。

### `providers.json` 版本 2

```json
{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "主用密钥",
          "apiKey": "test-key-main-not-real"
        }
      ],
      "selectedApiKeyId": "f8e62dc2-46df-4234-92d5-7d318d879ff7"
    }
  }
}
```

非当前 Provider 需要 `selectedApiKeyId` 记住未来应用的密钥。当前 Provider 的实际认证仍以
`auth.json` 为准：若实际值匹配已保存条目，列表映射到该名称；若不匹配，只暴露
“外部密钥”状态。任何 `Debug` 实现都只输出条目 ID、名称和是否配置，不输出 `apiKey`。

## 版本兼容与规范化

- 两个服务都支持显式读取版本 1 与版本 2；未知版本继续失败关闭。
- `providers.json` v1 的单密钥记录在内存中变为 ID 为 `legacy-default`、名称为“默认密钥”
  的单项列表，并设为预选。
- `provider-preferences.json` v1 的模型字段变为 `modelPreference`；只有在该 Provider
  已存在于任一 Relay v1 私有记录时，当前 `config.toml` URL 才在内存中变为
  `legacy-default` / “默认地址”。完全没有 Relay 私有记录的 Provider 仍视为外部 Provider。
- 读取返回规范结构和 `needsUpgrade` 标记，但不写文件。
- ProviderService 的下一次成功用户事务只在对应存储 `needsUpgrade` 或确有业务变化时写入
  v2；写入仍经过统一备份和验证。
- 临时文件验证、备份恢复和自检解析器同时接受 v1/v2，确保旧备份恢复后仍可读取；旧程序
  遇到 v2 会按版本错误失败，而不会静默丢弃新字段。

## 领域模型与校验

核心类型：

- `NamedBaseUrl { id, name, url }`
- `NamedApiKey { id, name, api_key }`
- `ProviderPrivatePreference { base_urls, model_preference }`
- `ProviderSecret { api_keys, selected_api_key_id }`

校验集中在存储所有者服务，前端只做即时反馈：

- 新条目由 Rust 分配 UUID；现有 ID 必须存在于当前快照，客户端不能伪造替换身份。
- 名称 trim 后非空、长度受限，同类名称使用 Unicode 小写规范键比较唯一性。
- URL 复用现有 HTTP(S)、主机和最大长度校验，以 `url::Url` 规范化结果比较实际值唯一性。
- API Key 复用现有 CR/LF 边界规范化，不能为空并限制单值长度；使用完整规范值比较唯一性。
- 数组顺序原样保留，不用 `BTreeMap` 代替有序条目列表。
- 受管记录的 URL/密钥列表非空；`selectedApiKeyId` 必须指向密钥列表中的一项。
- 批量保存时，旧快照中的当前选中 ID 必须仍存在；若当前值被修改，保持同一 ID 并同步
  生效文件。

## 公开 DTO 与命令

普通 `ProviderProfile` 保留兼容字段 `baseUrl` 与 `apiKeyConfigured`，并增加：

- 脱敏的 Base URL 条目摘要（ID、名称、URL）。
- 脱敏的 API Key 条目摘要（ID、名称）。
- 当前/预选条目 ID。
- URL 状态：`managed | external`。
- 密钥状态：`managed | external | missing`。
- 明确的配置完整性与禁用原因，组件不自行猜测。

新增或调整 typed commands：

- `get_provider_api_keys_for_management(providerId)`：显式返回目标 Provider 的完整密钥
  草稿、实际外部密钥状态和文件指纹；使用自定义脱敏 `Debug`。
- `save_provider_base_urls(input)`：批量保存命名 URL；必要时同步当前 `config.toml` URL。
- `save_provider_api_keys(input)`：批量保存命名密钥；必要时同步当前 `auth.json`。
- `select_provider_base_url(input)`：只切换 URL；不改变密钥或全局当前 Provider。
- `select_provider_api_key(input)`：只切换密钥预选；目标为当前 Provider 时同时写 `auth.json`。
- `create_provider`：增加初始 URL 名称和初始密钥名称。
- `update_provider`：编辑模式只负责 Provider 名称、Wire API 和模型集合，不再承担 URL/密钥
  清空或替换。
- 现有 `get_provider_api_key` 单值接口由管理集合接口替代；现有导入当前密钥流程改为要求名称，
  并复用密钥管理事务边界。

所有 mutation input 都携带 `expectedFiles`，避免打开管理器后外部文件变化被旧草稿覆盖。

## 读取投影与外部状态

ProviderService 一次读取四个受管文件并建立统一投影：

1. `config.toml.base_url` 匹配命名 URL → `managed` 与对应 ID。
2. URL 未匹配 → `external`，普通 DTO 可返回实际 URL，但不产生私有条目。
3. 非当前 Provider → 使用 `providers.json.selectedApiKeyId` 作为预选。
4. 当前 Provider 的 `auth.json` 匹配某个已保存密钥 → 映射到该名称；若与存储预选不同，
   在内存规范状态中把实际匹配项作为有效选择并标记 `needsReconcile`；启动不写文件，
   下一次成功 Provider 事务才把该 ID 写回预选。
5. 当前认证未知 → `external`，不返回值；缺少认证或私有密钥记录 → `missing`。

Provider 应用和可用性测试要求 URL、密钥和模型偏好都完整。当前外部值可以显示和进入管理
入口，但在显式命名纳管前不参与 Relay 的应用或测试流程。

## 事务矩阵

| 操作 | config | auth | providers | preferences |
|---|---|---|---|---|
| 创建 Provider | 写 Provider/可选顶层选择 | 立即应用时写 | 写初始命名密钥 | 写初始命名 URL/模型 |
| 编辑 Provider 常规字段 | 写名称等官方字段 | 按既有同步规则 | 仅升级时写 | 写模型偏好/升级 |
| 批量管理 Base URL | 当前值变化时写 | 不变 | 仅升级时写 | 写完整 URL 草稿 |
| 批量管理 API Key | 不变 | 当前密钥值变化时写 | 写完整密钥草稿 | 仅升级时写 |
| 选择 Base URL | 写目标 Provider `base_url` | 不变 | 不变 | 仅升级时写 |
| 选择 API Key | 不变 | 当前 Provider 时写 | 写 `selectedApiKeyId` | 不变 |
| 切换 Provider | 写顶层 Provider/模型/强度 | 写目标预选密钥 | 仅规范化/升级时写 | 仅规范化/升级时写 |
| 删除 Provider | 删除 Provider | 不变 | 删除目标记录 | 删除目标记录 |
| 恢复备份 | 按快照存在状态 | 按快照存在状态 | 按快照存在状态 | 按快照存在状态 |

为管理和选择操作增加明确的 `TransactionOperation` 变体与备份 operation 名称。写后验证必须
重新读取四个文件，验证稳定 ID、顺序、唯一性、实际 URL、密钥预选、当前认证和模型偏好。

## Vue 组件边界

- `ProvidersView.vue`：只组合资源状态、打开/关闭编辑器和管理对话框、显示通知。
- `ProviderEditor.vue`：创建模式编辑初始命名 URL/密钥；编辑模式只处理 Provider 常规字段
  与模型，不持有已保存完整密钥集合。
- `ProviderEndpointControls.vue`：Base URL 分段选择、实际 URL、外部状态和管理事件。
- `ProviderCredentialControls.vue`：API Key 分段选择、状态和管理事件，不接收密钥值。
- `ProviderBaseUrlManagerDialog.vue`：有序 URL 草稿、即时校验、统一保存/取消。
- `ProviderApiKeyManagerDialog.vue`：有序密钥草稿、明文/全部隐藏切换、逐项复制、统一保存；
  对话框关闭或卸载时清空草稿。
- `useProviders`：普通脱敏资源、选择和保存动作，仍只暴露只读状态与显式方法。
- `useProviderApiKeyManager`：按对话框生命周期加载和清理秘密，不缓存到应用级状态。
- `src/services/tauri.ts`：唯一 command 字符串、参数和 DTO 解包边界。

Element Plus 继续按当前版本手动导入。实施前再次核对 `Dialog`、`Segmented`、`Input`、
`Button` 的官方文档和安装包类型，不依赖私有 DOM；测试使用可见文本、ARIA、公开 props/emits。

## 交互与错误处理

- 详情选择操作沿用全局 `busy`，防止 URL、密钥、Provider 和模型选择并发交错。
- 当前 Provider 成功切换或修改选中值时提示“已写入，请重启 Codex 后生效”；非当前 Provider
  提示“已保存，将在应用此 Provider 时生效”。
- 管理保存失败时保留对话框草稿供用户修正；外部修改冲突要求重新加载，不提供强制覆盖。
- 复制成功只显示不含值的局部反馈；复制失败显示安全消息。应用日志和通知不记录剪贴板内容。
- 条目过多时分段控件横向滚动；管理对话框使用可滚动列表，不设置任意数量上限。
- 关闭密钥对话框时将数组替换为空并释放组件；不把“清空变量”声称为操作系统级内存擦除。

## 文档与契约同步

更新 README、AboutView、产品契约和架构规范，说明：

- `provider-preferences.json` 同时保存命名 Base URL 与模型偏好。
- `providers.json` 保存多个命名明文密钥及密钥预选。
- `config.toml` / `auth.json` 只保存实际生效值。
- 备份仍包含明文密钥，卸载仍保留配置、密钥和备份。
- 本功能不提供加密、云同步、自动故障转移或会话中途自动换上游。

## 回滚考虑

实现期间每个行为切片独立保持绿色。若新结构写入后需要代码回滚，旧程序会因版本 2 明确报错，
不会静默删除数组；用户可使用任务前事务备份恢复 v1。不得通过降级时忽略未知字段来制造伪兼容。
