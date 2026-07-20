# 跨层思考指南

> **用途**：实施前梳理跨层数据流。

---

## 问题

**多数缺陷发生在层边界**，而不是层内部。

常见跨层缺陷：

- API 返回格式 A，前端期望格式 B
- Database 存储 X，Service 转换为 Y，但丢失数据
- 多个层以不同方式实现同一逻辑

---

## 实施跨层功能前

### 步骤 1：绘制数据流

画出数据如何流动：

```
Source → Transform → Store → Retrieve → Transform → Display
```

对每个箭头提出以下问题：

- 数据采用什么格式？
- 可能出现什么问题？
- 谁负责验证？

### 步骤 2：识别边界

| 边界                  | 常见问题                          |
| --------------------- | --------------------------------- |
| API ↔ Service         | 类型不匹配、字段缺失              |
| Service ↔ Database    | 格式转换、null 处理               |
| Backend ↔ Frontend    | 序列化、日期格式                  |
| Component ↔ Component | Props 形态变化                    |

### 步骤 3：定义契约

对每个边界：

- 精确的输入格式是什么？
- 精确的输出格式是什么？
- 可能发生哪些错误？

---

## 常见跨层错误

### 错误 1：隐式假设格式

**反例**：未经检查就假设日期格式

**正例**：在边界显式转换格式

### 错误 2：验证逻辑分散

**反例**：在多个层验证同一事项

**正例**：只在入口验证一次

### 错误 3：抽象泄漏

**反例**：Component 了解 Database Schema

**正例**：每个层只了解相邻层

### 错误 4：每个消费者都解析同一载荷

**反例**：命令读取 JSONL 事件并内联转换字段：

```typescript
const thread = (ev as { thread?: string }).thread;
const labels = (ev as { labels?: string[] }).labels;
```

这看似是局部逻辑，却意味着每个消费者都拥有一份事件契约的私有版本。下次字段
变化可能只更新一个命令，而遗漏另一个。

**正例**：在事件边界只解码一次，然后导出带类型的投影：

```typescript
if (!isThreadEvent(ev)) return false;
return ev.thread === filter.thread;
```

**规则**：对于只追加日志、JSON 流、RPC 载荷或配置文件，为以下内容建立一个
所有者：

- 事件/载荷类型定义
- 类型守卫和从 `unknown` 开始的规范化
- UI 命令使用的元数据投影
- 从事实来源重放状态的 reducer

渲染代码可以格式化字段，但不得重新定义载荷契约。

---

## 跨层功能检查清单

实施前：

- [ ] 已绘制完整数据流
- [ ] 已识别所有层边界
- [ ] 已定义每个边界的格式
- [ ] 已确定验证位置

实施后：

- [ ] 已测试边界情况（null、空值、无效值）
- [ ] 已验证每个边界的错误处理
- [ ] 已检查数据经过往返后仍完整
- [ ] 已检查消费者导入共享解码器/投影，而不是在本地转换载荷字段
- [ ] 已检查派生状态指回源事件标识符（`seq`、`id`、`version`），而不是
      发明第二个 cursor

---

## 跨平台模板一致性

在 Trellis 中，命令模板（例如 `record-session.md`）以相同或近似内容存在于
**多个平台**。这属于跨层边界。

### 修改任意命令模板后的检查清单

