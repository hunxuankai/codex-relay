---
name: trellis-update-spec
description: "把可执行契约和编码约定沉淀到 .trellis/spec/ 文档中。调试、实施或讨论中学到值得为未来会话保留的内容时使用。"
---

# 更新代码规范——沉淀可执行契约

当你从调试、实施或讨论中学到有价值的内容时，使用此 Skill 更新相关代码规范文档。

**时机**：完成任务、修复缺陷或发现新模式之后

---

## 代码规范优先规则（关键）

在本项目中，实施工作的“规范”是指**代码规范**：
- 可执行契约，而不是只有原则的文本
- 具体签名、Payload 字段、环境变量键和边界行为
- 可测试的验证/错误行为

如果改动涉及基础设施或跨层契约，必须达到代码规范深度。

### 强制触发条件

改动包含以下任一内容时，必须达到代码规范深度：
- 新增/变更命令或 API 签名
- 跨层请求/响应契约变更
- 数据库 Schema/迁移变更
- 基础设施集成（存储、队列、缓存、密钥、环境变量接线）

### 强制输出（7 个章节）

触发此要求的任务必须包含以下全部章节：
1. 范围/触发条件
2. 签名（命令/API/数据库）
3. 契约（请求/响应/环境变量）
4. 验证与错误矩阵
5. 良好/基线/错误用例
6. 必需测试（包含断言点）
7. 错误与正确做法（至少一组）

---

## 何时更新代码规范

| 触发条件 | 示例 | 目标规范 |
|---------|---------|-------------|
| **实施功能** | 添加新集成或模块 | 相关规范文件 |
| **作出设计决定** | 选择可扩展模式而非简单方案 | 相关规范及“设计决定”章节 |
| **修复缺陷** | 发现错误处理中的细微问题 | 相关规范（如错误处理文档） |
| **发现模式** | 找到更好的代码组织方式 | 相关规范文件 |
| **遇到易错点** | 发现必须先做 X 再做 Y | 相关规范及“常见错误”章节 |
| **建立约定** | 团队同意命名模式 | 质量规范 |
| **新的思考触发器** | “做 Y 前别忘了检查 X” | `guides/*.md`（作为检查项） |

**关键洞察**：代码规范更新不只针对问题。每次功能实施都包含未来 AI/开发者安全执行所需的设计决定和契约。

---

## 规范结构概览

```
.trellis/spec/
├── <layer>/           # Per-layer coding standards (e.g., backend/, frontend/, api/)
│   ├── index.md       # Overview and links
│   └── *.md           # Topic-specific guidelines
└── guides/            # Thinking checklists (NOT coding specs!)
    ├── index.md       # Guide index
    └── *.md           # Topic-specific guides
```

### 关键：区分代码规范与指南

| 类型 | 位置 | 用途 | 内容风格 |
|------|----------|---------|---------------|
| **代码规范** | `<layer>/*.md` | 告诉 AI“如何安全实施” | 签名、契约、矩阵、用例、测试点 |
| **指南** | `guides/*.md` | 帮助 AI 知道“应考虑什么” | 检查清单、问题、规范指针 |

**判断规则**：问自己：

- “这是**如何编写**代码”→ 放入规范层目录
- “这是编写前**要考虑什么**”→ 放入 `guides/`

**示例**：

| 学到的内容 | 错误位置 | 正确位置 |
|----------|----------------|------------------|
| “此任务使用 API X，不使用 API Y” | ❌ `guides/`（对思考指南而言过于具体） | ✅ 相关规范文件（具体约定） |
| “做 Y 时记得检查 X” | ❌ 规范文件（对规范而言过于抽象） | ✅ `guides/`（思考清单） |

**指南应是指向规范的简短检查清单**，不能重复详细规则。

---

## 更新流程

### 步骤 1：识别学到的内容

回答以下问题：

1. **学到了什么？**（具体说明）
2. **为什么重要？**（它预防什么问题？）
3. **应放在哪里？**（哪个规范文件？）

### 步骤 2：分类更新类型

| 类型 | 说明 | 行动 |
|------|-------------|--------|
| **设计决定** | 为什么选择方案 X 而不是 Y | 添加到“设计决定”章节 |
| **项目约定** | 本项目如何实施 X | 添加到相关章节并提供示例 |
| **新模式** | 发现的可复用做法 | 添加到“模式”章节 |
| **禁止模式** | 会引发问题的做法 | 添加到“反模式”或“禁止”章节 |
| **常见错误** | 容易犯的错误 | 添加到“常见错误”章节 |
| **约定** | 已达成一致的标准 | 添加到相关章节 |
| **易错点** | 不直观的行为 | 添加警告提示 |

### 步骤 3：读取目标代码规范

编辑前读取当前代码规范，以便：
- 理解现有结构
- 避免重复内容
- 找到适合更新的章节

```bash
cat .trellis/spec/<category>/<file>.md
```

### 步骤 4：执行更新

