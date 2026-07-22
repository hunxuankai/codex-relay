<script setup lang="ts">
import { ElAlert, ElButton } from 'element-plus'

defineProps<{
  errorCount: number
}>()

const emit = defineEmits<{
  viewDetails: []
}>()
</script>

<template>
  <section class="self-check-error-banner" aria-label="系统自检错误提示" role="alert">
    <ElAlert
      class="self-check-error-copy"
      type="error"
      :title="`系统自检发现 ${errorCount} 个错误项，请及时处理。`"
      :closable="false"
      show-icon
    />
    <ElButton
      native-type="button"
      type="danger"
      class="self-check-error-action"
      aria-label="查看自检详情"
      @click="emit('viewDetails')"
    >
      查看自检详情
    </ElButton>
  </section>
</template>

<style scoped>
.self-check-error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  border-bottom: 1px solid var(--danger);
  padding: 0.75rem 1.25rem;
  color: var(--danger);
  background: var(--danger-soft);
}

.self-check-error-copy {
  flex: 1;
  padding: 0;
  background: transparent;
}

.self-check-error-action {
  flex: 0 0 auto;
}

@media (max-width: 720px) {
  .self-check-error-banner {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
