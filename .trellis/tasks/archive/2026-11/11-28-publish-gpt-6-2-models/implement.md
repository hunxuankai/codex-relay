# 实施与验证

- [x] 核对唯一模型目录、OpenAI Codex 元数据、旧 Latest、默认分支和发布工作流范围。
- [x] 为两款模型添加公开 Provider 工作流回归；先确认两项因缺少各自目录项失败（退出 101），实现后四项 GPT-6 Provider 回归均通过（退出 0）。
- [x] 同步 README、模型契约、`0.5.5` 版本与最终发布说明；路径、发布结构、请求校验、完整项目检查和普通 Windows 构建均通过，实际产物见 `release-evidence.md`。
- [x] 候选 `fa26ef7` 已精确提交并普通推送；唯一 Draft Run `46139422066` 成功，记录了 Job/Step 与耗时。
- [x] 审计 Draft `461511927` 的提交、说明、资产哈希、清单及实际签名，并确认新的安装器 FileVersion。
- [x] 公开同一 Draft 后，复核匿名下载、Latest、tag 和清理 Run。
- [x] 记录未执行的真实安装、升级和供应商调用；归档、会话日志及 `origin/main` 最终核验按 Trellis 收尾步骤执行。
