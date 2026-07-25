# Tauri 与 NSIS 发布契约

## 1. 范围与触发条件

修改 Tauri Windows bundle、NSIS 模板、安装范围、默认目录、已安装版本升级、旧安装迁移或卸载行为时，必须遵循本规范。安装器是 `perMachine` 基础设施边界，涉及 HKLM、管理员权限、程序文件和用户数据保留，因此结构测试、实际构建与隔离安装验证必须分别取得证据。

## 2. 签名与入口

- Release + NSIS：`npm run build`。
- 带 updater artifacts 的发布构建：`npm run build:release`。
- 显式 NSIS bundle：`npm run bundle:nsis`；成功的 Release bundle 后不重复运行。
- Debug：`npm run build:debug`，不生成安装包。
- 模板：`src-tauri/installer/custom-installer.nsi`，基于 Tauri 2.11.4 官方 `installer.nsi`。
- NSIS 状态：`$PassiveMode`、`$UpdateMode`、`$UpgradeMode`、`$WixMode`。
- 关键入口：`.onInit`、`RestorePreviousInstallLocation`、`PageReinstall`、`PageLeaveReinstall`、`SkipIfUpgrade` 和安装/卸载 Section。
- 登记范围：当前 `perMachine` 安装使用 HKLM；产品目录来自 `${MANUPRODUCTKEY}`，有效 NSIS 安装还必须存在 `${UNINSTKEY}` 的 `UninstallString`。

## 3. 契约

### 新鲜安装目录

在 `.onInit` 且 `$INSTDIR` 仍为 Tauri 占位值时：

1. 调用 `GetDriveTypeW("D:\\")`。
2. 结果为 `DRIVE_FIXED`（3）时使用 `D:\Program Files\${PRODUCTNAME}`。
3. 否则按目标架构使用 `$PROGRAMFILES64` 或 `$PROGRAMFILES`。
4. 选择默认值后调用 `RestorePreviousInstallLocation`；只有有效的已登记 NSIS 安装才能覆盖新鲜安装目录。

新鲜安装保留标准 `MUI_PAGE_DIRECTORY` 和 `/D=` 语义。普通卸载会保留上次目录键但删除卸载登记；只有 `${MANUPRODUCTKEY}`、没有 `UninstallString` 时必须按新鲜安装处理，允许重新选择目录。

### 已登记 NSIS 升级

`${MANUPRODUCTKEY}` 与 `UninstallString` 同时非空时，`RestorePreviousInstallLocation` 必须恢复 `$INSTDIR` 并设置 `$UpgradeMode = 1`。该状态具有以下约束：

- `/D=` 不能绕过登记目录；普通 NSIS 和应用内 updater 都沿用原目录。
- `PageReinstall` 只显示中英文原地升级说明，不渲染“不要卸载/并存安装”单选项。
- `SkipIfUpgrade` 跳过目录页；用户若要换位置，必须先从 Windows 卸载旧版，再重新安装。
- `PageLeaveReinstall` 不调用普通 NSIS 旧卸载器，避免用户确认安装前取消时旧程序已被删除，并保留现有快捷方式和开机自启。
- 安装 Section 在原目录覆盖文件并重写当前安装登记；旧主程序名仍由既有清理逻辑处理。

### 兼容与数据边界

- 被动安装和 `/UPDATE` 继续沿用上游模板行为，不新增下载或签名逻辑。
- 旧 WiX 安装使用专用识别、维护 UI 和卸载兼容分支，不套用普通 NSIS 原地覆盖规则。
- 早期 current-user 版本登记在 HKCU，不自动迁移；发布说明要求先卸载旧版。
- 卸载器只移除程序和快捷方式，不得删除 `%USERPROFILE%\.codex`、`%LOCALAPPDATA%\CodexRelay`、API Key、日志或备份。
- 构建成功只证明产物生成，不等于真实安装、升级、卸载、数据保留或签名成功。

