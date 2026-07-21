# 无服务器检查更新与应用内更新

## 目标

在不自建常驻后端服务的前提下，为 Codex Relay 增加由用户手动触发的检查更新与应用内安装能力。更新元数据和产物由公开 GitHub Releases 托管，下载内容经 Tauri 更新包签名校验后交给现有 per-machine NSIS 安装器升级。

## 背景与已确认事实

- Codex Relay 是面向 Windows 10/11 的 Tauri 2 桌面应用，当前只生成 per-machine NSIS 安装器（`src-tauri/tauri.conf.json`）。
- 当前产品契约把自动更新列为非目标，启动阶段不访问网络；本需求只增加显式手动更新，不改变启动无网络行为。
- 应用已有集中插件注册（`src-tauri/src/lib.rs`）、唯一前端 Tauri 边界（`src/services/tauri.ts`）、设置视图和安全通知组件。
- 自定义 NSIS 模板已保留版本比较、`/P`、`/UPDATE`、管理员执行级别、已有安装目录恢复和重启支持（`src-tauri/installer/custom-installer.nsi`）。
- 官方 Tauri v2 updater 支持 Windows、静态 JSON 和 GitHub Releases；更新包签名校验不能关闭。Tauri 更新签名与 Windows Authenticode 是两个独立机制。
- 仓库目前没有 Git remote、tag、`.github` 工作流或公开发布源；现有版本为 `0.1.0`。
- 当前机器已有 Node、Rust、Git 和 Tauri 缓存的 NSIS；GitHub CLI 不是必需依赖。Windows Sandbox 是否已启用需要管理员权限才能确认。

## 产品决定

- 使用一个公开 GitHub 仓库同时托管源码和 Releases，不建设动态 API 或常驻更新服务。
- MVP 仅支持 `windows-x86_64` 和一条稳定通道。
- 只在用户进入设置页并点击“检查更新”后访问网络；不在启动或后台定时检查。
- 使用官方 Tauri updater 下载、校验并调用 NSIS 被动更新；允许显示下载进度和 UAC。
- 保留免费的 Tauri 更新包签名；不使用 Windows Authenticode，也不把 SmartScreen 未知发布者作为阻塞项。
- Release 由 GitHub Actions 手动触发，先生成 Draft，人工检查后再发布。
- 验收采用脚本自动准备/核验加少量应用点击和 UAC 确认的半自动方式，优先使用 Windows Sandbox。

## 功能需求

### FR-1 手动检查

- 设置页显示当前应用版本和“检查更新”按钮。
- 创建应用、进入设置页或创建更新 composable 均不得调用远端检查；只有用户点击按钮才请求固定 HTTPS endpoint。
- 同一时刻只允许一个检查、下载或安装动作，旧响应不得覆盖新状态。

### FR-2 检查结果

- 明确区分“已是最新版本”“发现新版本”和“检查失败”。
- 发现更新时展示新版本、发布日期和按纯文本处理的 Release notes。
- 网络、代理、HTTP、JSON、SemVer 或目标平台错误不得影响 Provider 管理、自检、备份或设置等本地能力。

### FR-3 下载与安装

- 用户点击“下载并安装”后再次确认应用将退出且可能出现 UAC。
- 下载期间展示确定或不确定进度，并禁用重复操作；MVP 不要求取消或断点续传。
- Tauri updater 必须在启动安装器前验证更新包签名；验证失败不得执行下载内容。
- 校验通过后使用 NSIS 被动更新模式沿用原安装目录并重启应用；启动安装器前不得显示“更新成功”。
- UAC 取消、安装器失败或更新后无法启动均不得被描述为成功。UAC 取消后应用可能已经退出，用户可重新打开旧版本。

### FR-4 数据保留

- 更新不得读取、移动、覆盖或删除 `.codex`、`%LOCALAPPDATA%\CodexRelay`、API Key、日志、事务备份或损坏文件保留副本。
- 更新安装不属于 Provider 受管配置写入，不调用 `TransactionService`；NSIS 只替换安装目录中的程序产物。

