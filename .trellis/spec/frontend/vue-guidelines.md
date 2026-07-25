# Vue 与类型约定

## 目录职责

- `src/views/`：页面级编排，不直接调用 `invoke`。
- `src/components/`：可复用展示和局部交互，通过 typed props/emits 协作。
- `src/composables/`：资源状态、事件订阅和显式业务动作。
- `src/services/tauri.ts`：唯一 Tauri IPC 边界。
- `src/types/`：与 Rust command DTO 对应的 TypeScript 类型。

## 组件形式

所有 Vue 文件使用 Composition API：

```vue
<script setup lang="ts">
const props = defineProps<{ provider: ProviderSummary }>()
const emit = defineEmits<{ select: [providerId: string] }>()
</script>
```

不得新增 Options API、隐式 `any`、未类型化 emit 或组件内散落的字符串 command 名。

使用 Element Plus 组件前，必须查阅当前安装版本的官方文档和类型声明，核对组件名、Props、Events、Slots 及 `v-model` 类型；不得凭记忆猜测 API。当前项目的模型编辑使用 `ElSelect` 多选，详情偏好使用两行 `ElSegmented`。

## Element Plus 使用边界

- 交互控件、表单、对话框、反馈、状态标签、空状态和进度优先使用 Element Plus；`main`、`nav`、`section`、`header`、`dl`、`ul/ol` 等有意义的文档结构保留原生语义。
- `src/style.css` 的项目变量是视觉唯一事实来源，必须同步映射到 `--el-*` 变量；应用级
  `ElConfigProvider` 使用 `default` 密度。桌面普通按钮、输入和选择控件的项目最小高度为
  36px，明确标记为 `small` 的次要操作至少为 32px；不得直接暴露 Element Plus 原生 24px
  小尺寸作为项目点击目标。新增或调整组件时同时检查浅色、`prefers-color-scheme: dark`、
  键盘焦点和窄窗口布局。
- 使用 Element Plus 后不得再用全局 `button:hover`、`button:disabled`、`input:not(...)` 或 `input[type=checkbox]` 重设背景、边框、文字和尺寸；这些选择器会命中组件内部原生节点，产生白字浅底、双层输入框和开关错位。应改用 `.el-button`、`.el-input__wrapper`、`.el-switch` 等明确组件边界。
- `ElCard` 的插槽内容位于 `.el-card__body`；grid、gap、padding、对齐等内容布局必须写在 `:deep(.el-card__body)`，卡片根类只负责边框、背景、圆角和选中态。
- `ElButton` 会为默认插槽增加内容 `<span>`；按钮内包含图标与文字或多个字段时，必须增加显式内容容器，或对 `.el-button > span` 设置布局，不能假设按钮的直接子节点仍是业务元素。
- 绑定到 `ElButton` 的装饰类只负责布局（如宽度、对齐、间距），不要在裸类规则中直接
  设置 `color`、`background`、`background-color` 或 `border-color`。这些声明会与 Element
  Plus 的 `:hover`、`:active` 和 `.is-disabled` 状态规则竞争，尤其会把危险纯按钮的白色
  hover 文字覆盖回危险色，导致危险背景上的文字不可读。颜色应优先由 `type`、`plain`、
  `text`、`link` 和主题 `--el-*` 变量提供；确需覆盖时必须使用明确的状态选择器，并补充
  浅色、暗色和禁用状态测试。
- 继续显式导入组件并由 `unplugin-element-plus` 按需注入样式；不得改成 `app.use(ElementPlus)` 全量导入。
- 危险确认通过共享 `ConfirmDialog` 使用 `ElDialog`；必须关闭遮罩点击关闭，默认聚焦安全动作，并验证 Escape 与关闭后焦点恢复。
- 测试优先使用可见文本、`aria-label`、公开 props/emits 或组件类型，不锁定 `.el-*` 私有 DOM 层级。

```vue
<!-- 正确：Element Plus 提供交互，原生元素保留页面语义 -->
<nav aria-label="主导航">
  <ElButton text native-type="button">Providers</ElButton>
</nav>

<ElCard class="provider-card">
  <!-- 内容 -->
</ElCard>

<style scoped>
.provider-card :deep(.el-card__body) {
  display: grid;
  gap: 0.75rem;
  padding: 1rem;
}
</style>

<!-- 错误：为了组件覆盖率把所有语义结构机械替换成通用容器 -->
```

按钮样式示例：

```vue
<!-- 正确：语义颜色由 Element Plus 管理，类名只做布局 -->
<ElButton class="delete-key-button" type="danger" plain>删除密钥</ElButton>

<style scoped>
.delete-key-button {
  width: fit-content;
}
</style>

<!-- 错误：静态颜色覆盖 hover/active 的语义颜色 -->
<style scoped>
.delete-key-button {
  color: var(--danger);
}
</style>
```

紧凑布局示例：

```vue
<!-- 正确：全局使用 default，只有局部低频操作显式 small -->
<ElConfigProvider size="default">
  <ElButton>保存</ElButton>
  <ElButton size="small">管理</ElButton>
</ElConfigProvider>

<!-- 错误：把整个应用切为 small，绕过项目 32px/36px 点击目标 -->
<ElConfigProvider size="small">
  <!-- ... -->
</ElConfigProvider>
```

## IPC 契约

只有 `src/services/tauri.ts` 可以导入 `@tauri-apps/api/core` 的 `invoke`。它负责：

- 精确命令名和 camelCase 参数；
- `CommandResult<T>` 解包；
- 把稳定错误码和安全中文消息映射给调用方；
- 不把密钥放入普通 Provider DTO。

```ts
// 错误：组件直接调用后端
await invoke('switch_provider', { providerId })

// 正确：组件/composable 只使用 typed service
await tauri.switchProvider(providerId)
```

## 表单与密钥

- Provider ID 创建后只读。
- 创建时的地址名称、HTTP(S) Base URL、密钥名称、固定 `responses` 和模型在前端即时校验，Rust 必须再次验证。
- 常规编辑只修改 Provider 名称、Wire API 和模型，不渲染已保存 URL/Key 修改入口。
- 创建输入中的 API Key 默认密码显示；专用密钥管理器打开后默认明文显示，并提供统一隐藏/显示与逐项复制。
- 完整密钥只短暂存在 `useProviderApiKeyManager`，不进入 localStorage、日志、通知、快照或普通 composable 状态；关闭和 scope dispose 必须清空。

## 测试

组件和视图使用 Vitest + Vue Test Utils，通过 mock `src/services/tauri.ts` 或 composable 边界验证用户行为，不访问真实文件系统。
