# 技术设计

## 1. 架构与所有权

Fast 沿用现有“Relay 私有 Provider 偏好 -> Codex 顶层当前配置”的投影模型：

```text
ProviderEditor / ProviderPreferenceControls
  -> useProviders 显式动作
  -> typed Tauri service
  -> provider command
  -> ProviderService
     |- ProviderPreferenceService (provider-preferences.json v3)
     |- ConfigService (config.toml 顶层 service_tier / features.fast_mode)
     `- TransactionService (锁、指纹、备份、原子写、验证、回滚)
```

`provider-preferences.json` 是每个 Provider 的 `fastEnabled` 唯一真相；
`config.toml` 只表达当前 Codex 会话的全局生效选择。不得在
`[model_providers.<id>]` 中写入 Fast 或其他 Relay 私有字段。

## 2. 数据结构与兼容读取

### 2.1 私有偏好 v3

`fastEnabled` 放在 Provider 的 `modelPreference` 中，因为它依赖当前偏好模型，但
本身仍是 Provider 级布尔值：

```json
{
  "version": 3,
  "providerOrder": ["provider-a"],
  "providers": {
    "provider-a": {
      "baseUrls": [],
      "modelPreference": {
        "models": ["gpt-5.6-sol", "gpt-5.4-mini"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": {
          "gpt-5.6-sol": "medium",
          "gpt-5.4-mini": "none"
        },
        "fastEnabled": false
      }
    }
  }
}
```

- `PROVIDER_PREFERENCE_VERSION` 升到 3。
- 保留独立 v1/v2 解码结构；两者规范化为 v3，`fastEnabled = false`，并标记
  `needs_upgrade = true`。
- 缺少文件继续返回空 v3 store，不创建文件。
- v3 严格验证：`fastEnabled = true` 时 `selectedModel` 必须支持 Fast；非法组合返回
  `INVALID_PROVIDER_PREFERENCES` 并保留原件。
- 旧版 Relay 遇到 v3 会按未知版本失败关闭，避免降级写入时静默丢失字段。

### 2.2 模型目录与 DTO

`ModelCatalogEntry` / `ModelCatalogItem` 增加 `supportsFast`。当前值：

| 模型 | supportsFast |
|---|---:|
| `gpt-5.6-sol` | true |
| `gpt-5.6-terra` | true |
| `gpt-5.6-luna` | true |
| `gpt-5.5` | true |
| `gpt-5.4` | true |
| `gpt-5.4-mini` | false |

`ProviderProfile` 增加 `fastEnabled`；`CreateProviderInput`、`UpdateProviderInput` 增加
`fastEnabled`。Rust 与 TypeScript 继续使用 camelCase IPC。

新增：

```ts
interface UpdateProviderFastInput {
  providerId: string
  enabled: boolean
  expectedFiles: FileSetFingerprint
}
```

详情页 Fast 使用独立 `update_provider_fast` command；模型/推理强度继续使用
`update_provider_preference`，避免一个命令混合两类用户动作。

## 3. 领域行为

### 3.1 偏好校验

- `ProviderPreference::set_fast(true)` 检查当前模型能力，不支持时返回
  `MODEL_FAST_UNSUPPORTED`。
- 详情模型切换调用现有 `select`；目标模型不支持且 Fast 已开启时自动设为 false，
  并返回是否发生自动关闭供成功消息使用。
- 编辑模型集合的 `reconcile_models` 在回退到不支持模型时同样关闭 Fast。编辑 DTO
  若仍显式提交 `fastEnabled = true`，后端拒绝而不是猜测。

### 3.2 TOML 投影

扩展 `select_provider_with_preference` 接收 `fast_enabled`，所有创建立即启用、编辑
同步、详情即时修改和 Provider 切换复用该唯一投影函数。

Fast 开启：

```toml
service_tier = "fast"

