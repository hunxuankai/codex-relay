# Provider 模型与推理强度预设

## 目标

为每个 Provider 管理可用模型、当前偏好模型和逐模型推理强度，并在应用 Provider 或修改当前 Provider 偏好时，把选择同步到 Codex 顶层官方配置；Relay 私有元数据不得写入 `[model_providers.<id>]`。

## 已确认事实与约束

- `config.toml` 顶层 `model`、`model_reasoning_effort` 和 `model_provider` 是 Codex 当前生效配置。
- Relay 私有元数据存放在独立的 `%LOCALAPPDATA%\CodexRelay\provider-preferences.json`；`providers.json` 继续只保存 API Key，`settings.json` 继续只保存应用设置。
- 软件尚未正式投入使用，不实现旧 `[model_providers.<id>].model` 数据迁移；新实现不再读取或写入该字段。
- 首版模型目录随 Relay 版本发布，不联网更新，不在记录中保存来源或核验日期。
- OpenAI 官方目录确认：
  - `gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-5.6-luna`：`none`、`low`、`medium`、`high`、`xhigh`、`max`，默认 `medium`。
  - `gpt-5.5`：`none`、`low`、`medium`、`high`、`xhigh`，默认 `medium`。
  - `gpt-5.4`、`gpt-5.4-mini`：`none`、`low`、`medium`、`high`、`xhigh`，默认 `none`。
- 不展示 `gpt-5.6` 别名，避免与 `gpt-5.6-sol` 重复。
- 引入 Element Plus；本任务只使用 `Select` 多选和 `Segmented`，不借机重写其他 UI。
- 使用 Element Plus 组件前必须查对应版本官方文档，不得猜测组件名、Props、Events、Slots 或类型；任务完成时把规则沉淀到项目规范。
- 所有文件测试必须使用安全临时路径或成对 Relay 覆盖，不得触及真实用户目录或真实密钥。

## 功能需求

### 内置模型目录

- 内置模型 ID、支持的推理强度、最高等级和默认值。
- 前后端均可消费同一权威语义；后端是落盘校验最终边界。
- 未知模型或非法推理强度导致明确错误，不自动删除、替换或改写原文件。

### Provider 编辑

- 使用 Element Plus `Select` 的多选模式，从内置目录选择多个可用模型，不允许自定义输入。
- 每个 Provider 至少选择一个模型。
- 新建时不预选；第一个选中的模型成为初始偏好，各模型首次加入时使用官方默认推理强度。
- 编辑页不选择推理强度，但显示当前偏好模型。
- 移除当前偏好模型时，自动改用剩余模型中最早选择的一个，并在保存前提示；恢复该模型既有强度，没有记录则使用官方默认值。
- 当前 Provider 只有在用户选择“保存后同步当前 Codex 配置”时才同步顶层模型字段。

### Provider 详情

- 采用两行 Element Plus `Segmented`：第一行模型，第二行推理强度。
- 模型行只显示该 Provider 的可用模型；强度行只显示当前模型支持的等级。
- 每个模型分别记住推理强度；切回模型时恢复上次选择，首次选择使用官方默认值。
- 当前 Provider：点击后原子保存偏好并写入顶层 `model`、`model_reasoning_effort`，提示“配置已写入，请重启 Codex 后生效”。
- 非当前 Provider：点击后只保存偏好，提示“已保存，将在应用此 Provider 时生效”。
- 写入期间禁用两个控件；失败时重新加载后端权威状态，不保留失真的乐观状态。
- 响应式布局不得产生页面级横向滚动。

### 缺少偏好的外部 Provider

- 只存在于 `config.toml`、没有偏好记录的 Provider 仍可查看和编辑，状态显示“模型偏好未配置”。
- 详情页不显示可操作的模型/强度控件，提供编辑入口。
- 完成配置前禁止通过 Relay 应用，不自动假设其支持任何模型。

### 数据一致性

- `provider-preferences.json` 使用版本化 JSON；每条记录包含可用模型、当前偏好模型和逐模型强度。
- `selectedModel` 必须属于可用模型集合，强度映射必须恰好覆盖该集合且每个值合法。
- Provider 创建、编辑、删除、切换、即时偏好修改和恢复均经过 `TransactionService`。
- 新文件加入路径、指纹、备份、恢复、监控、自检、损坏处理和备份文件打开白名单。
- 当前 Provider 切换时，同一事务更新 `config.toml`、`auth.json` 与必要的偏好状态；失败时验证回滚。

## 可观察行为切片

1. 通过 Provider 创建公开接口提交多个模型后，列表返回初始偏好为首个选择，逐模型强度为官方默认值；`config.toml`、`providers.json` 与新偏好文件一致，立即应用时 `auth.json` 也一致。
2. 通过 Provider 更新公开接口移除当前偏好模型后，返回明确提示并选择最早剩余模型；未请求同步时顶层配置不变。
3. 通过详情偏好更新公开接口操作非当前 Provider，只改变偏好文件并返回延后生效提示。
4. 通过详情偏好更新公开接口操作当前 Provider，偏好文件与顶层模型字段原子更新，并返回重启提示。
5. 通过切换公开接口应用 Provider 时，顶层 Provider、模型、推理强度和认证全部与目标偏好一致。
6. 偏好缺失、未知模型、非法强度、外部修改冲突或任一文件写入失败时，不产生部分写入；错误码稳定且不泄漏路径或密钥。
7. 前端用户可通过 `Select multiple` 管理集合，通过两行 `Segmented` 管理偏好；当前/非当前 Provider 的提示和命令行为不同。

## 验收标准

- [ ] `[model_providers.<id>]` 不再由 Relay 读写 `model` 或推理强度私有字段。
- [ ] `provider-preferences.json` 的结构、严格校验、损坏保护和版本校验完整。
- [ ] 编辑页使用 Element Plus `Select multiple`，详情页使用两行 Element Plus `Segmented`。
- [ ] 每个 Provider 至少一个模型，当前偏好属于集合，每个模型独立保存合法强度。
- [ ] 当前 Provider 即时同步，非当前 Provider 只保存偏好；用户提示准确。
- [ ] 缺少偏好的外部 Provider 不能应用，但可查看并进入编辑配置。
- [ ] 新文件完整接入事务、备份、恢复、指纹、监控、自检和安全路径测试。
- [ ] TOML 未知字段、注释、其他 Provider 与顶层无关配置继续保留。
- [ ] README、关于页和项目规范准确说明数据职责及 Element Plus 文档核验规则。
- [ ] 完成本次问题复盘，并落实“外部配置字段先核验官方契约、私有元数据不污染外部配置”的规则。
- [ ] `npm run check` 通过；若未执行人工 Windows/Tauri 验证，交付时明确说明。

## 范围外

- 在线动态更新模型目录。
- 自定义模型 ID 或自定义推理强度。
- 调用远端模型接口验证 Provider 是否真正支持所选模型。
- 迁移旧 Provider 块中的 Relay 私有 `model` 字段。
- 全面替换现有 UI 为 Element Plus。
