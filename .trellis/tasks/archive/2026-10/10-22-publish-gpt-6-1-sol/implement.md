# 实施与验证记录

## 计划

- [x] 读取当前上下文和发布、安全、验证规范；核对用户发布授权。
- [x] 核对公开 Latest、目标未占用、Run 队列、GitHub Actions 状态与 Secret 名称。
- [x] 完成 PRD、设计和计划收敛审查，无待用户决定事项。
- [x] 激活任务，核对 CI Rust 工具链，准备安全临时执行根。
- [x] 同步目标 0.5.3 的六文件和最终发布说明，运行结构/请求验证。
- [x] 本轮完整检查与无 updater 私钥普通构建，记录实际产物。
- [x] Trellis inline 审查、规范更新判断、精确提交和普通推送候选，核对远端 SHA。
- [x] 仅触发一个目标发布 Run，记录排队/执行、步骤与最终结论。
- [x] Draft 完整审计、独立更新签名验证、再审同一身份并公开。
- [x] 公开下载/Latest/tag/清单/资产复核，监控并核对历史清理。
- 收尾执行门禁：提交最终证据，归档、日志、最终普通 push 与分支/SHA 核对；按 Phase 3.4 → 3.5 顺序执行，最终命令证据在交付响应中报告。

## 验证命令

- `npx vitest run src/release-config.test.ts src/release-request.test.ts src/release-retention.test.ts`
- `scripts/validate-release-request.ps1`（绑定目标版本和实际候选 SHA）。
- `npm run check`；移除进程签名变量后 `npm run build`。
- 实际 EXE/NSIS 枚举、SHA-256 和 Authenticode 状态核对。
- 生产 Draft/公开审计和独立 minisign 验证；公开无认证下载和资产哈希复核。
- `git diff --check`、暂存差异/秘密检查、HEAD/上游/远端 SHA 一致性。

## 当前检查点

2026-10-22 恢复会话：`v0.5.3` 已于 UTC `14:29:33` 公开，Release ID `420770763`。公开生产审计、无认证下载验证及同一 cleanup Run 均已通过，Release/Tag 列表只保留 `v0.5.3`。发布执行完成，进入证据提交及 Git 收尾；没有重复 dispatch。

- 本轮官方 stable manifest 为 `1.99.1`（2026-10-15），已安装独立工具链，不改变默认 1.97.1。`rustc 1.99.1` 和 `clippy 0.1.99` 用于本地完整检查及普通构建。
- 结构专项 3 文件/19 项通过；发布请求预检退出 0，正式候选提交后已按实际 SHA 复验通过。
- `npm run check` 退出 0：Trellis 8 项、根前端 60 文件/356 项、控制台前端 17 文件/107 项、Rust 485 项通过及 1 项既有 ignored；类型检查、fmt、严格 Clippy、依赖图和路径安全通过。新工具链 Clippy 编译约 3 分 30 秒、Rust 测试编译约 10 分 50 秒。
- 从检查/构建子进程移除两个 `TAURI_SIGNING_*` 变量，使用成对临时 Relay 覆盖。`npm run build` 退出 0，Release 编译约 7 分 51 秒，实际主程序与 NSIS 均为 0.5.3；未生成本次 `.sig`/清单，Authenticode 均为 `NotSigned`。
- 产物、哈希与未执行行为详见 `release-evidence.md`。临时执行根为系统 TEMP 的 `CodexRelay-release-0.5.3-d7392640721441a7933e8b145e96a402`，日志不入 Git。
- Trellis inline 审查：仅六个版本/说明文件变化，锁文件依赖版本未漂移；无新运行时代码，不增加同义反复测试。规范更新判断：发布流程契约不变，无需修改长期规范。
- 已知探测失败：初次按 `cleanup-releases.yml` 查找文件返回不存在，已确认真实文件名；初次 PowerShell stable manifest 正则未匹配，改用 Python 标准库 TOML 解析取得真实版本。均未修改产品或工作流。
- 恢复时核对 `HEAD`、`origin/main` 和 `git ls-remote origin refs/heads/main` 均为 `d381c9a1eb0057ce1495da9cf54fcb38cc9c43fb`，工作区原先干净。
- 临时目录中的 `dispatch.json` 与 GitHub 实时查询一致：唯一发布 Run `39785972510`，`workflow_dispatch` / `main`，候选 SHA 匹配，完整检查和 Draft 构建均成功。
- 恢复首次确认 Draft Release ID `420770763`、`target_commitish` 和三个预期资产，随后另行完成完整字节与签名审计。
- 恢复轮次完整 `npm run check` 已退出 0（UTC `14:12:47` 至 `14:27:18`）：Trellis 8、根前端 356、控制台前端 107、Rust 485 项通过及 1 项既有 ignored。
- 真实生产服务 Draft 审计与独立 minisign 校验已退出 0；三项资产大小、SHA-256、GitHub digest、清单说明及签名关联一致，篡改安装器的负向对照被拒绝。
- 生产 `publish_release` 在 PATCH 前重新完成同一 Draft 审计，随后公开同一 Release ID，退出 0；公开 UTC `2026-10-22T14:29:33Z`。
- 公开后生产审计与独立 minisign 验证退出 0；无认证下载复核于 UTC `14:33:31` 退出 0，固定 Latest/版本清单、资产 API 和手动安装器下载均与 Draft 字节一致。
- cleanup Run `39796115034` 最终 success。首次监控返回 `BackendFailed` 后仅恢复只读查询，没有重跑；GitHub 官方事故说明结果回传延迟，实际 Job UTC `14:29:40` 至 `14:29:46`，不能把状态等待时间计为清理执行时间。
- 清理前：公开 `v0.5.2`（386122082）和 Draft `v0.5.3`（420770763），远端 tag 只有 `v0.5.2`；清理后：仅正式 `v0.5.3`（420770763）、三个预期资产及 `v0.5.3` tag。
- 下一步只提交本任务三份更新材料，随后归档、记录日志并普通推送已配置的 `origin/main`；最终核对工作区、分支和本地/远端 SHA。

## 未执行项

本轮未执行真实安装、应用内升级、UAC、自动重启、卸载或真实用户数据保留验证。
