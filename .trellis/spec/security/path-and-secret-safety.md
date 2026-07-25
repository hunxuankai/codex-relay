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

fixture 只能使用明确标识的 `test-key-*-not-real`。专用密钥管理查询只可在用户明确打开
“管理与查看 API Key”对话框时返回目标 Provider 的完整命名密钥集合；普通 DTO、事件和刷新
不得复用该返回类型。完整密钥只能存在于短生命周期 manager，对话框关闭、scope dispose 和
请求作废时必须清空，晚响应不得恢复已关闭状态。

## 威胁模型

密钥在 `providers.json`、当前 `auth.json` 和事务备份快照中明文存在。本项目假设当前 Windows 账户和同账户进程基本可信，不抵御已获得当前用户文件访问权的恶意软件，也不适合共享账户、企业团队或受监管环境。

## 验证要求

- 路径安全集成测试必须建立默认路径哨兵并比较测试前后递归快照。
- 密钥扫描排除依赖和编译产物后仍需复核每个 `OPENAI_API_KEY`、Authorization、Bearer 命中。
- `git ls-files` 不得包含真实 `auth.json`、`providers.json`、备份或开发数据密钥文件。

## Provider 测试隔离契约

### 1. 范围/触发条件

用户显式点击测试时才允许短暂接触目标 Provider；所有自动生命周期路径仍保持离线。Codex 高级
测试必须先通过版本、managed requirements、临时路径、工具集合和进程门禁。

### 2. 签名

API/Codex command 只接收 Provider ID 与 UUID request ID。密钥由 Rust 的只读 Provider 解析边界
取得，绝不作为 IPC 参数、前端状态或 command 返回字段。

### 3. 请求/响应/环境契约

- 测试环境必须使用 `tempfile`/`AppPaths::for_test`，或成对的 `CODEX_RELAY_CODEX_HOME` 与
  `CODEX_RELAY_APP_DATA_DIR` 覆盖；缺一即失败，不回退生产路径。
- Codex 子进程环境先清空，再加入白名单、临时 Home/SQLite/workdir 和每次运行唯一的假 key 环境变量。
- 真实 key 只可在 Rust gateway 的上游 Authorization Header 短暂存在；不得落盘或进入 argv/stdout/stderr。
- Provider 目标解析和 API 代理读取必须采用只读加载；缺失的 `providers.json`/`settings.json` 不得
  因测试而自动创建，损坏文件也不得为了测试自动生成备份。
- 用户显式 API 测试可把实际请求 JSON 和最多 256 KiB 的响应正文作为会话内 trace 返回，但不得
  返回 Header、URL userinfo/敏感查询值、当前真实 key、代理地址或其他运行环境。响应意外回显当前 key 时在 Rust 网络边界
  移除；trace 不写入磁盘、日志、通知、事件、快照或普通 Provider DTO，Codex 结果不携带 trace。

### 4. 验证与错误矩阵

| 违规 | 结果 |
|---|---|
| 测试路径缺覆盖或指向真实用户目录 | `TEST_PATH_OVERRIDE_REQUIRED` / `UNSAFE_TEST_PATH`，立即停止 |
| 临时根不在系统 temp 或命中受保护根 | `CODEX_PREFLIGHT_FAILED` |
| key 出现在公开 DTO（包括 API trace）、日志、快照或命令行 | fail closed，并将泄漏视为测试失败 |
| 清理前路径验证失败或目录仍存在 | `CODEX_CLEANUP_FAILED`，不得声称已清理 |

### 5. 良好/基线/错误用例

- 良好：默认目录放置哨兵，测试全部在独立 temp 根完成，前后快照字节和存在状态完全一致。
- 良好：测试所用临时应用数据缺少密钥/设置文件时，测试前后仍保持缺失状态。
- 基线：fixture 使用 `test-key-provider-a-not-real` 等明确假 key。
- 错误：为方便调试读取 `%USERPROFILE%\\.codex\\auth.json`、复制真实 Home 或把用户提供的 key
  放进实验 fixture。

### 6. 必需测试

运行 `path_safety`、Codex invocation 环境白名单、argv/Debug/JSONL 脱敏、gateway Header 注入和
清理失败测试；每项都断言真实默认路径哨兵不变，并扫描 Git/任务材料没有真实 key。

### 7. 错误与正确做法

#### 错误

```rust
let home = dirs::home_dir().unwrap().join(".codex");
run_codex(home);
```

#### 正确

```rust
let layout = CodexTempLayout::new()?;
let invocation = build_invocation(..., layout.home(), layout.sqlite_home(), ...)?;
```

即使测试最终失败，也必须终止整个 Windows 进程树并验证临时目录清理；`kill_on_drop` 或
`taskkill` 不能替代 Job Object 安全门禁。
