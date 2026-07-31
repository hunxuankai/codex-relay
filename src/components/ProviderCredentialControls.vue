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
const connectionLocked = computed(() =>
  props.provider.connection.role === 'identity' && props.provider.connection.status !== 'none',
)
const lockedMessage = computed(() => {
  if (!connectionLocked.value) return null
  if (props.provider.connection.status === 'stale') {
    return props.provider.connection.disabledReason ?? '当前连接已失效，请先恢复自身连接。'
  }
  const source = props.provider.connection.sourceProviderName ?? '连接来源 Provider'
  const entry = props.provider.connection.appliedApiKeyName ?? '已应用密钥'
  return `当前身份正在使用 ${source} 的「${entry}」；恢复自身连接后可管理自身 API Key。`
})
const currentLabel = computed(() => {
  if (connectionLocked.value && props.provider.connection.status === 'active') {
    const source = props.provider.connection.sourceProviderName ?? '连接来源 Provider'
    const entry = props.provider.connection.appliedApiKeyName ?? '已应用密钥'
    return `当前连接：${source} · ${entry}`
  }
  if (selectedEntry.value) return `当前密钥：${selectedEntry.value.name}`
  if (props.provider.apiKeyStatus === 'external') return '当前使用外部密钥'
  return '尚未配置受管密钥'
})

function select(value: string | number | boolean) {
  if (!connectionLocked.value && typeof value === 'string') emit('select', value)
}

function manage() {
  if (!connectionLocked.value) emit('manage')
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
        size="small"
        native-type="button"
        aria-label="管理 API Key"
        :aria-describedby="connectionLocked ? 'api-key-connection-lock' : undefined"
        :disabled="busy || connectionLocked"
        @click="manage"
      >
        管理
      </ElButton>
    </header>

    <div class="segmented-scroll" role="group" aria-label="选择 API Key">
      <ElSegmented
        v-if="options.length > 0"
        size="small"
        :model-value="provider.selectedApiKeyId ?? undefined"
        :options="options"
        :aria-describedby="connectionLocked ? 'api-key-connection-lock' : undefined"
        :disabled="busy || connectionLocked"
        aria-label="选择 API Key"
        @change="select"
      />
      <p v-else class="empty-message">没有受管密钥，请先添加。</p>
    </div>

    <p v-if="lockedMessage" id="api-key-connection-lock" class="locked-message" role="note">
      {{ lockedMessage }}
    </p>

    <p v-if="provider.disabledReason" class="disabled-reason" role="note">
      {{ provider.disabledReason }}
    </p>
    <p class="secret-note">密钥值仅在“管理与查看”对话框中显示。</p>
  </section>
</template>

<style scoped>
.provider-control {
  display: grid;
  gap: 0.55rem;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 0.7rem;
  padding: 0.75rem;
  background: var(--surface);
}

.control-header {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 0.75rem;
}

.control-header :deep(.el-button) {
  flex: 0 0 auto;
  width: auto;
}

.control-title,
.current-label,
.empty-message,
.disabled-reason,
.locked-message,
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
  font-size: 0.78rem;
}

.segmented-scroll {
  min-width: 0;
  overflow-x: auto;
  padding-bottom: 0.1rem;
  scrollbar-width: thin;
}

.segmented-scroll :deep(.el-segmented) {
  width: max-content;
  min-height: 2rem;
}

.disabled-reason {
  color: var(--warning-text);
  font-size: 0.82rem;
}

.locked-message {
  color: var(--warning-text);
  font-size: 0.82rem;
}
</style>
