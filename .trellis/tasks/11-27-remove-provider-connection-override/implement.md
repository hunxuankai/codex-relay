# 实施计划：移除仅应用连接

- [x] 对旧版活动关系增加失败回归：只读投影为外部值、可用性不跨 Provider 取凭据、普通切换不复原旧目标、正常偏好写入清理旧字段。
- [x] 对 Provider 列表增加失败回归：普通动作保留，不显示旧连接动作。移除相关组件、页面逻辑、客户端和废弃用例，保留普通动作和「关于」说明的覆盖。
- [x] 移除 Rust DTO、命令、服务分支、事务写入种类和专用自检，验证旧快照仍可显式恢复。
- [x] 同步 README 和现行规范；扫描运行时代码和文档残留，运行专项检查、路径安全及 `npm run check`。

收尾顺序：审查暂存差异并提交，归档任务和记录日志，再正常推送已配置主分支并验证远端提交一致。

所有文件用例只使用 `tempfile` / `AppPaths::for_test` 或成对安全覆盖；不启动真实应用、不发出真实 Provider 请求。保持事务回滚和原始快照兼容性。

## 本轮验证

- `npm run test:rust:lib -- legacy_v4_relationship_is_inert_and_omitted_on_preference_save`：红灯如预期，旧实现返回 `Routed` 而非 `External`；移除后重跑，1 个通过。
- `npm test -- src/components/ProviderList.test.ts -t 'only the ordinary actions'`：红灯如预期，旧界面额外展示覆盖动作与风险入口；移除后运行列表、详情、控件及 typed client 专项，8 个测试文件、56 个用例通过。
- `npm run test:rust:lib -- legacy_v4`：2 个通过，已验证旧备份原字节恢复和显式重选自身条目。
- `npm test -- src/views/AboutView.test.ts`：2 个通过；`npm run typecheck` 退出 0。
- 初次 Rust 格式检查发现本次改动的排版差异，已运行 `cargo fmt --all`。
- `npm run check` 退出 0：Trellis 8 项、应用前端 324 项、发布控制台前端 107 项、两套类型检查、Rust 依赖图、格式、Clippy 和工作区默认测试均通过。全量检查使用的成对临时目录已清理。
- `cargo test --manifest-path src-tauri/Cargo.toml --workspace --test path_safety`：3 项通过，确认缺少覆盖时不会回退到真实用户目录，事务和可用性用例未改变模拟的默认目录。
- `git diff --check` 通过；旧功能标识仅保留在回归用例、现行兼容说明和历史记录，不存在运行时入口。
