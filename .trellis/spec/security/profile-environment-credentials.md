# CLI Profile 环境变量凭据

## 1. 范围/触发条件

「Providers → CLI Profiles」只发现 `AppPaths.codex_home` 根目录的独立 `<name>.config.toml`；不扫描项目或子目录，不跟随链接及重解析点。按 `config.toml` 与 Profile 的用户配置叠加解析选中的 `model_provider` / `model_providers.<id>.env_key`。这是静态视图，不替代 CLI 的完整配置、策略或认证解析；继承路由应标明。没有默认 Provider 也能打开页面。

## 2. 命令签名

| 命令 | 请求参数 | 成功响应数据 |
|---|---|---|
| `list_cli_profiles` | 无 | `CliProfileList`：目录、Profile 列表、`pendingWrite`；有凭据状态的条目含 `keyStatus.matchedUserKeyName: string \| null` |
| `prepare_profile_key_edit` | `profileName: string` | `ProfileKeyEdit`：目标、五分钟有效的单次编辑令牌与 `providerKeys: { id, name }[]`，无凭据值 |
| `save_profile_env_key` | `input: { profileName, token, newKey }` | `ProfileKeySaved { windowsNotified }` |
| `apply_profile_provider_key` | `input: { profileName, token, apiKeyId }` | `ProfileKeySaved { windowsNotified }` |
| `open_profile_environment_settings` | 无 | `()`：仅请求打开 Windows 编辑器 |

## 3. 配置、凭据与事务契约

- List 只返回可展示的名称、主机、变量名与状态。用户名、密码、完整 URL、TOML 源行、认证值与指纹不进入 DTO。仅安全 Profile 名可生成启动命令；复制不执行命令，不访问供应商。
- 对用户范围与当前 Relay 进程分别返回 `set` / `notDetected` / `unavailable`。它们不代表任意终端的环境或 Key 有效性。Debug、`AppPaths::for_test` 与设置任一 Relay 路径覆盖时不得接触真实注册表环境或进程凭据。
- `matchedUserKeyName` 只来自非空、普通字符串型的 Windows 用户变量。后端将其与默认配置中同 ID 供应商的 Relay 已保存 Key 精确比对，只返回名称，不返回值、条目 ID 或指纹。不按 Providers 的当前选择、进程或系统变量值推断。密钥库按列表请求至多只读加载一次，不创建、修复或向前端抛出读取错误。匹配与写入限制互不影响，UI 标明 Windows 用户范围。
- 只写 `HKCU\Environment` 中不存在或普通字符串型的专用 `*_API_KEY` / `*_TOKEN`；排除保留前缀、配置目录内已知共享引用及系统同名变量。Windows 比较不区分 ASCII 大小写。现有 Key 不回显；新值为非空且无首尾空白或控制字符的 UTF-8 字符串，最多 4096 字节，表单在提交、取消和卸载时清空。
- 可复用来源必须是默认配置中与 Profile 解析结果精确同 ID 的供应商，且是该 ID 在 Relay 密钥库中已命名保存、符合目标变量长度及格式约束的 Key。不能按名称、其他供应商或当前激活项回退，不读取 `auth.json` 寻找外部 Key。密钥库缺失、无匹配或不可读时返回空选项，不创建或修复密钥库，手填照常。
- 复用动作在同一编辑令牌下只接受打开时展示的条目 ID；供应商与变量均绑定于后端基线。保存前、提交前及回读时核对所选条目的名称和值指纹；指纹不会出现在 IPC 或日志中。弹窗展示目标主机、变量和命名按钮；仅显式点击时写入，不持续同步。
- `AppState` 使用与默认 Provider 相同的 `TransactionService` 锁。检查配置和原值指纹，在隔离的 KTM 注册表事务中暂存、读取校验，再核对路由及当前持久值；原子提交后回读确认。未通过 KTM 不用 `setx` 或非事务写入降级。该事务不修改 TOML、`auth.json`、默认供应商或 Relay 密钥库。
- 提交前失败取消事务，按已提交状态验证；提交后补偿仅在仍能确认当前值等于本次写入时用原生事务恢复并验证。发现外部改动不覆盖。`profile-env-pending/` 只含非秘密事务标记，不储存旧值或尝试自动恢复未知结果。
- 回读确认后尽力广播 `WM_SETTINGCHANGE("Environment")`。`windowsNotified` 只是通知结果，不意味着终端已刷新；已运行的进程不会热更新。应用更新或卸载不移除用户变量。

