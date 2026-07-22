# 技术设计

## 架构与数据边界

新增 `provider-preferences.json` 作为 Relay Provider 偏好唯一真相。`config.toml` 仅保留 Codex 官方 Provider 配置和当前顶层选择；`providers.json` 与 `auth.json` 继续只承担密钥职责。

```text
Vue ProviderEditor / Provider detail controls
  → useProviders 显式动作
  → typed Tauri service
  → provider command
  → ProviderService
     ├→ ConfigService（官方 TOML）
     ├→ ProviderSecretService（providers.json）
     ├→ ProviderPreferenceService（provider-preferences.json）
     ├→ AuthService（auth.json）
     └→ TransactionService（统一锁、备份、原子写、验证、回滚）
```

## 数据结构

Rust 与 TypeScript 使用稳定 camelCase DTO。磁盘格式示例：

```json
{
  "version": 1,
  "providers": {
    "provider-a": {
      "models": ["gpt-5.6-sol", "gpt-5.4-mini"],
      "selectedModel": "gpt-5.6-sol",
      "reasoningEfforts": {
        "gpt-5.6-sol": "high",
        "gpt-5.4-mini": "low"
      }
    }
  }
}
```

模型目录使用代码常量和强类型枚举/字符串联合表达。后端提供查询目录的公开 DTO，避免前端复制一份可能漂移的数据；前端只负责展示和即时约束，落盘前由后端重新校验。

## 校验规则

- `version == 1`。
- Provider ID 复用现有 ID 校验。
- `models` 非空、无重复、顺序稳定且全部存在于目录。
- `selectedModel ∈ models`。
- `reasoningEfforts` 的键集合与 `models` 相等。
- 每个强度存在于对应模型的支持集合。
- 解析或校验失败不覆盖原文件；公开错误使用稳定中文消息和错误码。

## 服务与公开接口

新增 `ProviderPreferenceService` 负责纯解析、规范化、目标 Provider 增删改和序列化，不直接绕过事务写文件。

Provider DTO 增加：可用模型、当前偏好模型、逐模型强度、偏好是否已配置。新增偏好更新输入，包含 Provider ID、目标模型、可选目标强度和预期文件指纹。ProviderService 仍是主界面与托盘的唯一业务入口。

详情页模型和强度可以共用一个“更新偏好”命令：后端根据 Provider 是否当前生效决定只写偏好，还是同时写顶层配置。返回结果包含刷新后的 Provider 状态和准确消息，前端不得自行推断落盘范围。

## 事务矩阵

| 操作 | 可能写入的文件 |
|---|---|
| 创建 Provider | config、providers、preferences；立即应用时再写 auth |
| 编辑 Provider | config、providers、preferences；同步当前时再写 auth/顶层模型 |
| 删除 Provider | config、providers、preferences |
| 切换 Provider | config、auth；偏好只读 |
| 修改非当前偏好 | preferences |
| 修改当前偏好 | preferences、config |
| 恢复备份 | 按备份存在状态恢复 config、auth、providers、preferences |

所有操作仍先读取完整受管状态、核对扩展后的文件集指纹、创建统一备份，再生成和验证目标内容。偏好文件加入临时解析器、写后业务验证器和逐字节回滚验证。

## 前端组件边界

- `ProviderEditor.vue`：表单编排；使用 `ElSelect`/`ElOption` 的 `multiple` 模式管理模型 ID 数组，保持密钥局部状态。
- 新的偏好展示组件：接收 Provider 偏好与模型目录，通过两个 `ElSegmented` 发出显式模型/强度选择事件，不直接调用 Tauri。
- `ProvidersView.vue`：组合视图和加载/失败状态，不承载目录校验算法。
- `useProviders`：新增偏好更新动作、busy 状态和权威刷新；旧响应不能覆盖新状态。
- `src/services/tauri.ts`：唯一新增 command 字符串与 DTO 解包位置。

Element Plus 使用手动按需组件导入，并配置 `unplugin-element-plus` 处理样式；不全量注册组件库。实际 API 以实施时安装版本的官方文档为准。

## 交互与失败恢复

- Select 选择顺序决定自动回退优先级。
- 模型变化时，强度 Segmented 切换为该模型已保存强度；不存在时使用目录默认值。
- 提交期间禁用相关控件；成功后使用后端返回状态，失败后刷新磁盘状态并显示稳定错误。
- 当前 Provider 与非当前 Provider 使用不同辅助说明和成功消息。
- 未配置偏好的 Provider 禁止应用，禁用原因必须可见且不只靠颜色表达。

## 兼容性与非迁移策略

软件尚未正式投入使用，不迁移旧 `[model_providers.<id>].model`。实现删除对该字段的业务读取和写入，更新开发 fixture。`toml_edit` 仍保留非目标未知内容；本任务不主动清理用户文件中既存的未知字段。

## 复盘与规范落实

完成阶段使用 `trellis-break-loop` 分析：未经官方契约核验、数据所有权误判、测试固化错误假设。随后用 `trellis-update-spec` 更新项目架构、事务和前端依赖使用规则，并增加开发前检查项。

