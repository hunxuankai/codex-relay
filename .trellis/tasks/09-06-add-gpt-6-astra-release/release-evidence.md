# v0.5.1 发布证据

## 候选与授权

- 用户要求增加 `gpt-6-astra` 并发布更新；目标 `v0.5.1`，上一公开版本 `v0.5.0`（Release ID `364697150`）。
- 当前本地分支 `master`，已配置上游 `origin/main`；候选发布需要先推送源码供 GitHub 构建，发布证据、归档和日志提交随后统一最终推送。
- 两项 updater 签名 Secret 名称存在；未读取或输出秘密。Windows Authenticode 未启用。

## 本轮本地验证

- `npm run check`：退出 0；Trellis 8 项、根 Vitest 338 项、发布控制台 Vitest 89 项、Rust 463 项通过。
- Rust 1 项既有嵌套完整检查探针保持 ignored；已实际运行顶层完整检查，不把 ignored 计为通过。
- `path_safety` 3 项与 `provider_workflow` 2 项通过；所有本地执行使用系统临时目录下成对 Relay 路径覆盖。
- 无更新私钥普通 `npm run build`：退出 0；主程序和 NSIS 均为 `0.5.1`，未生成本次 updater `.sig`。

| 产物（仓库相对路径） | 字节数 | 最后修改 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19433984 | `2026-09-05T20:26:01.4680522Z` | `1ea25bad6d2225a114d35cbfae4d7a103350df29d60ad035114d230c12600a89` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.1_x64-setup.exe` | 4693216 | `2026-09-05T20:26:01.4049608Z` | `c942f046e20bb6eaf73f19335f2467b04edd3d15a4bd1dea46be55492785f3ec` |

两个产物的 `Get-AuthenticodeSignature` 状态均为 `NotSigned`；不能把后续 updater 签名描述为 Windows 发布者签名。

## 发布与审计进度

GitHub 候选构建、Draft 审计、公开及 Latest/历史清理核验尚未执行，完成后在本节记录实际证据。

## 限制与失败记录

- 回归测试在添加模型前按预期失败；实现后通过。
- 一次 Rust 专项在会话中断后失去终态输出，未报告成功；后续完整检查重新覆盖。
- Vite 输出第三方 PURE 注解和 chunk 大小提示，构建退出 0；没有放宽检查或修改依赖。
- 未执行真实安装、应用内升级、UAC、重启、卸载或 Sandbox/VM 人工观察。
