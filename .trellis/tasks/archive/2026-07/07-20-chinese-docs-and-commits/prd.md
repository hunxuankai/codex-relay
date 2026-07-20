# 统一内部文档与提交信息中文规范

## 目标

将 `.agents`、`.codex`、`.trellis` 中面向开发者或 AI 的自然语言文档统一为简体中文，并把“相关文档与 Git 提交信息默认使用中文”固化为长期项目规则，避免后续生成内容重新回到英文。

## 已确认事实

- `.agents` 下的 Skill 与参考文档绝大多数为英文。
- `.trellis/workflow.md`、`.trellis/agents/*.md`、`.trellis/spec/guides/*.md`、工作区模板等仍包含大段英文；项目专属 spec 多数已是中文。
- `.codex/agents/*.toml` 与 `.codex/config.toml` 包含英文说明、提示词或注释。
- Trellis 自动提交仍会生成英文提交信息，例如 `chore: record journal` 和 `chore(task): archive ...`。
- 根目录 `AGENTS.md` 已要求项目文档和 Trellis 材料默认使用简体中文，但尚未明确覆盖 `.agents` Skill、`.codex` Agent/提示词，以及 Git 提交信息。
- 用户已在本轮明确要求执行翻译、固化中文规则并创建 Trellis 任务，构成进入实施阶段的明确批准。

## 需求

1. 所有受 Git 管理的 `.agents/**/*.md` 文档，其解释性自然语言改为简体中文。
2. 所有受 Git 管理的 `.trellis/**/*.md` 文档，其解释性自然语言改为简体中文，包括工作流、Agent 指令、思考指南、工作区模板、历史任务材料中仍存在的英文说明。
3. `.codex/agents/*.toml` 中的 Agent 描述、提示词等自然语言改为简体中文；`.codex/config.toml` 与相关配置中的说明性注释改为简体中文。
4. 翻译时保留命令、代码块、代码标识符、配置键、文件名、路径、占位符、协议字段、专有名词以及没有通行中文译名的技术名词；不得改变 Skill frontmatter、TOML/YAML/JSON 结构和运行语义。
5. 在 `AGENTS.md` 和 `.trellis/spec/workflow/documentation.md` 中记录长期规则：项目文档、README、设计、计划、验证记录、Trellis 材料、`.agents` Skill/参考资料、`.codex` Agent/提示词默认使用简体中文。
6. Git 提交主题与正文默认使用简体中文；允许 Conventional Commits 的 `type(scope):` 作为机器可识别前缀保留英文标识符。
7. Trellis 自动生成的归档提交和会话日志提交使用中文主题，并同步修改默认值、项目配置和相关测试或验证依据。
8. 当前任务的 PRD、设计、实施进度、验证证据和下一步持续保存在本任务目录中，且全部使用简体中文。

## 验收标准

- [x] `.agents`、`.codex`、`.trellis` 范围内受管文档的解释性段落、标题、表格说明、清单和提示词已统一为简体中文；抽检和扫描未发现应翻译的整段英文。
- [x] 所有代码块、命令、路径、配置键、链接目标、占位符和 frontmatter/TOML/YAML/JSON 结构保持有效。
- [x] `AGENTS.md` 与工作流文档明确规定相关文档和 Git 提交信息默认使用简体中文。
- [x] Trellis 的会话日志与任务归档自动提交信息改为中文主题，配置默认值与运行实现一致。
- [x] 与本次改动相关的语法检查、Trellis 检查和针对性测试通过，并在 `implement.md` 记录真实命令与结果。
- [x] Git diff 中没有真实密钥、认证信息、运行时缓存或非任务文件。

## 不在范围内

- 不翻译 Python、Rust、TypeScript 等可执行源码中的普通变量名、函数名、协议常量和运行时消息，除非它们直接负责生成 Git 提交主题。
- 不修改 `.trellis/.runtime/`、`__pycache__/`、`.trellis/.template-hashes.json` 等运行时或生成文件。
- 不重命名现有文件、目录、Skill 名称、任务 slug 或配置键。
