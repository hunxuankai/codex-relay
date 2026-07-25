# 实施计划：固定升级安装目录并延后旧版卸载

## 实施步骤

1. [已完成] 先为安装器结构契约补充红色测试：登记路径强制、原地升级标志、普通升级跳过目录页、移除并存选项、旧卸载不在确认前执行，以及新装/WiX 回归。
2. [已完成] 修改 `custom-installer.nsi`：读取登记路径时设置原地升级状态；普通 NSIS 显示本地升级说明并跳过目录页；让原地升级直接覆盖，不调用旧 NSIS 卸载器；保留新装、`/UPDATE` 和 WiX 兼容分支。
3. [已完成] 运行专项结构测试确认红转绿；检查 NSIS 预处理语法、路径恢复顺序和命令行边界。
4. [已完成] 同步 README、`AboutView` 及 `.trellis/spec/release/tauri-nsis.md`、`.trellis/spec/release/updater.md`、`.trellis/spec/project/product-contract.md` 的长期说明和错误矩阵。
5. [已完成] 运行 `trellis-check` 对应的专项测试、`npm run check`、`npm run build`，枚举实际 Release/NSIS 产物；构建证据与安装/升级证据分开记录。
6. [未执行] 当前环境查询或启用 Windows Sandbox 需要提升权限，未取得新装、同目录手动升级、应用内升级、取消、数据保留和旧版本兼容的真实 Sandbox/VM 证据；该项保留为发布前门禁。
7. [ ] 完成规范更新判断、精确审查 diff/密钥/路径，提交本任务文件；恢复当前 Provider 任务的活动指针，不提交其未完成改动。

## 验证命令

```powershell
npm run test -- src/release-config.test.ts
npm run typecheck
npm run test:trellis
npm run check
npm run build
git diff --check
git status --short --ignored
```

安装器构建后还要枚举：

```powershell
Get-Item src-tauri/target/release/CodexRelay.exe
Get-ChildItem src-tauri/target/release/bundle/nsis -Filter '*.exe'
Get-FileHash -Algorithm SHA256 src-tauri/target/release/CodexRelay.exe
Get-ChildItem src-tauri/target/release/bundle/nsis -Filter '*.exe' | Get-FileHash -Algorithm SHA256
```

## 风险与回滚点

| 风险 | 控制 |
|---|---|
| `$INSTDIR` 恢复遗漏导致 `/D=` 绕过保护 | 在 `.onInit` 和结构测试中锁定登记读取后覆盖顺序 |
| 普通升级仍调用旧卸载器 | 红测搜索 `PageLeaveReinstall`/`ExecWait` 的分支并构建 NSIS 预处理产物核对 |
| 新装目录页被误跳过 | 结构测试分别断言 FreshInstall、InPlaceUpgrade、WiX 路径 |
| 快捷方式/自启被意外删除 | 原地升级不调用普通卸载器；隔离安装验证实际 HKCU/HKLM 状态 |
| 数据目录被清理 | 只使用安装器模板/临时 fixture；禁止真实用户目录，Sandbox 报告只写无密钥哈希 |
| 当前 Provider 任务文件被混入提交 | 只精确暂存本任务文件，提交前按路径审查 `git diff` |

## 开始实施前检查

- [x] 用户已批准实施方向。
- [x] 新任务已与当前 Provider 任务分离创建。
- [x] 已读取 release、security、testing、project 相关规范索引及具体文件。
- [x] 复杂任务的 PRD、设计和实施计划已建立。
- [x] 红色结构测试已因缺少 `UpgradeMode` 原地升级契约而按预期失败。

## 进度与验证记录

- 2026-07-25：完成代码/文档探索，确认当前普通 GUI 升级在目录页之前调用旧卸载器，并允许并存安装；应用内 updater 已沿用登记目录。
- 2026-07-25：用户批准“升级固定原目录、移除并存选项、延后旧版卸载”的实施方向；尚未编辑运行时代码或运行本任务测试。
- 2026-07-25：新增 `src/release-config.test.ts` 原地升级结构测试；`npm run test -- src/release-config.test.ts` 运行 13 项，其中 12 项通过、目标测试因模板缺少 `Var UpgradeMode` 失败，确认红色原因正确。
- 2026-07-25：实现 NSIS `UpgradeMode`、登记路径强制恢复、普通升级说明页和 `SkipIfUpgrade`；专项结构测试 13/13 通过。Tauri `npm run build` 外层命令在 300 秒超时，但其 NSIS 子进程随后完成；生成的 `0.1.2` 安装器由独立 `makensis` 复核退出 0，警告仅为未引用的上游 `Skip` 函数。
- 2026-07-25：`npm run test -- src/release-config.test.ts src/views/AboutView.test.ts` 通过，2 个文件、14 项测试；用户可见安装目录说明同步完成。
- 2026-07-25：质量审查发现普通卸载会保留目录键但删除卸载登记；先补红测确认仅有目录键时不应进入原地升级，再让 `RestorePreviousInstallLocation` 同时检查 `UninstallString`。专项结构测试重新通过 13/13，保证“卸载后重装可选新目录”。
- 2026-07-25：最终 `npm run check` 退出码 0；Trellis 8 项测试、前端 36 个文件/169 项测试、Rust workspace 格式、Clippy、依赖图与全部测试通过。
- 2026-07-25：最终 `npm run build` 退出码 0；生成 `CodexRelay.exe`（18,890,240 字节，SHA-256 `9000427F141F58B0E0438AD0849BAEEFCF15E7288B7EBBD8CF09666A092360E7`）和 `Codex Relay_0.1.2_x64-setup.exe`（4,571,178 字节，SHA-256 `9FA1FD09E37C1F61DB8BA0A258F77D61E27C1B38333EE07ECC73D3097B75152C`）。这些证据只证明构建成功。
- 2026-07-25：Windows Sandbox 功能状态查询返回“请求的操作需要提升”，本轮没有真实安装、升级、取消或数据保留人工证据，不能声明这些场景成功。
- 2026-07-25：Phase 3.3 规范判断完成；安装目录双登记门禁、普通 NSIS 原地覆盖、WiX 例外、数据保留和必需测试已沉淀到 release/project 规范，其中 `tauri-nsis.md` 补齐基础设施契约的七段结构。
