# 代码复用思考指南

> **用途**：创建新代码前停下来思考：现有代码中是否已经存在？

---

## 问题

**重复代码是不一致缺陷的首要来源。**

复制粘贴或重写现有逻辑时：
- 缺陷修复不会同步传播
- 行为会随时间产生偏差
- 代码库会更难理解

---

## 编写新代码前

### 步骤 1：先搜索

```bash
# Search for similar function names
grep -r "functionName" .

# Search for similar logic
grep -r "keyword" .
```

### 步骤 2：提出这些问题

| 问题 | 如果答案是“是” |
|----------|-----------|
| 是否存在类似函数？ | 使用或扩展它 |
| 这一模式是否在其他位置使用？ | 遵循现有模式 |
| 能否成为共享工具？ | 在正确位置创建 |
| 是否正在从另一个文件复制代码？ | **停止**，提取为共享实现 |

---

## 常见重复模式

### 模式 1：复制粘贴函数

**反例**：将验证函数复制到另一个文件

**正例**：提取到共享工具，在需要的位置导入

### 模式 2：相似组件

**反例**：创建与现有组件 80% 相似的新组件

**正例**：通过 props/variant 扩展现有组件

### 模式 3：重复常量

**反例**：在多个文件中定义相同常量

**正例**：建立唯一事实来源，并在各处导入

### 模式 4：重复提取载荷字段

**反例**：多个消费者在本地转换相同的 JSON/事件字段：

```typescript
const description = (ev as { description?: string }).description;
const context = (ev as { context?: ContextEntry[] }).context;
```

即使代码只有两行，这也是重复的契约逻辑。每个消费者都拥有一份对有效载荷的
私有定义。

**正例**：将解码器、类型守卫或投影放在数据所有者旁边：

```typescript
if (isThreadEvent(ev)) {
  renderThreadEvent(ev);
}
```

**规则**：如果同一个无类型载荷字段已在两个或更多位置读取，在添加第三个
读取者前创建共享类型守卫、规范化函数或投影。

---

## 何时抽象

**以下情况应抽象**：
- 相同代码出现 3 次或更多
- 逻辑复杂到容易产生缺陷
- 可能有多人需要使用

**以下情况不要抽象**：
- 只使用一次
- 简单的单行代码
- 抽象会比重复更复杂

---

## 批量修改后

对多个文件完成相似修改后：

1. **审查**：是否覆盖所有实例？
2. **搜索**：运行 grep 查找遗漏
3. **考虑**：是否应该抽象？

### Reducer 应使用穷尽结构

当状态派生自类似 action 的值（`action`、`kind`、`status`、`phase`）时，
优先使用带一个 `switch` 的 reducer，而不是分散的 `if/else` 更新。

```typescript
// BAD - action-specific state transitions are hard to audit
if (action === "opened") { ... }
else if (action === "comment") { ... }
else if (action === "status") { ... }

// GOOD - one reducer owns the transition table
switch (event.action) {
  case "opened":
    ...
    return;
  case "comment":
    ...
    return;
}
```

当事件日志是事实来源时，这一点尤其重要。Reducer 是有文档记录的重放模型；
展示代码和命令不应重复实现该重放模型的一部分。

---

## 提交前检查清单

- [ ] 已搜索现有相似代码
- [ ] 没有本应共享的复制粘贴逻辑
- [ ] 共享解码器之外没有重复提取无类型载荷字段
- [ ] 常量只在一个位置定义
- [ ] 相似模式采用相同结构
- [ ] Reducer/action 转换位于同一个 reducer 或命令派发器中

---

## 易错点：Python if/elif/else 穷尽检查

**问题**：Python 的 if/elif/else 链没有编译时穷尽检查。向 `Literal` 类型
（例如 `Platform`）添加新值时，现有 if/elif/else 链会静默落入 `else`，
并采用错误的默认值。

**症状**：新平台只能部分工作；某些方法返回 Claude 默认值，而不是平台专属值，
且不会引发错误。

**示例**（`cli_adapter.py`）：
```python
# BAD: "gemini" falls through to else, returns "claude"
@property
def cli_name(self) -> str:
    if self.platform == "opencode":
        return "opencode"
    else:
        return "claude"  # gemini silently gets "claude"!

# GOOD: explicit branch for every platform
@property
def cli_name(self) -> str:
    if self.platform == "opencode":
        return "opencode"
    elif self.platform == "gemini":
        return "gemini"
    else:
        return "claude"
```

**预防**：向 Python `Literal` 类型添加新值时，搜索所有基于该类型切换的
if/elif/else 链并添加显式分支。不要依赖 `else` 对新值碰巧正确。

---

## 易错点：不对称机制生成相同输出

**问题**：当两种不同机制必须生成相同文件集时，例如 init 递归复制目录，而
update 手动调用 `files.set()`，结构变化（重命名、移动、添加子目录）只会通过
自动机制传播，手动机制会静默偏移。

**症状**：Init 完全正常，但 update 在错误路径创建文件，或彻底遗漏文件。

**预防**：
- **最佳方案**：消除不对称，让手动路径调用自动机制，例如让
  `collectTemplateFiles()` 调用 `getAllScripts()`，而不是维护自己的列表
- **无法避免不对称时**：添加比较两种机制输出的回归测试
- 迁移目录结构时，搜索引用旧结构的所有代码路径

**真实示例**：`trellis update` 曾为 `getAllScripts()` 已跟踪的 11 个脚本维护
手动 `files.set()` 列表。修复方式是用 `for..of getAllScripts()` 循环替代手动
列表。参见 v0.4.0-beta.3 中对 `update.ts` 的重构。

---

## 模板文件注册（Trellis 专属）

向 `src/templates/trellis/scripts/` 添加新文件时：

**唯一注册点**：`src/templates/trellis/index.ts`

1. 添加 `export const xxxScript = readTemplate("scripts/path/file.py");`
2. 添加到 `getAllScripts()` Map

仅需这些步骤。`commands/update.ts` 直接使用 `getAllScripts()`，无需手动同步。

**为何重要**：未在 `getAllScripts()` 中注册时，`trellis update` 不会将文件
同步到用户项目，缺陷修复和功能也不会传播。

**历史**：v0.4.0-beta.3 之前，`update.ts` 有一份手动维护的文件列表，经常与
`getAllScripts()` 失去同步，导致 `trellis update` 静默跳过 11 个 Python 文件。
修复方式是删除重复列表，并将 `getAllScripts()` 作为唯一事实来源。

### 新脚本速查清单

```bash
# After adding a new .py file, verify it's in getAllScripts():
grep -l "newFileName" src/templates/trellis/index.ts  # Should match
```

### 模板同步约定

`.trellis/scripts/`（项目自用）和 `packages/cli/src/templates/trellis/scripts/`
（模板）必须保持一致。编辑 `.trellis/scripts/` 后，始终执行同步：

```bash
rsync -av --delete --exclude='__pycache__' .trellis/scripts/ packages/cli/src/templates/trellis/scripts/
```

**易错点**：使用错误的源/目标路径运行 rsync，可能创建嵌套的无效目录，例如
`.trellis/scripts/packages/cli/...`。运行前始终再次检查路径。
