<script setup lang="ts">
defineProps<{
  errorCount: number
}>()

const emit = defineEmits<{
  viewDetails: []
}>()
</script>

<template>
  <section class="self-check-error-banner" aria-label="系统自检错误提示" role="alert">
    <div class="self-check-error-copy">
      <span class="self-check-error-icon" aria-hidden="true">!</span>
      <strong>系统自检发现 {{ errorCount }} 个错误项，请及时处理。</strong>
    </div>
    <button
      type="button"
      class="self-check-error-action"
      aria-label="查看自检详情"
      @click="emit('viewDetails')"
    >
      查看自检详情
    </button>
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
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.self-check-error-icon {
  display: inline-grid;
  flex: 0 0 auto;
  width: 1.6rem;
  height: 1.6rem;
  place-items: center;
  border: 2px solid currentColor;
  border-radius: 50%;
  font-weight: 800;
}

.self-check-error-action {
  flex: 0 0 auto;
  border-color: var(--danger-button-background);
  color: var(--on-danger);
  background: var(--danger-button-background);
}

.self-check-error-action:hover:not(:disabled) {
  border-color: var(--danger-button-background);
  color: var(--on-danger);
  background: var(--danger-button-background);
  filter: brightness(0.92);
}

@media (max-width: 720px) {
  .self-check-error-banner {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