- [ ] 找出拥有相同命令的所有平台：`find src/templates/*/commands/trellis/ -name "<command>.*"`
- [ ] 更新所有平台副本（Markdown `.md` 和 TOML `.toml`）
- [ ] 对 Gemini TOML 调整续行符（`\\` 与 `\`）和三引号字符串
- [ ] 运行 `/trellis:check-cross-layer`，确认没有遗漏

**真实示例**：更新 Claude 中的 `record-session.md` 以使用 `--mode record`，
却遗漏了 iFlow、Kilo、OpenCode 和 Gemini；跨层检查发现了该问题。

---

## 生成式运行时模板的升级一致性

部分生成文件既是文档，也是运行时输入。在 Trellis 中，`.trellis/workflow.md`
由 `get_context.py`、`workflow_phase.py`、SessionStart 过滤器和逐轮 hook 解析。
模板变更必须同时针对全新 init 和升级路径验证。

### 修改运行时解析模板后的检查清单

- [ ] 识别每个读取模板的运行时解析器，而不仅是安装模板的文件写入器
- [ ] 检查相关语法是否位于 tag 块等明显受管区域之外
- [ ] 验证全新 `init` 输出，以及写入旧 `.trellis/.version` 的版本化 `update` 场景
- [ ] 使用旧版原始模板 fixture 添加升级回归，并断言安装后的文件达到当前
      打包形态
- [ ] 更新拥有该运行时契约的后端规范

---

## 版本化文档边界

版本化文档是跨层边界：源路径、`docs.json` 版本路由和渲染后的版本选择器必须
描述同一发布线。

### 编辑版本化文档前的检查清单

- [ ] 识别目标发布线：stable、beta 或 RC
- [ ] 验证所编辑的 MDX 路径与该发布线匹配：
  - stable：`docs-site/{start,advanced,...}` 和 `docs-site/zh/{start,advanced,...}`
  - beta：`docs-site/beta/**` 和 `docs-site/zh/beta/**`
  - RC：`docs-site/rc/**` 和 `docs-site/zh/rc/**`
- [ ] 验证 `docs.json` 导航将版本标签指向相同路径
- [ ] 提交前在相对的目录树中搜索发布线专属术语
- [ ] 将出现在根发布路径下的 beta 内容视为源路径缺陷，而不是渲染缺陷

**真实示例**：一项仅适用于 beta 的任务工作流变更，在根 `start/` 和
`advanced/` 路径下记录了 `prd.md` + `design.md` + `implement.md`、任务创建
同意和 Codex 模式横幅。文档站点因此在 Release 选择器下提供了 0.6 beta 行为。
修复方式是恢复根路径的 release 文档，将 0.6 内容移至 `beta/` 和 `zh/beta/`，
并添加针对根 release 目录树 beta 标记的 grep 审计。

**真实示例**：Codex inline 模式将工作流平台标记从 `[Codex]` /
`[Kilo, Antigravity, Windsurf]` 改为 `[codex-sub-agent]` /
`[codex-inline, Kilo, Antigravity, Windsurf]`。全新 init 正确，但
`trellis update` 只合并 `[workflow-state:*]` 块，并保留块外的过时标记。
结果是升级项目获得了新 hook 脚本，却仍使用旧工作流路由，导致
`get_context.py --mode phase --platform codex` 可能返回空的 Phase 2.1 细则。

---

## 模式检测探测检查清单

当 CLI 通过探测远程资源自动检测模式时，例如检查 `index.json` 是否存在，以决定
使用 Marketplace 还是直接下载：

### 实施前

- [ ] 探测在使用结果的**所有**代码路径中运行（交互式、`-y`、`--flag` 组合）
- [ ] 区分 404 与瞬时错误，不要把两者都视为“未找到”
- [ ] 瞬时错误应**中止或重试**，绝不能静默切换模式
- [ ] 上下文变化时，例如用户切换来源，**重置**共享状态（缓存、预取数据）
- [ ] **快捷路径**，例如使用 `--template` 跳过选择器，必须具备与探测路径
      相同质量的错误处理；检查下游函数是否调用了捕获所有错误的包装器

### 实施后

- [ ] 追踪从探测结果到模式决策分支的每条路径，不能意外落入其他分支
- [ ] 测试外部格式契约（giget URI、原始 URL），或至少用注释记录
- [ ] 元数据读取应消费完整响应或使用流式解析器，绝不能把固定长度前缀当作
      完整 JSON 解析
- [ ] 从解析结果重建复合标识符时，验证包含**所有**字段且位置**正确**，例如
      使用 `provider:repo/path#ref`，而不是 `provider:repo#ref/path`
- [ ] 验证快捷路径后调用的**操作函数**内部没有使用旧的捕获所有错误的 fetch；
      需要区分错误时，必须使用具备探测质量的变体

**真实示例**：自定义注册表流程在 3 轮审查中发现了 8 个缺陷：(1) 探测只在
交互模式运行；(2) 瞬时错误落入错误模式；(3) giget URI 中 `#ref` 位置错误；
(4) 预取模板在切换来源后泄漏；(5) `--template` 快捷路径绕过探测，但
`downloadTemplateById` 内部使用捕获所有错误的 `fetchTemplateIndex`，将超时
变成“Template not found”。

**真实示例**：Agent session 更新提示使用 `response.read(4096)` 获取 npm
`latest` 元数据，然后将其作为完整 JSON 解析。`@mindfoldhq/trellis` 包元数据
超过 4 KB，导致 JSON 被截断、解析静默失败，首次 session 注入没有显示更新
提示。修复方式是在解析前读取完整响应，并添加 `version` 后跟 8 KB 元数据尾部
的回归测试。

---

## 跨平台模板一致性

在 Trellis 中，命令模板（例如 `record-session.md`）以相同或近似内容存在于
**多个平台**。这属于跨层边界。

### 修改任意命令模板后的检查清单

- [ ] 找出拥有相同命令的所有平台：`find src/templates/*/commands/trellis/ -name "<command>.*"`
- [ ] 更新所有平台副本（Markdown `.md` 和 TOML `.toml`）
- [ ] 对 Gemini TOML 调整续行符（`\\` 与 `\`）和三引号字符串
- [ ] 运行 `/trellis:check-cross-layer`，确认没有遗漏

**真实示例**：更新 Claude 中的 `record-session.md` 以使用 `--mode record`，
却遗漏了 iFlow、Kilo、OpenCode 和 Gemini；跨层检查发现了该问题。

---

## 生成式运行时模板的升级一致性

部分生成文件既是文档，也是运行时输入。在 Trellis 中，`.trellis/workflow.md`
由 `get_context.py`、`workflow_phase.py`、SessionStart 过滤器和逐轮 hook 解析。
模板变更必须同时针对全新 init 和升级路径验证。

### 修改运行时解析模板后的检查清单

- [ ] 识别每个读取模板的运行时解析器，而不仅是安装模板的文件写入器
- [ ] 检查相关语法是否位于 tag 块等明显受管区域之外
- [ ] 验证全新 `init` 输出，以及写入旧 `.trellis/.version` 的版本化 `update` 场景
- [ ] 使用旧版原始模板 fixture 添加升级回归，并断言安装后的文件达到当前
      打包形态
- [ ] 更新拥有该运行时契约的后端规范

**真实示例**：Codex inline 模式将工作流平台标记从 `[Codex]` /
`[Kilo, Antigravity, Windsurf]` 改为 `[codex-sub-agent]` /
`[codex-inline, Kilo, Antigravity, Windsurf]`。全新 init 正确，但
`trellis update` 只合并 `[workflow-state:*]` 块，并保留块外的过时标记。
结果是升级项目获得了新 hook 脚本，却仍使用旧工作流路由，导致
`get_context.py --mode phase --platform codex` 可能返回空的 Phase 2.1 细则。

---

## 模式检测探测检查清单

当 CLI 通过探测远程资源自动检测模式时，例如检查 `index.json` 是否存在，以决定
使用 Marketplace 还是直接下载：

### 实施前
- [ ] 探测在使用结果的**所有**代码路径中运行（交互式、`-y`、`--flag` 组合）
- [ ] 区分 404 与瞬时错误，不要把两者都视为“未找到”
- [ ] 瞬时错误应**中止或重试**，绝不能静默切换模式
- [ ] 上下文变化时，例如用户切换来源，**重置**共享状态（缓存、预取数据）
- [ ] **快捷路径**，例如使用 `--template` 跳过选择器，必须具备与探测路径
      相同质量的错误处理；检查下游函数是否调用了捕获所有错误的包装器

### 实施后
- [ ] 追踪从探测结果到模式决策分支的每条路径，不能意外落入其他分支
- [ ] 测试外部格式契约（giget URI、原始 URL），或至少用注释记录
- [ ] 元数据读取应消费完整响应或使用流式解析器，绝不能把固定长度前缀当作
      完整 JSON 解析
- [ ] 从解析结果重建复合标识符时，验证包含**所有**字段且位置**正确**，例如
      使用 `provider:repo/path#ref`，而不是 `provider:repo#ref/path`
