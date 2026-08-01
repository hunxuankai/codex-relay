# 修复发布控制台本地门禁失败诊断

## 目标

让“一键开始候选发布”在发布结构测试实际通过时不再因 PowerShell 中文输出编码差异误报失败；若本地门禁真实失败，控制台应显示具体失败命令和安全的退出状态，并明确候选文件已经回滚、尚未提交或推送。

## 背景与事实

- 失败会话 `release-20260801154147-0000000000000002` 停在本地检查阶段，候选未提交、远端未触发，候选文件已回滚。
- 0.5.0 候选的四个发布结构测试直接执行时 24/24 通过，经发布控制台的过滤环境执行时退出码为 1。
- `scripts/validate-release-request.ps1:117` 在识别到疑似秘密时只输出中文错误文本；`src/release-request.test.ts:100` 又依赖该中文文本判断拒绝行为。过滤环境下 PowerShell 输出编码变化会使中文变为乱码，测试因此误判。
- `tools/release-console/src-tauri/src/services/local_verification.rs:127` 的失败类型只保留命令 ID，未保留非零退出码；`tools/release-console/src-tauri/src/services/release_orchestrator.rs:379` 又把具体失败折叠为通用编排错误。
- `tools/release-console/src-tauri/src/services/release_application.rs:1053` 始终发送 `releasePipeline` 和通用文案，因此界面无法指出是 `release-structure-tests` 等哪一步失败。

## 需求

### R1：编码无关的发布请求错误契约

- 发布说明命中疑似秘密规则时，验证脚本必须继续以非零状态拒绝，并在错误文本前输出稳定 ASCII 错误码 `RELEASE_NOTES_SECRET_DETECTED`。
- 结构测试必须断言稳定错误码，不再把中文可读文案作为机器契约。
- 拒绝发生后不得写入 GitHub workflow output。

### R2：保留本地门禁失败证据

- 本地命令返回非零状态时，失败对象必须保留固定命令 ID 和退出码。
- 命令未能启动或进程后端失败时，失败对象必须保留命令 ID，并用“无退出码”与非零退出区分。
- 编排层在完成候选回滚后必须把这些字段原样传递给应用层；不得回显命令 stdout、stderr、环境变量、代理 URL、认证信息或真实密钥。

### R3：控制台显示具体安全错误

- `StepFailed.stepId` 对本地门禁失败必须使用具体命令 ID，不再固定为 `releasePipeline`。
- 稳定错误码继续使用 `RELEASE_LOCAL_VERIFICATION_FAILED`。
- 安全文案在存在退出码时显示该退出码；进程后端失败且没有可用退出码时明确说明这一事实，不虚构启动、超时等底层类别；两种情况都明确候选文件已回滚，尚未提交或推送。
- 其他发布阶段仍保持现有通用失败事件行为，取消和回滚不完整等既有错误码不变。

## 验收标准

- [x] AC1：疑似秘密发布说明在普通环境和发布控制台过滤环境下均被拒绝，测试只依赖 `RELEASE_NOTES_SECRET_DETECTED`，且 workflow output 未生成。
- [x] AC2：本地命令返回退出码 1 时，`LocalVerificationError` 和 `ReleaseOrchestratorError` 都保留具体命令 ID 与 `Some(1)`；进程后端失败且没有退出码时保留命令 ID 与 `None`。
- [x] AC3：本地门禁失败事件显示具体 `stepId`、`RELEASE_LOCAL_VERIFICATION_FAILED`、退出码或“没有可用退出码”说明，以及候选已回滚且未提交/推送的安全文案。
- [x] AC4：本地门禁失败仍停止后续命令和 Push，候选文件及事务标记按既有机制完成回滚；取消与回滚不完整语义不回归。
- [x] AC5：专项测试、发布控制台 Rust 测试、项目完整检查和发布控制台重新打包通过，并记录交付 EXE 的路径、大小、时间与 SHA-256。

## 范围外

- 不重构 PowerShell、Node 或 Windows 控制台的全局编码策略。
- 不把原始子进程输出加入前端日志、通知、任务材料或持久化会话。
- 不改变本地门禁命令集合、执行顺序、超时、候选文件范围、提交、Push 或 GitHub 发布流程。
- 不新增重试、跳过门禁或允许带失败状态继续发布的入口。
