# 实施计划：Trellis 混合子代理策略

## 当前进度

- [x] 已更新 `AGENTS.md`、工作流规范和 Inline 状态提示，区分核心生命周期与受控辅助 Agent。
- [x] 已移除配置、Hook、阶段解析器和 Codex Agent 角色卡中的固定 `fork_turns` 行为假设。
- [x] 结构契约红色检查：对 `HEAD` 旧版本运行新策略断言，退出码 1（缺少混合策略文字）。
- [x] 结构契约绿色检查：对当前工作树运行相同断言，退出码 0，输出
  `hybrid policy structure OK`。
- [x] 运行 Trellis 解析、Python 测试、TOML、差异和安全检查。
- [x] 完成全范围审查并记录证据。
- [ ] 提交改动并归档任务；当前由未跟踪 `bash.exe.stackdump` 影响工作树清洁状态。

## 验证证据

- `npm run check` 最终运行退出码 0，耗时约 92.9 秒：
  - Trellis：8 项测试通过；
  - 前端：类型检查通过，39 个测试文件、197 项测试通过；
  - Rust：依赖图、fmt、Clippy 通过；177 项单元测试、3 项路径安全测试和
    1 项 Provider 工作流测试通过。
- 平台过滤断言退出码 0：Codex 能看到辅助 Agent 规则，Kilo 不会收到 Codex 专属提示。
- Codex 路由断言退出码 0：默认仍解析为 `codex-inline`，显式 `sub-agent` 仍保持兼容。
- 3 个 `.codex/agents/*.toml` 解析通过；高置信度密钥扫描无命中。
- `git diff --check` 退出码 0，仅报告 Git 的 LF→CRLF 工作区提示。

## 已保留的失败与限制

- 第一次 `npm run check` 使用 1 秒命令超时，被退出码 124 终止，未形成项目检查结果；
  随后以可等待方式重跑并取得退出码 0。
- 第一次路由断言把含相对导入的模块当作孤立文件加载，触发 `ImportError`；确认根因后
  改为从 `.trellis/scripts` 包路径导入，相同路由断言通过。
- 本任务没有实际运行子代理试点；混合策略的效率、总 token 和交接质量仍需后续低风险
  任务提供实测证据。

## 实施顺序

1. **刷新工作流权威规则**
   - 修改 `AGENTS.md` 和 `.trellis/spec/workflow/trellis-superpowers.md`，区分主生命周期
     与受控辅助 Agent，保留 Inline 核心默认。
   - 在 `.trellis/workflow.md` 的 Phase 1/2 和 Codex Inline 状态块中补充触发条件、上下文
     契约、并发写入限制和主 Agent 复核责任。

2. **移除版本耦合的解释**
   - 更新 `.trellis/config.yaml`、`.trellis/scripts/common/workflow_phase.py` 和
     `.codex/hooks/inject-workflow-state.py` 的注释/Docstring：说明 `fork_turns` 由宿主
     决定，默认策略依赖持久化任务材料，而不是固定继承值。
   - 检查 `.codex/agents/trellis-*.toml`，只在说明与新契约冲突时同步，不改变递归防护和
     现有写入边界。

3. **静态一致性检查**
   - 搜索旧的“禁止任何子代理”与固定 `fork_turns` 表述，确认只剩历史记录或明确的兼容
     说明。
   - 检查所有派发协议仍要求 `Active task` 和任务材料读取顺序。

4. **运行验证**
   - 运行 `get_context.py --mode packages`、`--mode phase` 以及 Codex 平台步骤输出。
   - 运行 Trellis Python 测试和 TOML 解析检查。
   - 运行 `git diff --check`、敏感路径/密钥审计，并确认未触碰 `bash.exe.stackdump`。

5. **完成前审查**
   - 复读 PRD、设计和修改后的规则，确认未把 Inline 默认误改为全局 Sub-agent。
   - 记录验证证据与剩余限制；当前任务不实际派发子代理试点。

## 验证命令

```powershell
python ./.trellis/scripts/get_context.py --mode packages
python ./.trellis/scripts/get_context.py --mode phase
python ./.trellis/scripts/get_context.py --mode phase --step 2.1 --platform codex
python -m pytest ./.trellis/scripts/tests -q
@'
import pathlib, tomllib
for path in pathlib.Path('.codex/agents').glob('*.toml'):
    tomllib.loads(path.read_text(encoding='utf-8'))
print('TOML OK')
'@ | python -
git diff --check
```

## 风险与回滚点

- **风险**：规则文字过宽导致主 Agent 在共享文件上并发派发写入 Agent。
  **防护**：只允许读多写少辅助工作，明确串行写入和主 Agent 最终复核。
- **风险**：修改生成文件后 `trellis update` 产生模板冲突。
  **防护**：不修改模板哈希；检查 `.trellis/.template-hashes.json`，保留本地 `.new` 处理路径。
- **风险**：默认用户路径或 Codex 认证状态被验证命令读取。
  **防护**：本任务不运行需要用户级状态的 Codex 实验；若未来试点，必须使用临时 Home。
- **回滚**：按文件回退本任务的规则/注释改动，不恢复或删除用户的 `bash.exe.stackdump`。
