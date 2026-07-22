<script setup lang="ts">
import { ElAlert, ElButton } from 'element-plus'

defineProps<{ version: string }>()

const emit = defineEmits<{
  viewUpdate: []
}>()
</script>

<template>
  <section class="update-available-banner" aria-label="软件更新提示" role="status">
    <ElAlert
      class="update-available-copy"
      type="info"
      :title="`发现新版本 ${version}，可前往设置页下载并安装。`"
      :closable="false"
      show-icon
    />
    <ElButton
      native-type="button"
      class="update-available-action"
      aria-label="前往软件更新设置"
      @click="emit('viewUpdate')"
    >
      前往更新
    </ElButton>
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
  flex: 1;
  padding: 0;
  background: transparent;
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
