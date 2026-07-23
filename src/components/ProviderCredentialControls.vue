<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElSegmented } from 'element-plus'
import type { ProviderProfile } from '../types/provider'

const props = defineProps<{
  provider: ProviderProfile
  busy: boolean
}>()

const emit = defineEmits<{
  select: [apiKeyId: string]
  manage: []
}>()

const options = computed(() =>
  props.provider.apiKeys.map((entry) => ({ label: entry.name, value: entry.id })),
)
const selectedEntry = computed(() =>
  props.provider.apiKeys.find((entry) => entry.id === props.provider.selectedApiKeyId) ?? null,
)
const currentLabel = computed(() => {
  if (selectedEntry.value) return `当前密钥：${selectedEntry.value.name}`
  if (props.provider.apiKeyStatus === 'external') return '当前使用外部密钥'
  return '尚未配置受管密钥'
})

function select(value: string | number | boolean) {
  if (typeof value === 'string') emit('select', value)
}
</script>

<template>
  <section class="provider-control" aria-label="API Key 设置">
    <header class="control-header">
      <div>
        <h2 class="control-title">API Key</h2>
        <p class="current-label" role="status">{{ currentLabel }}</p>
      </div>
      <ElButton
        native-type="button"
        aria-label="管理 API Key"
        :disabled="busy"
        @click="emit('manage')"
      >
        管理与查看
      </ElButton>
    </header>

    <div class="segmented-scroll" role="group" aria-label="选择 API Key">
      <ElSegmented
        v-if="options.length > 0"
        :model-value="provider.selectedApiKeyId ?? undefined"
        :options="options"
        :disabled="busy"
        aria-label="选择 API Key"
        @change="select"
      />
      <p v-else class="empty-message">没有受管密钥，请先添加。</p>
    </div>

    <p v-if="provider.disabledReason" class="disabled-reason" role="note">
      {{ provider.disabledReason }}
    </p>
    <p class="secret-note">密钥值仅在“管理与查看”对话框中显示。</p>
  </section>
</template>

<style scoped>
.provider-control {
  display: grid;
  gap: 0.75rem;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  padding: 0.9rem;
  background: var(--surface);
}

.control-header {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 1rem;
}

.control-title,
.current-label,
.empty-message,
.disabled-reason,
.secret-note {
  margin: 0;
}

.control-title {
  font-size: 1rem;
}

.current-label,
.empty-message,
.secret-note {
  color: var(--text-secondary);
  font-size: 0.82rem;
}

.segmented-scroll {
  min-width: 0;
  overflow-x: auto;
  padding-bottom: 0.2rem;
}

.segmented-scroll :deep(.el-segmented) {
  width: max-content;
  min-width: 100%;
}

.disabled-reason {
  color: var(--warning-text);
  font-size: 0.86rem;
}
</style>
