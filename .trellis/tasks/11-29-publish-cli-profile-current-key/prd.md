# 发布 CLI Profiles 密钥名称更新 v0.5.6

## 目标

公开发布包含 CLI Profiles 已保存密钥名称识别的 Codex Relay Windows 更新 `v0.5.6`（当前公开基线 `v0.5.5`）。用户已明确要求改完后发布更新。

## 需求

- 发布候选基于默认分支 `main` 上已交付的功能提交 `dca1f0a`，不引入其他产品或发布工具改动。
- 版本元数据一致，GitHub Release 与 `latest.json` 使用最终简体中文说明；保持现有 Tauri updater 公钥、签名流程、Windows NSIS 和历史 Release 保留策略。
- 只有完整本地与 CI 检查、普通构建及实际签名草稿审计通过后，才公开同一个草稿。禁止手工推送标签、读取私钥或接触真实用户配置。

## 验收标准

- [x] 发布候选通过 `npm run check`、普通 Windows 构建及发布请求校验，推送后远端 `main` 与候选 SHA 一致。
- [x] 只有一个与候选 SHA 绑定的 GitHub Actions 发布 Run，其成功生成 Draft；审计版本、正文、资产摘要、更新清单、安装器版本及实际 updater 签名。
- [x] `v0.5.6` 成为公开 Latest；在线 tag、匿名下载字节和签名与审计证据一致，核对历史发布保留结果，记录未执行的安装、升级和 Authenticode 验证边界。
- [x] 发布证据已记录；Trellis 归档、会话记录和最终主分支一致性以对应 Git 提交及远端 ref 回读为准。
