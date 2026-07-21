# 发布更新操作文档设计

## 文档归属

完整维护者流程属于长期发布契约，放在 `.trellis/spec/release/publishing.md`。`.trellis/spec/release/index.md` 负责规范导航；README 只保留产品行为摘要和进一步阅读入口，避免同一套发布步骤在多个位置漂移。

## 内容结构

文档按真实状态转换组织：发布前确认 → 版本与说明准备 → 本地验证 → 提交推送 → 触发 Actions → Draft 核对 → 人工发布 → 发布后验证 → 失败处置。命令使用 PowerShell，版本使用占位符，避免绑定某个历史版本。

## 安全与证据边界

- 只记录 Secret 名称，不记录值或设置私钥的命令。
- 普通 `npm run build` 只证明未签名 Release/NSIS 可生成；GitHub Actions 才生成 updater 签名资产。
- Draft、公开发布和 Sandbox/VM 升级分别要求独立证据。
- Windows Authenticode 与 Tauri updater 签名分开陈述。

## 兼容与回滚

保持现有 `workflow_dispatch`、Draft 和 `v__VERSION__` 机制不变。错误 Draft 可以删除重建；公开资产不可原地替换，修复使用更高 SemVer。
