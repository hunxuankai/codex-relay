# 规范化发布控制台仓库路径显示

## 目标

让发布控制台在启动恢复、仓库检查成功和跨启动偏好恢复时，以用户熟悉的 Windows 路径格式显示并保存 Codex Relay 仓库路径，不再把 Rust 内部规范化产生的 `\\?\` 扩展长度前缀暴露到输入框。

## 背景与事实

- 当前失败会话的 `session.json` 中仓库路径为 `\\?\D:\Kai\Project\unifyProject\codex-relay`；该路径与普通 `D:\Kai\Project\unifyProject\codex-relay` 指向同一目录。
- `tools/release-console/src-tauri/src/services/release_application.rs:232` 使用 Windows `canonicalize()`，其结果可能带 `\\?\`。
- `tools/release-console/src/App.vue:63` 在仓库检查成功后直接保存后端 `inspection.repositoryPath`。
- `tools/release-console/src/composables/useRepositoryPreference.ts` 当前在加载和记住路径时只执行 `trim()`，因此扩展路径会跨启动保留并原样显示。
- “检测到发布会话”是终态会话的既有恢复行为，不属于本缺陷。

## 需求

### R1：用户友好的仓库偏好

- v1 仓库偏好加载到界面时，`\\?\D:\...` 必须显示为 `D:\...`。
- 仓库检查返回扩展盘符路径后，内存状态和 localStorage 都必须保存普通盘符路径。
- `\\?\UNC\server\share\...` 必须转换为标准 UNC `\\server\share\...`，不得错误变成 `UNC\...`。

### R2：安全兼容

- 已是普通盘符、标准 UNC 或其他普通字符串的路径只做既有首尾空白清理，不改变内容。
- 只转换明确匹配扩展盘符或扩展 UNC 的路径；未知 `\\?\Volume{...}` 等设备路径保持原样，避免生成不可用路径。
- 用户尚未通过仓库检查的输入仍只更新内存，不提前写 localStorage。

### R3：保持后端与恢复边界

- Rust 内部继续使用 canonical `PathBuf` 执行 Git、文件和会话操作，不改变路径安全与仓库身份检查。
- 不迁移或重写现有 `.git/codex-relay-release-console/session.json`；界面偏好兼容即可消除当前可见问题。
- 不隐藏或删除失败/完成会话的“查看上次结果”入口，不自动继续发布。

## 验收标准

- [ ] AC1：含 `\\?\D:\...` 的既有 v1 偏好在新 EXE 启动时显示为 `D:\...`。
- [ ] AC2：成功检查返回扩展盘符路径时，仓库输入框和 localStorage 均为普通盘符路径。
- [ ] AC3：扩展 UNC 转为标准 UNC；普通路径与未知设备路径保持正确。
- [ ] AC4：显式 `update` 仍不持久化，损坏/未知版本偏好与 storage 异常行为不回归。
- [ ] AC5：专项前端测试、发布控制台类型检查与测试、项目完整检查通过，发布控制台重新打包并记录 EXE 路径、大小、时间和 SHA-256。

## 范围外

- 不修改 Rust `canonicalize()`、Git workdir 或会话内部路径格式。
- 不清理、删除或自动忽略现有发布会话。
- 不增加仓库选择器、路径自动补全或通用 Windows 设备路径编辑能力。
