# 技术设计

## 目录与输入校验

继续以 Rust `MODEL_CATALOG` 为唯一能力来源，仅移除 Astra 的 `ultra`。界面消费后端 DTO，`validate_preference` 保持严格；不增加前端模型特判。

## 旧偏好兼容

仅在当前 v4 文件的读取边界、严格校验之前，将明确的 `gpt-6-astra` → `ultra` 转成 `max`。此组合由 v0.5.1 实际写出，映射到最高有效档位可保留原先选择最高推理强度的意图。

转换返回是否发生变化，沿用 `LoadedProviderPreferenceStore.needs_upgrade` 与既有 Provider 事务写入流程。读取不写盘，原始指纹和备份仍以磁盘字节为准；显式操作才写入规范化值。保留既有升级时的 Provider 校准规则。

`serialize_store` 和用户选择仍调用严格校验，不接受新的 `ultra`。不泛化未知强度、不修改其他模型、不变更 v1/v2/v3 的已建立迁移规则。

## 验证与回滚

公共服务集成测试使用 `tempfile` 与 `AppPaths::for_test`，覆盖目录、合法选择、非法输入无写入、旧偏好只读加载、显式事务持久化及备份恢复。模块测试补充兼容范围与严格序列化。

任何受管文件写入继续使用现有 `TransactionService`；不引入直接生产文件写入。测试直接写入仅用于安全临时目录中的旧文件夹具。
