# 实施计划

## 顺序与 TDD 行为切片

1. **模型目录与偏好纯逻辑**
   - 先写 Rust 单元测试：目录矩阵、默认值、合法/非法偏好、未知模型、键集合不一致、稳定 JSON。
   - 公开被测边界：模型目录查询函数、偏好解析/校验/序列化函数。
   - 不 mock 文件系统；纯函数使用内存字符串。

2. **安全路径与第四受管文件**
   - 先写路径和指纹失败测试，再扩展 `AppPaths`、`FileSetFingerprint`、测试覆盖和默认路径哨兵。
   - 公开被测边界：`AppPaths::for_test`、文件集指纹 DTO。
   - 只使用 `tempfile`；不得读取真实目录。

3. **偏好服务与损坏保护**
   - 先写缺失文件、合法文件、损坏文件、版本错误、目标增删改测试。
   - 公开被测边界：`ProviderPreferenceService`。
   - mock/替换边界只限现有文件操作抽象；不 mock 业务校验。

4. **事务、备份与恢复扩展**
   - 先写第四文件备份、写后验证、故障回滚、恢复存在状态、打开白名单测试。
   - 公开被测边界：`TransactionService`、`BackupService`、恢复 command。
   - 使用故障注入 `FileOps` 触发确定性失败。

5. **Provider 创建/编辑/删除/切换**
   - 先按 PRD 行为切片 1、2、5、6 写服务测试。
   - 删除 config service 对 Provider 嵌套 `model` 的读取/写入；顶层模型来自偏好服务。
   - 扩展 DTO、命令和托盘共享切换路径。

6. **详情即时偏好更新**
   - 先写当前/非当前 Provider 的公开服务与 command 测试。
   - 当前 Provider 原子写 preferences + config；非当前只写 preferences。
   - 验证消息、指纹、并发与失败无部分写入。

7. **安装并配置 Element Plus**
   - 安装 `element-plus` 与 `unplugin-element-plus`，按安装版本官方文档配置 Vite 按需样式。
   - 在编码前再次核对 `Select`、`Option`、`Segmented` 的实际 Props、Events 和类型。
   - 运行最小 typecheck/build 验证依赖兼容。

8. **Vue 编辑页与详情页**
   - 先写组件失败测试：Select 多选、首选模型、删除回退提示、未配置禁用、两行 Segmented、逐模型强度恢复、当前/非当前不同动作与消息。
   - 扩展类型、typed service、composable，再实现组件。
   - mock `src/services/tauri.ts` 或 composable；不访问文件系统。

9. **自检、监控、文档和关于页**
   - 扩展自检错误、文件监控和事件刷新测试。
   - 更新 README、AboutView 及其测试、数据保留说明和备份文件说明。

10. **问题复盘与规范更新**
    - 运行 `trellis-break-loop`，形成根因、失效防线和预防措施。
    - 运行 `trellis-update-spec`，更新项目数据所有权、事务文件集、Element Plus 文档核验规则和外部配置契约检查项。

## 风险与回滚点

- 第四受管文件会触及事务、备份、恢复、指纹、监控和 DTO；任何遗漏都视为阻塞完成。
- 不允许用 settings service 或 command 直接写偏好文件作为临时捷径。
- Element Plus 样式可能影响现有全局样式；只按需导入并运行明暗主题、窄窗口和现有组件回归测试。
- 每个行为切片保持独立绿色；失败三次以上返回架构讨论，不叠加补丁。

## 验证命令

按切片运行专项 Vitest/Rust 测试；完成前至少运行：

```powershell
npm run typecheck
npm run test
npm run check:frontend
npm run check:rust
npm run check
git diff --check
git status --short --ignored
git ls-files
```

如执行 Tauri/Windows 人工验证，必须使用安全开发覆盖和 `npm run dev:safe`；未执行则不得声称桌面交互已人工验证。依赖或前端构建发生变化时补跑 `npm run build:frontend`。

