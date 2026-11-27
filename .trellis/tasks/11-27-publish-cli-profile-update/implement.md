# 实施与验证

## 计划

1. 核对公开版本、Git 上游、待发改动、目标 tag/Draft 与活动 Run；仅验证 Actions Secrets 名称及 Actions 服务状态。
2. 同步 `.github/release-notes.md`、`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/crates/codex-relay-core/Cargo.toml`。只改版本和说明，无新业务行为需要额外 TDD 切片。
3. 运行发布专项测试、`scripts/validate-release-request.ps1`、`npm run check` 与普通 `npm run build`；使用成对安全临时路径、移除进程内签名变量、记录退出码和实际产物。审查差异、已跟踪敏感文件与高置信度凭据模式。
4. 精确暂存并普通推送候选至已配置 `origin/main`，核对远端 SHA；唯一 dispatch 绑定 `0.5.4` 与该 SHA，记录 Run ID、可恢复检查点、Step 和结果。
5. 审计实际 Draft 与更新签名；重新确认同一 ID、资产和目标，单次公开；复核匿名 Latest、tag、下载字节和历史清理 Run、最终 Release/Tag 列表。
6. 补齐验证记录，检查是否有长期契约变化，精确提交、归档、记录会话日志，最终普通推送并核对远端与本地 `HEAD`。

## 检查点

- 规划：已确认 `v0.5.3` 为公开 Latest，只有 `main` 和对应上游，无并行发布 Run；发布工作流已启用，两个 updater Secrets 名称均存在。下一步同步候选并启动门禁。
- 首次完整检查（Rust 1.99.1）前端通过，严格 Clippy 因 `chunks_exact_to_as_chunks` 退出 101；已用 core 专项 Clippy 在原候选上复现。Windows UTF-16 解码改用固定宽度切片，保留奇数长度、缺少终止符、内嵌 NUL 与无效 UTF-16 的拒绝行为；不降级检查或写入边界。
- 实测 CI 所用 stable 为 Rust 1.100.0。已单独安装该工具链并核对 `rustc`/Clippy，使用它重跑完整检查和普通构建；没有切换全局默认版本。
- 完整检查重跑退出 0，UTC `05:20:05` 启动不含签名变量的普通 Windows 构建。待枚举并核验实际产物后方可推送候选。
- 本地构建已于 UTC `05:31:02` 退出 0；主程序与 NSIS 的版本、时间、大小、哈希和无 Authenticode 状态已核对。下一步精确推送候选、唯一触发 `release.yml`。
- 候选 `094d96ff6c7b569d442f85da84d7fa5be6821d31` 已推送，远端和上游核对一致。Run `45946562364` 已绑定本版候选并开始完整检查；后续只监控此 Run。
- Run `45946562364` 成功；Draft `460212909` 三项实际资产、发布说明、清单和签名审计成功。接下来重审同一对象后公开并复核下载及历史清理。
- 正式 Release `460212909` 已公开，生产审计、匿名 Latest 和真实下载校验通过。清理 Run `45947994962` 成功，旧 Release、三项资产与远端 tag 均已消失。
- 最后提交本次证据、归档任务和会话日志，再推送并核验主分支。未执行真实安装或更新操作，不把托管验证当成升级验证。
- 遇到中断以候选 SHA、Run ID、Release ID 逐级恢复；没有明确终态时不重复 dispatch 或 publish。
