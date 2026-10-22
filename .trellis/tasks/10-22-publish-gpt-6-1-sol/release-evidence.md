# v0.5.3 发布证据

## 初始与候选

- 初始 main：`80b1730069fe408201c50518481d9ee125c6b301`，与 origin/main 一致。
- 上一公开版本：v0.5.2，Release ID `386122082`，目标 `fe39d72a2f88898ce1db7e476cacac1c90f34857`，公开 UTC `2026-09-10T11:04:53Z`。
- 目标：v0.5.3；预检确认无目标 Release/Tag，无活动发布 Run，Actions operational，updater Secret 名称完整。

## 本地验证

Rust 1.99.1 / Clippy 0.1.99 / Node v22.20.0。发布结构专项 19 项和最终 `npm run check`、普通 `npm run build` 均退出 0。
完整检查：Trellis 8 项，根 Vitest 356 项，发布控制台 Vitest 107 项，Rust 485 项通过、1 项既有 ignored。
检查/构建使用成对临时 Relay 路径，不接触真实 Codex/Relay 数据；构建进程没有 updater 签名变量。

| 本地产物 | 字节 | 最后写入 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19446272 | 2026-10-22T12:20:39.7281823Z | `f29868a88ba9663de83cb94097eb13492cd2beb22426bec659f1ca18e34f70bc` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.3_x64-setup.exe` | 4704881 | 2026-10-22T12:20:39.6696330Z | `7999ca6b1286ddde373b329ddffb475cad7de67365ff7bfdd1ea9e1732d9c5f6` |

两个产物 FileVersion 均为 0.5.3、Authenticode 均为 `NotSigned`。普通构建没有生成本版 `.sig` 或 `latest.json`。

## 后续证据

候选提交、远端 SHA、唯一发布 Run、Draft 审计、更新签名、公开端点和清理结果在实际发生后追加；当前不宣称已发布。

## 限制

未执行真实安装、应用内升级、UAC、自动重启、卸载或真实用户数据保留测试。构建/托管发布/更新签名证据不能替代这些行为。
