<script setup lang="ts">
defineProps<{ version: string }>()

const emit = defineEmits<{
  viewUpdate: []
}>()
</script>

<template>
  <section class="update-available-banner" aria-label="软件更新提示" role="status">
    <div class="update-available-copy">
      <span class="update-available-icon" aria-hidden="true">↑</span>
      <strong>发现新版本 {{ version }}，可前往设置页下载并安装。</strong>
    </div>
    <button
      type="button"
      class="update-available-action"
      aria-label="前往软件更新设置"
      @click="emit('viewUpdate')"
    >
      前往更新
    </button>
  </section>
</template>

<style scoped>
.update-available-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  border-bottom: 1px solid var(--accent);
  padding: 0.75rem 1.25rem;
  color: var(--accent-strong);
  background: var(--accent-soft);
}

.update-available-copy {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.update-available-icon {
  display: inline-grid;
  flex: 0 0 auto;
  width: 1.6rem;
  height: 1.6rem;
  place-items: center;
  border: 2px solid currentColor;
  border-radius: 50%;
  font-weight: 800;
}

.update-available-action {
  flex: 0 0 auto;
}

@media (max-width: 720px) {
  .update-available-banner {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