## 4. 验证与错误矩阵

| 条件 | 预期结果 |
|---|---|
| 用户变量精确匹配同 ID 供应商的一条已保存 Key | `matchedUserKeyName` 返回该条名称，即使 Providers 选中其他 Key 或该变量只读 |
| 用户变量没有可识别的值，同 ID 来源没有匹配或密钥库不可读 | `matchedUserKeyName: null`；若用户变量是 `set`，UI 提示未能匹配，否则保留检测状态、不显示推测名称 |
| 未解析、安全规则不满足、系统同名、已知共享或不可检测 | 列表解释原因；打开编辑返回 `PROFILE_KEY_READ_ONLY` |
| TTL 过期、Profile 不匹配或重复提交 | `PROFILE_KEY_EDIT_EXPIRED`，不写入 |
| 所选条目未知、被删除、改名或替换，或来源不可读 | `PROFILE_PROVIDER_KEY_UNAVAILABLE`，提示重新打开；事务未写入或已验证恢复 |
| 配置或目标变量与编辑基线不符 | `PROFILE_KEY_CONFLICT`，重新打开后再核对 |
| 写入/提交失败且验证了原值与存在状态 | `PROFILE_KEY_WRITE_FAILED` |
| 无法验证原态恢复，或事务标记不能安全清理 | `ROLLBACK_INCOMPLETE`，保留待确认提示 |
| 写入和回读成功、系统通知失败 | 正常返回，`windowsNotified: false`，提示重新打开终端 |

## 5. 良好/基线/错误用例

- 良好：两个 Profile 引用不同专用变量，设置或更换一个后另一个不变，仍用各自的原生启动命令。
- 良好：当前启用其他供应商时仍可点选同 ID 供应商的任一命名 Key，不把 Providers 当前 Key 或 `auth.json` 一并切换。
- 良好：用户变量指向「备用」而 Providers 选中「主用」时，列表显示「备用」；用户变量在外部更改后可刷新重新匹配。
- 基线：只有 Relay 进程可见时可新建当前用户变量，但不能导入该进程中的值；`REG_EXPAND_SZ` 等类型外部管理。
- 错误：保存时重新选用依据前端输入指定的变量名、记录原 Key、或用「已生效」取代「已保存」。

## 6. 必需测试

用临时配置和可控环境存储校验叠加、作用域、保留名、共享、编辑令牌、冲突及错误恢复；验证同 ID 选项仅含摘要、其他条目和过时来源不能写入，断言默认配置、认证和密钥库不变。Windows 原生测试只使用随机 `HKCU\Software` 子键，验证事务内外读、提交、取消和竞争写不被覆盖；不得改真实的用户变量。Vue 测试通过 IPC mock 覆盖入口、遮挡、取消/卸载清空、按单一条目 ID 提交和保存后刷新。运行 `path_safety` 和完整 `npm run check`。

列表匹配专项须断言能识别未选中的同供应商 Key、共享变量仍只读、不同供应商或不同值不误报、仅进程可见不推断、缺失或损坏密钥库不产生新文件，且列表响应不包含值或条目 ID。页面专项须验证手动刷新和保存后以新读取得到的名称为准。

## 7. 错误与正确做法

错误：通过 `setx` 或前端构造命令把值写到用户环境，会泄漏参数且绕开事务和回读；也不要让前端读取 Key 管理器的明文来复用。正确：手填通过 `save_profile_env_key` 提交；复用通过 `apply_profile_provider_key` 提交条目 ID，后端解析密钥，由同一事务验证并写入基线目标。

错误：用 Providers 当前选中的 Key 名称填充 Profile 列表。正确：对该 Profile 的 `env_key` 读取持久用户变量，仅将后端精确匹配出的名称交给列表。