[features]
fast_mode = true
```

- 顶层插入/替换 `service_tier`。
- `[features]` 缺失时创建标准表；存在 Table/InlineTable 时只插入或替换
  `fast_mode`；若 `features` 不是 table-like，返回 `INVALID_FEATURES_CONFIG` 且不
  进入正式写入。
- 即使当前 Codex 默认开启 `fast_mode`，仍显式写 true，保证用户应用 Fast Provider
  后能力门禁与选择一致。

Fast 关闭：

- 只移除顶层 `service_tier`。
- 不读取、删除或修改 `features.fast_mode`，也不把任何伪造的“关闭 tier”写回。

`toml_edit::DocumentMut` 保留非目标内容；专项测试覆盖已有 `[features]` 注释/其他项、
缺少 `[features]`、inline table 和异常 scalar。

### 3.3 操作矩阵

| 操作 | preferences | config | auth/providers |
|---|---|---|---|
| 新建，不立即启用 | 写 v3 Fast 偏好 | 只写 Provider 块 | 沿用现有创建行为 |
| 新建并立即启用 | 写 v3 Fast 偏好 | 写当前 Provider/模型/强度/Fast | 沿用现有认证行为 |
| 编辑，未同步 | 写模型与 Fast 偏好 | 不投影当前顶层 Fast | 沿用现有行为 |
| 编辑当前并同步 | 写模型与 Fast 偏好 | 完整投影 | 沿用现有行为 |
| 详情修改非当前 Fast | 写 | 不变 | 不变 |
| 详情修改当前 Fast | 写 | 完整投影 | 不变 |
| 详情切到不支持模型 | 写模型并自动关 Fast | 当前 Provider 才完整投影 | 不变 |
| 切换 Provider | 只在版本升级时写 | 完整投影目标 Fast | 写目标认证 |

所有多文件变化属于一个 `TransactionRequest`。新增
`TransactionOperation::UpdateProviderFast` 及备份操作名，但不增加受管文件。

## 4. 前端设计

### 4.1 `ProviderPreferenceControls`

- 在现有模型/推理强度区域增加 `ElSwitch`，接收后端 DTO，不直接调用 Tauri。
- 新增 `update-fast` emit；`ProvidersView` 转给 `useProviders.updateFast`。
- 当前模型支持 Fast：开关可操作，显示“Fast 可能增加 ChatGPT credits 或 API
  费用”之类不含固定价格的提示。
- 当前模型不支持 Fast：模型仍可选择，开关显示为关闭且禁用，并通过可见文本和
  `aria-describedby` 说明原因。
- busy 时与现有偏好控件统一禁用。操作完成后只使用后端刷新结果，不做长期乐观
  状态。

### 4.2 `ProviderEditor`

- draft 增加 `fastEnabled`，create 初始化 false，edit 来自 `ProviderProfile`。
- “编辑后实际偏好模型”定义为：现有 `selectedModel` 仍在 draft models 时继续使用；
  否则使用 draft 中第一个模型，与后端 reconcile 回退一致。
- watch 该模型能力；不支持时立即把 draft Fast 设为 false 并禁用开关。
- Fast 变化加入 `activeFieldsChanged`，当前 Provider 才显示现有同步复选框。
- 提交 create/update DTO 时始终携带规范化后的 `fastEnabled`。

### 4.3 typed 边界与状态

- `src/types/provider.ts` 定义新增字段/输入。
- `src/services/tauri.ts` 是唯一新增 `update_provider_fast` command 字符串的位置。
- `useProviders.updateFast` 复用 mutation 防重、指纹和权威刷新；错误保留稳定 code。
- `ProvidersView` 只组合组件和动作，不复制 Fast 支持算法。

## 5. 错误、消息与可访问性

- `MODEL_FAST_UNSUPPORTED`：用户请求为当前模型开启 Fast；零写入。
- 异常 `[features]`：返回 `INVALID_FEATURES_CONFIG`，内部详情只进脱敏日志。
- 自动关闭成功消息同时说明模型已切换和 Fast 因不支持而关闭。
- 当前 Provider Fast 修改成功提示已写入当前 Codex 配置并需要按现有产品语义重启；
  非当前 Provider 提示将在应用时生效。
- switch 的禁用态不能只靠颜色；提示与控件建立描述关系，窄窗口允许换行且不产生
  页面级横向滚动。

## 6. 兼容性、回滚与文档

- 启动、列表、自检和文件监控只兼容读取，不为迁移写盘。
- TransactionService 的现有 preferences/config 临时解析、写后验证和逐字节回滚扩展
  v3 与 Fast 不变量；失败时恢复原始字节和存在状态。
- 外部已有 `service_tier` 不反向导入。用户下一次应用 Provider 时，以该 Provider
  私有 Fast 偏好覆盖或移除顶层值。
- README、AboutView 与 `.trellis/spec/` 的 product、architecture、provider、backend、
  security/frontend 契约同步到 v3 与 Fast 投影。
- 不做真实网络能力测试；第三方 Provider 是否接受 Priority processing 仍是用户和
  远端契约，UI 只承诺模型目录与本地配置正确。

## 7. 取舍

- 选择 Provider 级布尔而非通用 tier 字符串：符合需求并避免暴露动态、非封闭枚举。
- 选择内置模型能力而非运行时 `codex debug models`：保持当前离线目录、启动和错误
  边界不变，代价是随 Relay 发布更新名单。
- 选择独立 Fast command：公开行为更清晰，代价是增加一条 typed IPC 路径。
- 选择单向写 `fast_mode = true`：保证 Fast 完整生效；关闭 Provider Fast 时不反向
  关闭全局能力，避免不同 Provider 相互误伤。