遵循以下原则：

1. **具体**：包含具体示例，不只有抽象规则
2. **解释原因**：说明它预防的问题
3. **展示契约**：添加签名、Payload 字段和错误行为
4. **展示代码**：为关键模式添加代码片段
5. **保持简短**：每个章节只表达一个概念

### 步骤 5：更新索引（如需要）

如果新增章节或代码规范状态发生变化，更新该类别的 `index.md`。

---

## 更新模板

### 基础设施/跨层工作的强制模板

```markdown
## Scenario: <name>

### 1. Scope / Trigger
- Trigger: <why this requires code-spec depth>

### 2. Signatures
- Backend command/API/DB signature(s)

### 3. Contracts
- Request fields (name, type, constraints)
- Response fields (name, type, constraints)
- Environment keys (required/optional)

### 4. Validation & Error Matrix
- <condition> -> <error>

### 5. Good/Base/Bad Cases
- Good: ...
- Base: ...
- Bad: ...

### 6. Tests Required
- Unit/Integration/E2E with assertion points

### 7. Wrong vs Correct
#### Wrong
...
#### Correct
...
```

### 添加设计决定

```markdown
### Design Decision: [Decision Name]

**Context**: What problem were we solving?

**Options Considered**:
1. Option A - brief description
2. Option B - brief description

**Decision**: We chose Option X because...

**Example**:
\`\`\`typescript
// How it's implemented
code example
\`\`\`

**Extensibility**: How to extend this in the future...
```

### 添加项目约定

```markdown
### Convention: [Convention Name]

**What**: Brief description of the convention.

**Why**: Why we do it this way in this project.

**Example**:
\`\`\`typescript
// How to follow this convention
code example
\`\`\`

**Related**: Links to related conventions or specs.
```

### 添加新模式

```markdown
### Pattern Name

**Problem**: What problem does this solve?

**Solution**: Brief description of the approach.

**Example**:
\`\`\`
// Good
code example

// Bad
code example
\`\`\`

**Why**: Explanation of why this works better.
```

### 添加禁止模式

```markdown
### Don't: Pattern Name

**Problem**:
\`\`\`
// Don't do this
bad code example
\`\`\`

**Why it's bad**: Explanation of the issue.

**Instead**:
\`\`\`
// Do this instead
good code example
\`\`\`
```

### 添加常见错误

```markdown
### Common Mistake: Description

**Symptom**: What goes wrong

**Cause**: Why this happens

**Fix**: How to correct it

**Prevention**: How to avoid it in the future
```

### 添加易错点

```markdown
> **Warning**: Brief description of the non-obvious behavior.
>
> Details about when this happens and how to handle it.
```

---

## 交互模式

如果不确定要更新什么，回答以下提示：

1. **刚完成了什么？**
   - [ ] 修复缺陷
   - [ ] 实施功能
   - [ ] 重构代码
   - [ ] 讨论方案

2. **学到了什么或作出了什么决定？**
   - 设计决定（为什么选择 X 而不是 Y）
   - 项目约定（如何实施 X）
   - 不直观行为（易错点）
   - 更好的做法（模式）

3. **未来 AI/开发者是否需要知道？**
   - 为了理解代码如何工作 → 是，更新规范
   - 为了维护或扩展功能 → 是，更新规范
   - 为了避免重复犯错 → 是，更新规范
   - 纯一次性实现细节 → 可以跳过

4. **与哪个领域相关？**
   - [ ] 后端代码
   - [ ] 前端代码
   - [ ] 跨层数据流
   - [ ] 代码组织/复用
   - [ ] 质量/测试

---

## 质量检查清单

完成代码规范更新前：

- [ ] 内容是否具体且可执行？
- [ ] 是否包含代码示例？
- [ ] 是否解释了原因，而不只说明内容？
- [ ] 是否包含可执行签名/契约？
- [ ] 是否包含验证和错误矩阵？
- [ ] 是否包含良好/基线/错误用例？
- [ ] 是否包含带断言点的必需测试？
- [ ] 是否放在正确的代码规范文件中？
- [ ] 是否与现有内容重复？
- [ ] 新团队成员能否理解？

---

## 与其他命令的关系

```
Development Flow:
  Learn something → `update-spec` (Trellis command) → Knowledge captured
       ↑                                  ↓
  `break-loop` (Trellis command) ←──────────────────── Future sessions benefit
  (deep bug analysis)
```

- `break-loop`（Trellis 命令）——深度分析缺陷，通常会发现所需的规范更新
- `update-spec`（Trellis 命令）——实际执行更新
- `finish-work`（Trellis 命令）——提醒检查规范是否需要更新

---

## 核心理念

> **代码规范是活文档。每次调试、每个“恍然大悟”的时刻，都是让实施契约更清晰的机会。**

目标是形成**组织记忆**：
- 一个人学到的内容让所有人受益
- AI 在一次会话中学到的内容延续到未来会话
- 错误转化为有记录的护栏
