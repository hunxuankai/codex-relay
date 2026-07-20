# Vue 前端规范导航

## 开发前检查

- 组件、视图、类型或 Tauri 调用：读取 [vue-guidelines.md](vue-guidelines.md)。
- composable、异步请求或事件刷新：读取 [state-management.md](state-management.md)。
- 交互、键盘、焦点、主题或响应式布局：读取 [accessibility.md](accessibility.md)。
- 涉及密钥时同时读取 `../security/path-and-secret-safety.md`。

## 质量检查

- 是否使用 Vue 3 Composition API 与 `<script setup lang="ts">`？
- 是否只有 `src/services/tauri.ts` 导入 `invoke`？
- props/emits、DTO 和状态是否显式类型化？
- composable 是否只暴露只读状态和显式动作？
- 键盘、焦点、错误消息、窄窗口与明暗主题是否可用？

## 文件

- [Vue 与类型约定](vue-guidelines.md)
- [状态管理](state-management.md)
- [可访问性](accessibility.md)
