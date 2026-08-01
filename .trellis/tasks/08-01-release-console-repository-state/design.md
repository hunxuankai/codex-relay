# 发布控制台仓库状态与版本展示设计

## 1. 设计目标

在不改变发布状态机、GitHub 写入边界和发布说明生成算法的前提下，完成四项用户可见改进：

1. 重新打开控制台时恢复最近一次预检成功的本地仓库路径。
2. 仓库预检摘要显示线上最新正式 Release tag。
3. 发布说明区域明确说明其由 Git 提交与固定模板生成，不调用 Codex。
4. 删除主界面的首版范围横幅。

## 2. 方案选择

采用版本化 WebView 本地偏好保存仓库路径，而不新增 Rust 配置文件、Tauri Store 插件或自动目录探测。

- 仓库路径是非秘密的 UI 便利偏好，不属于 Codex 配置、Codex Relay 正式应用数据或发布会话事实。
- 只在仓库预检成功后保存后端返回的规范化绝对路径，不保存尚未校验的输入，也不使用 watcher 保存每次击键。
- 存储不可用、JSON 损坏、schema 版本未知或字段无效时回退为空字符串；失败不阻断手动输入和发布流程。
- 发布会话继续保存在 worktree 的 Git 元数据目录；本地偏好只负责找到仓库，不替代会话状态或恢复验证。

未采用的方案：

- Rust 应用偏好文件：会增加路径解析、原子写、迁移和测试边界，对单个非敏感路径过重。
- 从 EXE 当前目录自动探测：便携程序的启动目录不稳定，容易选错仓库。

## 3. 组件与职责

### Vue

- `src/composables/useRepositoryPreference.ts`：拥有版本化存储格式、容错读取和显式 `remember` 动作；不依赖发布会话或 Tauri IPC。
- `src/App.vue`：保持组合面，绑定偏好中的 `repositoryPath`；预检成功后用后端规范化路径更新并保存；删除范围横幅。
- `src/components/release/RepositorySetupPanel.vue`：只展示 typed preflight DTO 中的线上 Latest，不自行解析 GitHub JSON。
- `src/components/release/ReleasePlanPanel.vue`：展示确定性、非 AI 的发布说明生成说明，不改变说明编辑和计划失效行为。

### Rust/Tauri

- `src-tauri/src/models.rs`：`ReleasePreflightResult` 新增规范化 `repositoryPath`；`ExternalPreflightSnapshot` 新增可空 `latestReleaseTag`。
- `src-tauri/src/services/release_application.rs`：复用现有 Release 列表响应，提取首个 `draft != true` 且 `prerelease != true`、拥有非空 `tag_name` 的 Release；构造预检 DTO 时返回规范化仓库路径和 Latest tag。
- Git、GitHub CLI 和 command 边界保持不变；不新增外部写请求，不读取 Token、认证文件或 Codex 数据。

## 4. 数据流

```text
应用启动
  → 读取 version=1 的 WebView 偏好
  → repositoryPath 输入框

点击“检查仓库”
  → typed Tauri client
  → ReleaseApplication canonicalize + Git/GitHub 只读预检
  → { repositoryPath, repository, external.latestReleaseTag }
  → App 更新并记住规范化路径
  → RepositorySetupPanel 展示“线上 Latest”
```

Release 列表同时承担两个投影：

- Draft 数量：阻止与既有 Draft 冲突。
- Latest tag：首个正式 Release；没有正式 Release 时为 `null`。

原始 GitHub JSON 只在 Rust 边界解析一次，Vue 只消费 typed DTO。

## 5. 存储契约

键名固定为 `codex-relay-release-console.repository-preference.v1`，值为：

```json
{
  "version": 1,
  "repositoryPath": "D:\\safe-temp\\repository"
}
```

规则：

- 只接受 `version === 1` 且去除首尾空白后非空的字符串。
- 读取、解析或写入异常全部静默回退；不把存储故障误报为仓库或发布失败。
- 只保存仓库路径，不保存 GitHub Token、Authorization Header、API Key、发布说明或会话证据。
- 测试使用内存 `Storage` 替身或 jsdom localStorage，不触及真实 `%USERPROFILE%\.codex` 和 `%LOCALAPPDATA%\CodexRelay`。

## 6. Latest 语义与错误处理

- `latestReleaseTag` 保留 GitHub tag 原文，例如 `v0.4.0`。
- Release 列表为空或只有 Draft/prerelease 时返回 `null`，UI 显示“尚无正式版本”。
- GitHub CLI 执行失败或 JSON 无效继续使用现有 `GITHUB_PREFLIGHT_FAILED` / `GITHUB_RESPONSE_INVALID`；不得显示缓存或伪造版本。
- recovery 预检不访问 GitHub，`latestReleaseTag` 为 `null`；该内部快照不用于仓库摘要展示。

## 7. 发布说明说明文案

发布说明区域显示：

> 根据 Git 提交与固定模板生成，不调用 Codex；正文会同时进入 GitHub Release 和 latest.json.notes。

本任务不改变 `ReleaseNotesService`：仍按 Conventional Commit 分类、过滤内部提交、保留提交主题原语言、生成固定中文章节并执行安全校验。

## 8. 测试策略

- 偏好 composable：有效恢复、损坏/未知版本回退、trim 后保存、存储异常容错。
- App：已保存路径启动即显示；预检成功后保存后端规范化路径；首版范围横幅不存在。
- Rust helper：跳过 Draft/prerelease，返回首个正式 tag；无正式 Release 返回 `None`。
- DTO/command：camelCase 字段包含 `repositoryPath` 与 `latestReleaseTag`。
- RepositorySetupPanel：显示真实 tag 和“尚无正式版本”空态。
- ReleasePlanPanel：显示“不调用 Codex”说明，既有编辑/重新生成门禁保持。

## 9. 兼容性与回滚

- 新字段仅用于同版本前后端；独立控制台不承诺与旧二进制混搭，现有 session JSON 不变。
- 删除 localStorage 键即可回到每次手填；字段损坏会自动空态回退。
- 回滚代码时无需迁移源码、Git 状态、Release 或正式 Codex Relay 用户数据。
