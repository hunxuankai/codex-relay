# 路径与密钥安全

## 真实目录红线

开发和自动化测试严禁读取、写入或删除真实：

- `%USERPROFILE%\.codex`
- `%LOCALAPPDATA%\CodexRelay`

测试只能使用 `tempfile`、`AppPaths::for_test`，或同时设置：

```text
CODEX_RELAY_CODEX_HOME
CODEX_RELAY_APP_DATA_DIR
```

任一覆盖缺失必须返回 `TEST_PATH_OVERRIDE_REQUIRED`；覆盖指向真实目录必须返回 `UNSAFE_TEST_PATH`。测试构造器不得回退到生产路径。

开发完整应用优先运行 `npm run dev:safe`。普通 `npm run dev` 只有在当前终端已成对设置安全覆盖时才允许。

## 生产路径优先级

Codex 配置：Relay 覆盖 → `CODEX_HOME` → `%USERPROFILE%\.codex`。应用数据：Relay 覆盖 → `%LOCALAPPDATA%\CodexRelay`。不得写死用户名或本机绝对路径。

## 密钥边界

真实 API Key、Authorization Header、完整 `auth.json` 和完整 `providers.json` 不得进入：

- Git、fixture、文档示例或 Trellis 任务/spec；
- 日志、通知、事件、测试输出和快照；
- localStorage 或普通前端全局状态；
- 工单、聊天、云盘或共享目录。

fixture 只能使用明确标识的 `test-key-*-not-real`。专用密钥编辑接口可在用户明确打开编辑器时短暂返回目标密钥，但不得持久化或扩散。

## 威胁模型

密钥在 `providers.json`、当前 `auth.json` 和事务备份快照中明文存在。本项目假设当前 Windows 账户和同账户进程基本可信，不抵御已获得当前用户文件访问权的恶意软件，也不适合共享账户、企业团队或受监管环境。

## 验证要求

- 路径安全集成测试必须建立默认路径哨兵并比较测试前后递归快照。
- 密钥扫描排除依赖和编译产物后仍需复核每个 `OPENAI_API_KEY`、Authorization、Bearer 命中。
- `git ls-files` 不得包含真实 `auth.json`、`providers.json`、备份或开发数据密钥文件。
