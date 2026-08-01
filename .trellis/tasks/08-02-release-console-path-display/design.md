# 发布控制台仓库路径显示规范化设计

## 1. 方案选择

已比较三种方案：

1. **仓库偏好边界规范化（采用）**：加载旧 localStorage 与保存检查结果时统一转换。可以同时修复现有用户数据和未来检查结果，且不改变后端安全路径。
2. **只修改后端 DTO**：未来检查结果会正常，但旧 localStorage 在首次 IPC 前仍显示 `\\?\`，不能完整修复当前问题。
3. **只在输入组件渲染时格式化**：存储和复制出的值仍含内部前缀，组件会拥有路径语义，边界错误。

用户已同意“内部继续使用规范化路径，界面显示常规路径”的方案；因此路径显示/偏好的唯一责任放在 `useRepositoryPreference`。

## 2. 规范化契约

在 `useRepositoryPreference.ts` 增加纯函数 `normalizeRepositoryPathForDisplay(value)`，先执行 `trim()`，再按顺序处理：

1. `\\?\UNC\<server>\<share>`（大小写不敏感）→ `\\<server>\<share>`；
2. `\\?\<drive-letter>:\...` → `<drive-letter>:\...`；
3. 其他值原样返回。

只识别明确的盘符和 UNC 语法，不对所有 `\\?\` 做盲目截断。这样 `\\?\Volume{GUID}\...` 不会被转换成无根路径。

## 3. 数据流

### 跨启动恢复

```text
localStorage v1 → JSON/schema 校验 → normalizeRepositoryPathForDisplay → readonly repositoryPath → ElInput
```

### 检查成功

```text
Rust canonical inspection.repositoryPath
→ App.inspectRepository
→ repositoryPreference.remember
→ normalizeRepositoryPathForDisplay
→ 内存状态 + localStorage
→ ElInput
```

`update(value)` 保持现有语义：只更新当前输入，不持久化；仓库检查失败时不会覆盖上次成功偏好。

## 4. 测试设计

- composable RED：既有扩展盘符偏好恢复为普通盘符；`remember` 保存普通盘符；扩展 UNC 转标准 UNC；未知设备路径不截断。
- App RED：typed inspection 返回 `\\?\D:\canonical\repository` 后，输入框和 localStorage 都显示/保存 `D:\canonical\repository`。
- GREEN：只修改仓库偏好 composable；App 和组件不增加路径判断。
- 完成前运行 release-console 专项、typecheck、完整项目检查和实际打包。

## 5. 兼容与回滚

- localStorage schema/version/key 不变，无显式数据迁移；每次加载都会兼容旧值。
- Rust DTO、Tauri command、session schema 与 Git 行为不变。
- 回滚只需移除前端规范化函数；不会留下后端或文件格式迁移。
