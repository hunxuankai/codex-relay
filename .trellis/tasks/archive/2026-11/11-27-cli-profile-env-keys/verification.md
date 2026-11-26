# 验证记录

## 行为与隔离

- 解析、作用域和保存先执行各自红测试，再用临时 `AppPaths` 与可控环境存储验证。覆盖叠加、损坏隔离、大小写共享、系统同名、配置或原值变化、回读失败、补偿和未知结果；认证文件和非目标变量没有被修改。
- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core --lib native_registry_transaction`：两项原生测试通过。使用独立的 `HKCU\Software` 测试子键，验证提交、显式取消、句柄关闭回滚及竞争写入；竞争方写成功，原生事务提交失败，外部值保留。全量检查回归了提交前的事务外回读路径。
- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay --lib cli_profile_commands`：一项通过，确认 Tauri 的安全模式能列出 Profile，但不能打开真实环境变量编辑事务。
- Codex CLI 0.167.0：在临时 `CODEX_HOME` 和清理过的子进程环境中，分别用 `codex -p agentrouter`、`codex -p qiandao` 的 `exec` 路径复用基础供应商和地址。两次请求均到达隔离 loopback `/v1/responses`，按次命中预期假 Key；mock 刻意返回 HTTP 422，不宣称模型调用成功。最初尝试的 `app-server config/read` 路线被 CLI 拒绝，因为 `app-server` 不接受 `-p`。
- Vitest 覆盖首次引导、复制启动命令、只读原因、遮挡输入、取消/卸载清空、显式保存与结果刷新。Chrome 用 mock IPC 检查 900×620 浅色和 560×620 暗色，无横向溢出；两项 Profile 操作在常规宽度可见。核对输入关联标签、36px 高度、自动聚焦、Escape 取消后焦点返回，以及 Enter 提交后的列表状态更新。

## 全范围检查

- `npm run check`：退出码 0。Trellis 8 项、前端 59 文件/331 项、Release Console 独立前端 17 文件/107 项，另通过两套类型检查、Rust 依赖图、fmt、Clippy 和 workspace 测试。Release Console 有一个既有的递归完整检查用例按配置忽略。
- `cargo test --manifest-path src-tauri/Cargo.toml --workspace --test path_safety`：3 通过，验证默认路径哨兵和缺少覆盖不回退。
- `npm run build:frontend`：退出码 0。Vite 报告既有第三方注解处理，并提示入口 chunk 约 504 kB；不是失败项。

## 未覆盖

未读取或修改真实 Codex/Relay 数据与生产环境变量；没有实测已安装桌面应用的 Windows 设置入口、向真实桌面广播后由终端继承环境、安装器、签名或发布。持久写入、广播与终端刷新是三个不同结果，不能用浏览器 mock 代替原生桌面验证。
