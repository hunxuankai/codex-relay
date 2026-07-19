# 安全说明

## 1. 适用场景与威胁模型

Codex Relay 面向单个 Windows 用户在个人计算机上管理本地 Codex Provider。它假设当前 Windows 账户及同账户进程基本可信，不提供多用户隔离、团队密钥权限、远程托管或抵御已获得当前用户文件访问权的恶意软件。

## 2. 明文密钥设计

API Key 会以明文存在于：

- `%LOCALAPPDATA%\CodexRelay\providers.json`
- 当前 `%USERPROFILE%\.codex\auth.json` 或 `CODEX_HOME\auth.json`
- 事务备份中的 `auth.json` / `providers.json` 快照

项目明确不使用 Credential Manager、Windows Keyring、DPAPI、Stronghold 或其他加密存储。原因是本项目定位为轻量个人工具，并需要与 Codex 现有明文 `auth.json` 行为兼容。这是有意识的便利性与安全性取舍，不代表明文存储是高安全方案。

不要共享这些目录，不要把它们上传到云盘、工单、聊天或公共仓库。共享计算机、高权限长期密钥、受监管环境和企业团队不应依赖这一设计。

## 3. 前端与命令边界

只有 `src/services/tauri.ts` 导入 `invoke`。普通 Provider 列表只返回 `apiKeyConfigured`，不返回密钥。专用密钥编辑命令要求具体 Provider ID；编辑器未触碰密钥时提交 `unchanged`，明确清空时提交 `clear`。

`CommandResult<T>` 的错误只包含稳定 code 和安全中文 message，不包含 Rust 堆栈、路径内部错误、文件全文或秘密。Tauri CSP 限制内容到本地应用资源和 IPC；运行时不加载网络字体、脚本或图片。

## 4. 日志与通知

日志层会脱敏 JSON 密钥、Bearer/Authorization、查询参数和结构化秘密。日志不记录 `auth.json`、`providers.json` 全文。滚动日志数量受限。

前端通知、Windows 通知、托盘标签和事件 payload 只包含 Provider 名称、状态与安全错误。测试快照也不得包含真实密钥。

脱敏是纵深防御，不是保存秘密的替代方案。新增日志前仍应避免把密钥传给日志格式化代码。

## 5. 路径安全

生产路径可来自 `CODEX_HOME`、`USERPROFILE` 与 `LOCALAPPDATA`。开发/测试使用专用 `CODEX_RELAY_CODEX_HOME`、`CODEX_RELAY_APP_DATA_DIR`。

测试模式要求两个 Relay 覆盖同时存在，并拒绝指向真实 `.codex`/CodexRelay 目录。集成测试把默认用户路径换成带损坏内容的临时哨兵，再验证哨兵树完全不变，从而避免任何测试接触实际用户数据。

`npm run dev:safe` 只在仓库 `dev-data` 中创建假数据。不要把普通 `npm run dev` 用于不确定的环境变量组合。

## 6. 配置完整性

所有写入通过全局事务锁、文件指纹、备份、同目录临时文件、解析验证、原子替换和写后验证。外部修改冲突会停止写入。TOML 使用 `toml_edit` 局部修改，避免重建导致注释和未知设置丢失。

回滚结果必须可验证。自动恢复不完整时保留事务标记，要求用户使用备份，不谎称原配置已恢复。

## 7. Provider 存储损坏

`providers.json` 无法解析时，应用先复制损坏文件到带时间/标识的备份，再返回错误。不会把损坏文件覆盖为空结构。用户需要重新设置相应 API Key，并检查损坏备份中是否包含仍有效的秘密。

## 8. 备份安全

`metadata.json` 不包含密钥，但文件快照包含原始字节，可能含全部 Provider 密钥。备份目录与主密钥文件具有相同敏感级别。卸载不会删除备份，避免擅自造成数据损失；这也意味着用户必须自行决定何时安全清理。

## 9. 系统集成

开机启动仅注册当前用户，不创建 Windows 服务，不修改所有用户启动项。关闭窗口默认不退出，真实退出必须通过托盘或首次引导的显式退出。Single Instance 防止多个进程同时管理同一配置，但事务层仍承担最终并发保护。

## 10. 构建与签名

Release 使用本地 CSP、当前用户 NSIS 安装与仓库图标。仓库不含签名证书。未签名构建可能显示“未知发布者”，不能把构建成功描述为已签名。

正式分发应：

1. 在干净环境运行完整检查与 Release/NSIS 构建。
2. 扫描仓库和产物日志中的秘密。
3. 使用可信代码签名证书签署主程序和安装器。
4. 用 Windows 工具验证签名与时间戳。
5. 记录实际 SHA-256、产物路径和签名结果。

## 11. 事件响应建议

若怀疑密钥泄漏：立即退出应用、在 Provider 平台吊销密钥、生成新密钥、检查 `auth.json`、`providers.json`、备份、日志、开发目录和同步软件，再更新目标 Provider。仅从界面清空密钥不会自动吊销远端凭据。
