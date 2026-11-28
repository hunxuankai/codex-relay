# 设计

- 复用 `CliProfileCatalog` 解析出的 `env_key`、`provider_id` 和 `provider_defined_in_base`。名称与「更换 Key」的已保存来源使用同一个 ID 边界，不按供应商显示名或选中项回退。
- 复用 `UserEnvironment::user_value` 读取已有的用户范围状态。对非空普通字符串，在后端与已保存的完整值精确比较，沿用 `UserValue::text` 的编码和清零；现有原生适配与进程存在性检测不变。
- 列表请求内首次需要匹配时只读加载一次密钥库，局部持有，且不创建文件、修复损坏文件或记录错误明细。匹配结果与写入限制独立；不会把当前 Windows 用户变量误认为其他作用域中正在使用的 Key。
- 在现有 `ProfileKeyStatus` DTO 增加可为空的 `matchedUserKeyName`，仅返回命名元数据。没有合格匹配时返回 `null`，原有的变量检测状态继续区分未设置和无法检测。
- `CliProfilesView` 仍组织页面，`useCliProfiles` 仍负责刷新，`CliProfileList` 使用已有列表 props 渲染名称或未匹配提示。弹窗、Tauri 命令参数、配置格式、事务和持久化均不改变，无迁移需求。