### FR-5 发布

- 基础应用配置包含固定 GitHub Releases endpoint、公钥和 updater 权限；客户端不包含 GitHub 写入令牌或私钥。
- 发布专用 Tauri 配置覆盖开启 updater artifacts，普通本地构建不因缺少私钥而失败。
- GitHub Actions 只由 `workflow_dispatch` 触发，先运行完整检查，再构建 Windows x64 NSIS、`.sig` 和 `latest.json`。
- Release 初始为 Draft；人工确认资产和说明后发布，客户端才可通过 `releases/latest` 发现。
- 第一个带 updater 的版本仍需用户手动安装；只有后续更高 SemVer 才能应用内升级。

### FR-6 密钥与版本

- 更新私钥和可选密码只保存在 GitHub Actions Secrets 与开发者控制的离线备份中，不进入 Git、日志、任务材料或普通前端状态。
- 公钥是客户端信任根，可以公开提交；丢失私钥或更换信任根时，MVP 通过手动安装新版本恢复，不实现自动密钥轮换。
- `package.json` 是应用发布版本来源，Tauri 配置引用该版本；发布检查必须发现 Cargo 元数据或构建产物的意外版本漂移。

## 验收标准

- [ ] AC-1：自动化测试证明未发生用户点击时不会调用远端更新检查。
- [ ] AC-2：手动检查能够观察到最新、可更新和安全失败三类公开结果。
- [ ] AC-3：可更新状态展示纯文本说明；确认、进度、单飞控制和安全错误状态可通过组件/composable 测试观察。
- [ ] AC-4：签名错误或下载失败不启动安装器；任何安装路径都不在当前进程退出前报告成功。
- [ ] AC-5：基础构建不需要更新私钥；发布配置和 GitHub Actions 能生成 Draft Release 所需的 NSIS updater 资产、`.sig` 与 `latest.json`。
- [ ] AC-6：配置和结构测试锁定固定 HTTPS endpoint、公开公钥、`updater:default` 权限、NSIS 优先、手动触发和 Draft 发布。
- [ ] AC-7：在安全隔离的 Windows 环境中，真实旧版到新版升级沿用安装目录，重启后版本正确，测试配置、应用数据与备份保持不变。
- [ ] AC-8：断网、错误签名、下载失败和 UAC 取消不被报告为成功；未实际执行的场景不得声明通过。
- [ ] AC-9：README 和长期规范说明首个版本需手动安装、Tauri 签名与 Windows 签名的区别、发布步骤和人工恢复边界。
- [ ] AC-10：完整检查、构建、发布、安装和人工观察分别按本轮真实证据报告，不以旧结果或约定路径代替。

## 范围外

- 启动检查、后台轮询、强制更新、完全静默更新、下载取消、断点续传和版本跳过。
- 多发布通道、灰度发布、私有源认证、动态更新 API 和客户端 GitHub Token。
- Windows Authenticode、证书时间戳、SmartScreen 声誉或安全软件白名单。
- 自动二进制回滚、自动密钥轮换和安装失败后的自动恢复；保留旧 Release 供人工重新安装。
- Windows ARM、macOS、Linux、移动平台和 current-user 安装模式。

## 实施前置条件

- 公开 GitHub 仓库已创建为 `hunxuankai/codex-relay`；SSH 地址为 `git@github.com:hunxuankai/codex-relay.git`，HTTPS 更新清单地址为 `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`。
- 用户在安全位置生成 Tauri 更新密钥，把私钥/密码配置到 GitHub Actions Secrets，并保留离线备份；私钥不得发送到对话或写入仓库。
- Windows Sandbox 已启用，或提供可恢复快照的 Windows 虚拟机；自动化无法操作安全桌面时由用户确认 UAC。
- 用户审查本 PRD、`design.md` 和 `implement.md`，并明确批准开始实施后，任务才能从 planning 进入 in_progress。