- [ ] 验证快捷路径后调用的**操作函数**内部没有使用旧的捕获所有错误的 fetch；
      需要区分错误时，必须使用具备探测质量的变体

**真实示例**：自定义注册表流程在 3 轮审查中发现了 8 个缺陷：(1) 探测只在
交互模式运行；(2) 瞬时错误落入错误模式；(3) giget URI 中 `#ref` 位置错误；
(4) 预取模板在切换来源后泄漏；(5) `--template` 快捷路径绕过探测，但
`downloadTemplateById` 内部使用捕获所有错误的 `fetchTemplateIndex`，将超时
变成“Template not found”。

**真实示例**：Agent session 更新提示使用 `response.read(4096)` 获取 npm
`latest` 元数据，然后将其作为完整 JSON 解析。`@mindfoldhq/trellis` 包元数据
超过 4 KB，导致 JSON 被截断、解析静默失败，首次 session 注入没有显示更新
提示。修复方式是在解析前读取完整响应，并添加 `version` 后跟 8 KB 元数据尾部
的回归测试。

---

## 何时创建流程文档

以下情况应创建详细流程文档：

- 功能跨越 3 层或更多层
- 涉及多个团队
- 数据格式复杂
- 该功能以前导致过缺陷

---

## 事件日志/投影边界

只追加日志属于跨层契约。单个事件经过：

```
CLI input → event writer → events.jsonl → reader → filter → reducer → display
```

### 添加新事件 Kind 或字段后的检查清单

- [ ] 将事件 kind 添加到中央事件分类中
- [ ] 在事件层添加带类型的事件变体或类型守卫
- [ ] 为来自用户输入或 JSON 的数组/对象字段添加规范化辅助函数
- [ ] 只在事件写入器中分配 `seq` / `id`
- [ ] 让过滤器和 reducer 使用带类型的事件守卫，而不是本地类型转换
- [ ] 让展示代码使用 reducer 输出或带类型的事件，而不是原始 JSON
- [ ] 添加至少一个回归测试，证明历史重放和实时过滤使用同一过滤模型

**真实示例**：Thread channel 添加了 `kind: "thread"`、`description`、
`context`、labels 和 `lastSeq`。首个实现能正确重放 thread 状态，但多个命令
仍通过本地类型转换重新解析事件载荷字段。修复方式是让核心事件层拥有
`ThreadChannelEvent` 和 `isThreadEvent`，将 `reduceChannelMetadata` 作为
唯一的 channel 元数据投影，并将 `reduceThreads` 作为唯一的 thread 重放
reducer。
