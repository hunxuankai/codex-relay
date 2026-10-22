# 技术设计

## 版本和内容

目标 `0.5.3`，沿用固定六文件：`package.json`、`package-lock.json`、主应用和 core 的 `Cargo.toml`、workspace `Cargo.lock`、`.github/release-notes.md`。Tauri 版本来自 npm manifest，无需更改配置或信任根。发布说明以 GPT-6.1 Sol 为主，并列出已包含的发布控制台修复及从 v0.5.2 升级方式。

## 执行与安全边界

由主会话 inline 编排已存在的命令与发布服务。为便于自动核验，允许在系统临时目录构建独立小型探针，直接复用控制台 `SystemGhBackend`、`DraftAuditService` 和 `GithubReleaseService`；不改产品控制台源码、不读取签名私钥、不启动真实 Codex 配置管理应用。探针复用 workspace target，运行时不输出秘密。

本地检查/构建成对设置安全 Relay 路径；构建进程移除两个 `TAURI_SIGNING_*` 变量。先确认当前 CI stable Rust 版本，再使用相应已安装或独立工具链核验；不降低 CI lint 规则。日志与下载留在安全临时根，任务材料只保存非秘密摘要与哈希。

## 发布和复核

先完成结构测试、完整检查与普通构建，精确暂存候选及任务材料并普通推送 `origin/main`。触发固定 `release.yml`，输入 `expected_version` 和 `expected_sha`，唯一定位并监控该 Run。轮询只读，不重复触发。

Draft 审计绑定 `target_commitish`，确认仅安装器、`.sig`、`latest.json` 三项资产，核对说明、两种 Windows 平台 URL、内联/独立签名和 GitHub digest。再用仓库公开 minisign 公钥验证实际安装器签名。审计通过后重审同一身份并 PATCH `draft=false`，沿用用户本次发布授权。

公开后重新核对 Latest、tag ref 与候选，使用不带认证的 updater 等价下载验证内容。清理只由仓库既有工作流按新标签/发布时间识别，确认旧 v0.5.2 Release/资产/tag 被清理且新 Latest 保留。

## 失败和恢复

本地或 CI 失败保留原证据并修复后产生新候选；不把失败重写成成功。Draft 有误不公开；公开后不得替换同版本资产，只能更高 SemVer 修复。push 失败保留本地提交。安装、UAC、重启等行为与托管发布证据严格分开。
