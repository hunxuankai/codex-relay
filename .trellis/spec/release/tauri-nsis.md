# Tauri 与 NSIS 发布契约

## 产物与入口

`npm run build` 与 `npm run build:release` 执行 Tauri Release 并按当前 `targets: ["nsis"]` 生成 NSIS；`npm run bundle:nsis` 是显式替代入口，不需要在成功的 Release bundle 后重复运行。Debug 使用 `npm run build:debug` 且不生成安装包。

## 安装范围

安装器为 `perMachine`，请求管理员权限，注册元数据位于 HKLM。旧 current-user 版本使用 HKCU，不能自动迁移；发布说明要求用户先卸载旧版。卸载仍保留 Codex 配置和应用数据。

## 首次安装目录算法

自定义模板基于 Tauri 2.11.4 官方 `installer.nsi`。在 `.onInit` 且 `$INSTDIR` 仍为 Tauri 占位值时：

1. 调用 `GetDriveTypeW("D:\\")`。
2. 结果为 `DRIVE_FIXED`（3）时使用 `D:\Program Files\${PRODUCTNAME}`。
3. 否则按目标架构使用 `$PROGRAMFILES64` 或 `$PROGRAMFILES`。
4. 选择首次安装默认值后调用 `RestorePreviousInstallLocation`，使已有 per-machine 注册目录优先。

标准 `MUI_PAGE_DIRECTORY` 必须保留，允许用户交互修改。命令行 `/D=`、被动安装和更新行为继续沿用上游模板。

## 兼容性

- 缺失、可移动、光驱或网络 D 盘必须回退系统 Program Files。
- 已安装版本不自动跨盘迁移；升级沿用登记目录。
- 不改变上游 WebView2、快捷方式、注册表、更新和卸载主体逻辑。
- 卸载不得删除 `.codex`、应用数据、API Key、日志或备份。

## 回归测试

`src/release-config.test.ts` 必须锁定：per-machine、模板路径、固定盘检查、两个目标目录、`RestorePreviousInstallLocation` 的顺序和 `MUI_PAGE_DIRECTORY` 存在。

## 构建验收

运行 `npm run check` 和实际 `npm run build`；枚举生成安装器的路径、大小、时间与 SHA-256。生成成功不代表真实安装、升级或卸载已验证。
