# 实施与验证

- [x] 后端发现切片：先测同 ID 匹配、只返回摘要，以及缺失或孤立来源回退；再实现安全投影。
- [x] 后端写入切片：先测指定命名 Key、其他文件与变量不变、越界和过时提交；再让两种来源走同一事务并接入 Tauri 命令。
- [x] 前端切片：先测按钮点击只提交条目 ID、防重、空列表与手填；再接入 typed IPC、composable 和弹窗。
- [x] 核对 README、「关于」页及项目与安全契约；验证隔离数据、路径安全、Provider 工作流与完整 `npm run check`。

## 检查点

- 用户已确认按 ID 匹配、点击选中的命名 Key、只修改目标 Profile 用户变量、无可用项时仍可手填。已审查规划；下一步先补后端发现回归。
- 后端发现：修正 fixture 使用有效条目 UUID 后，首次断言因 `providerKeys` 缺失失败；实现后 `npm run test:rust:lib -- cli_profile` 通过 7 项。
- 后端写入：新用例因命令参数类型与服务入口尚未实现而编译失败；实现后同一 Rust 专项通过 9 项。
- 前端交互：新增断言因缺少命名按钮与空列表提示而失败；接入后 `npm test -- src/views/CliProfilesView.test.ts` 通过 5 项。进入全量审查与验证。
- 最终 Rust 专项 `npm run test:rust:lib -- cli_profile` 通过 10 项，含暂存后密钥轮换的回滚验证；`npm run test:rust:path-safety` 通过 3 项；`npm run test:rust:provider-workflow` 通过 4 项。
- 使用模拟 Tauri IPC 在浏览器观察 390×620 和 900×620。调整弹窗位置后两种尺寸弹窗均完整可见，没有横向溢出，多条 Key 可纵向滚动；键盘 Tab、Enter 选中「备用」后列表更新为「已设置」。
- 最终代码使用双临时路径运行 `npm run check`（退出 0，307 秒），包含 Trellis、Vue 类型及测试、Rust 格式／Clippy／全量测试；保留发布台原有的 1 项显式忽略。临时测试路径和界面服务已清理。未在真实用户变量上操作，未打包或发布。
- 复核跨层字段、一次性令牌、只读密钥来源与提交前回查；高置信密钥扫描 0 命中，无受保护数据文件被跟踪。通过质量门禁后精确暂存代码和规范，随后由 Trellis 完成归档与会话记录，再同步已配置上游。
