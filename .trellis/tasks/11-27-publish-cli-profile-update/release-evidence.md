# v0.5.4 发布证据

## 预检与回退基线

- `main`、`origin/main` 与远端 `refs/heads/main` 初始均为 `08dd3ad5d85c2cc8de8396a48383743e3638fbf8`。仅有主分支和一个工作树；目标版本尚无 Release/Tag，无活动发布 Run。
- 公开 Latest `v0.5.3` 为 Release `420770763`，tag 绑定 `d381c9a1eb0057ce1495da9cf54fcb38cc9c43fb`。本轮已匿名下载和核对旧固定 Latest、带 tag 的清单与三项资产，和上次记录的实际 SHA-256 一致；未运行安装器。
- 发布工作流启用，两个 updater Actions Secrets 的名称存在，未读取值。初始 GitHub Actions、Git Operations 和 API Requests 状态均为 `operational`。

## 本地候选

- 更新六个版本与说明文件；应用 Tauri 配置、更新公钥和安装器模板不改。发布结构、请求和历史清理专项 19 项通过；候选内容的独立发布请求校验退出 0。
- 首次 `npm run check`（Rust 1.99.1）前端 332 项、发布控制台 107 项通过；严格 Clippy 以 `chunks_exact_to_as_chunks` 拦下 Windows 环境变量解码，整体退出 101，未启动构建。core 专项 Clippy 在修复前复现退出 101，改用固定两字节切片并显式检查余数后退出 0，未屏蔽 lint。
- Rust 1.100.0 下直接对比修复前后两段解码函数：覆盖全部 65,536 个 UTF-16 单元的正常对齐与奇数长度输入，以及缺终止符、内嵌 NUL、合法代理对等，总共 131,077 组结果一致。
- GitHub Actions 使用 `stable`；根据官方工具链清单安装 Rust 1.100.0，未切换全局默认。安装器的自动 rustup 自更新报告 `cargo-miri.exe` 已存在；随后的 `rustc +1.100.0` 和 Clippy 版本检查退出成功，独立工具链可用。
- 所有本地验证使用系统临时根的成对 Relay 路径，进程中移除 updater 签名环境变量。未读取真实 Codex/Relay 数据或签名 Secret。
- 最终 `npm run check` 在 UTC `05:01:54` 开始、`05:20:05` 退出 0。Trellis 8 项、主前端 59 文件/332 项、发布控制台 17 文件/107 项、Rust 462 项通过、0 失败、1 项既有 ignored；类型检查、格式、严格 Clippy、依赖图与路径安全门禁通过。Rust 测试编译 9 分 20 秒；随机隔离注册表子键中的原生提交、回滚与竞争写测试也通过。
- 既有规范已要求匹配 CI stable 工具链、严格 Clippy 和安全路径；本次等价语句重写未引入新产品契约，不修改长期规范。

### 普通 Windows 构建

`npm run build` 在 UTC `05:20:05` 开始、`05:31:02` 退出 0，Rust 优化构建 9 分 56 秒。无 updater 签名变量，未生成本版 `.sig` 或 `latest.json`。两项产物的 FileVersion 均为 `0.5.4`，Authenticode 均为 `NotSigned`。Vite/Rollup 报告既有依赖注释与大包非阻塞警告。

| 产物 | 字节 | 最后写入 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19454464 | 2026-11-27T05:31:02.8892989Z | `acd4208c41e66c36a3a403238cf8bfdb11729078e05b2e5928a918ac865b3247` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.4_x64-setup.exe` | 4714482 | 2026-11-27T05:31:02.7586486Z | `9478a94d674f8c0477b253ee3dc557b21518c263d4cf4bbd1ae387372b17f9b5` |