## 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| D 盘为固定磁盘且是新鲜安装 | 默认目录为 `D:\Program Files\${PRODUCTNAME}`，仍显示目录页 |
| D 盘缺失、可移动、光驱或网络盘 | 回退目标架构的系统 Program Files |
| 产品目录键和 `UninstallString` 同时存在 | 恢复登记目录、设置原地升级、跳过目录页 |
| 只有历史产品目录键，没有 `UninstallString` | 不设置原地升级；按新鲜安装允许选目录 |
| 已登记升级同时传入 `/D=` | 登记目录最终生效，不创建第二套安装 |
| 普通 NSIS 升级在确认或实际安装前取消 | 不调用旧卸载器；旧程序仍应可启动 |
| 发现旧 WiX 安装 | 进入专用迁移分支，不误用普通 NSIS 原地覆盖 |
| UAC 取消、文件写入或安装器执行失败 | 不报告升级成功；保留真实失败和未完成项 |
| 执行升级、重装或卸载 | 不删除 Codex/Relay 用户数据、密钥、日志和备份 |

## 5. 良好、基线与错误用例

- 良好：用户在 `C:\Program Files\Codex Relay` 已有有效 NSIS 登记，手动运行更高版本；安装器显示原地升级说明、跳过目录页并覆盖原目录。
- 良好：用户先从 Windows 卸载旧版，只留下历史目录键；再次运行安装器时可以选择新的安装位置。
- 基线：新鲜安装优先固定 D 盘，否则回退 Program Files；用户仍可在目录页修改位置。
- 基线：应用内 updater 使用被动更新参数并沿用登记目录。
- 错误：已登记升级仍显示“不要卸载”选项，允许两套程序并存并覆盖同一套注册表登记。
- 错误：在说明页或目录页之前调用旧 NSIS 卸载器，导致用户取消后旧程序已经不可用。
- 错误：仅凭残留产品目录键锁定 `$INSTDIR`，使正常卸载后的重新安装无法选择新目录。

## 6. 必需测试

- `src/release-config.test.ts` 必须断言 `perMachine`、自定义模板、固定 D 盘判断、两个新装默认目录和 `MUI_PAGE_DIRECTORY`。
- 同一结构测试必须断言 `.onInit` 在新装默认值之后调用 `RestorePreviousInstallLocation`，且恢复函数同时检查产品目录键和 `UninstallString`。
- 同一结构测试必须断言已登记升级设置 `$UpgradeMode`、显示原地升级说明、普通分支没有单选按钮、`SkipIfUpgrade` 跳过目录页，并保留 WiX 兼容分支。
- 同一结构测试必须断言普通 NSIS 的 `PageLeaveReinstall` 在旧卸载调用前结束，防止维护 UI 之后又提前执行卸载器。
- 运行 `npm run check` 和实际 `npm run build`，枚举主程序与 NSIS 安装器的路径、大小和 SHA-256；生成成功不得写成安装成功。
- 发布前在隔离 Windows Sandbox/VM 中验证新装、同目录手动升级、应用内升级、安装前取消、快捷方式/开机自启和用户数据保留；未执行场景必须明确标为未验证。

## 7. 错误与正确做法

错误：只看历史目录键就进入升级，普通卸载后会把新装错误锁回旧目录。

```nsi
ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
${If} $4 != ""
  StrCpy $INSTDIR $4
  StrCpy $UpgradeMode 1
${EndIf}
```

正确：产品目录键和有效卸载登记必须同时存在。

```nsi
ReadRegStr $4 SHCTX "${MANUPRODUCTKEY}" ""
ReadRegStr $5 SHCTX "${UNINSTKEY}" "UninstallString"
${If} $4 != ""
${AndIf} $5 != ""
  StrCpy $INSTDIR $4
  StrCpy $UpgradeMode 1
${EndIf}
```

错误：普通 NSIS 升级在用户进入实际安装前调用旧卸载器，或提供并存安装选项。

正确：普通 NSIS 分支显示原地升级说明后直接离开维护页；只有明确的旧 WiX 兼容分支可以执行旧卸载流程。
