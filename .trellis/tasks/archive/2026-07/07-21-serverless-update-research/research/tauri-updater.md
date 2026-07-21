# Tauri 无服务器更新调研证据

调研日期：2026-07-21。

## 仓库现状

- `package.json`、`src-tauri/Cargo.toml` 和锁文件没有 updater 插件。
- `src-tauri/tauri.conf.json` 只配置 `nsis`、`perMachine` 和自定义安装模板，没有 updater endpoint、公钥或 updater artifacts。
- `src-tauri/capabilities/default.json` 只有 core、autostart 和 notification 权限。
- `src-tauri/src/lib.rs` 集中注册 Tauri 插件，应用版本来自 `app.package_info().version`。
- `src/services/tauri.ts` 是唯一应用级前端 Tauri IPC 边界；`SettingsView.vue` 是合适的手动更新入口。
- 自定义 NSIS 模板保留 `/P`、`/UPDATE`、`/R` 相关重启路径、版本比较、per-machine 管理员执行级别和 `RestorePreviousInstallLocation`。
- Git 状态检查时没有 remote、tag 或 `.github` 工作流。

## 官方 updater 能力

官方文档：<https://v2.tauri.app/plugin/updater/>。

- npm 与 crates.io 查询到的 `@tauri-apps/plugin-updater` / `tauri-plugin-updater` 最新版本均为 `2.10.1`。
- Windows、Linux 和 macOS 受支持；本任务只采用 `windows-x86_64`。
- endpoint 可以是动态服务器，也可以是 GitHub Releases、S3 等托管的静态 JSON。
- 生产构建默认要求 TLS；本设计不启用不安全 HTTP 或无效证书选项。
- 静态 JSON 要求有效 SemVer，以及目标平台的 URL 和内联 `.sig` 内容。
- Tauri 更新包签名校验不能关闭；它与 Windows Authenticode/SmartScreen 发布者签名无关。
- Windows 默认 `passive` 安装模式；NSIS 参数为 `/P /R`，updater 额外传入 `/UPDATE`。
- 当前官方插件源码在启动 Windows 安装器前调用 Tauri cleanup，然后通过 `ShellExecuteW` 启动安装器并退出当前进程。UAC 取消可能导致旧应用已退出但未升级，因此不能预先显示成功。
- `bundle.createUpdaterArtifacts: true` 需要构建环境提供 Tauri 更新私钥。Tauri CLI `--config` 支持按顺序合并 JSON/JSON5/TOML 配置，因此可以只在发布覆盖中开启该选项，保持普通构建不依赖私钥。

## GitHub Actions

官方指南：<https://v2.tauri.app/distribute/pipelines/github/>。

- `tauri-apps/tauri-action` 在 2026-06-29 发布 `action-v1.0.0`。
- Action 可以创建 Release、上传 Tauri bundles、`.sig` 和 updater JSON。
- `uploadUpdaterJson` 默认开启；Windows 同时存在 MSI/NSIS 时需要显式 `updaterJsonPreferNsis: true`。
- `releaseDraft: true` 支持先生成 Draft 再人工发布；客户端的 `releases/latest` 不应消费 Draft。
- 发布 job 需要 `contents: write`，可以使用 GitHub 自动提供的 `GITHUB_TOKEN`。
- 更新私钥环境变量只应注入 Tauri 构建步骤；工作流中的第三方 Actions 应固定到核验过的不可变提交。

## 本机前置能力

- Windows 10 企业版 `10.0.19045`，64 位；系统报告 hypervisor 已存在。
- Node.js `22.20.0`、npm `10.9.3`、Rust/Cargo `1.97.1`、Git `2.45.1` 已安装。
- Tauri 已缓存 NSIS，并已有 `Codex Relay_0.1.0_x64-setup.exe` 构建产物；该旧产物不作为未来成功声明证据。
- GitHub CLI、Docker、VirtualBox 和 VMware `vmrun` 未发现，它们都不是推荐方案的必需依赖。
- 查询 Windows Sandbox 功能状态需要管理员权限，本轮未确认其是否已启用。

## 公开仓库

- 用户提供并创建了 `git@github.com:hunxuankai/codex-relay.git`。
- GitHub API 在本轮返回 `private=False`、`size=0`、默认分支 `main`；仓库公开但尚无提交、tag 或 Release。

## 结论

无需自建服务器即可实现完整应用内更新。最低维护路径是公开 GitHub Releases + 固定静态 `latest.json` + Tauri 更新包签名 + 官方 updater + 现有 NSIS。主要成本在首次信任根/Actions 配置和真实 Windows 升级验证，而不是自写下载或安装代码。
