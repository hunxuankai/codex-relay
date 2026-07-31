<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElSegmented } from 'element-plus'
import type { ProviderProfile } from '../types/provider'

const props = defineProps<{
  provider: ProviderProfile
  busy: boolean
}>()

const emit = defineEmits<{
  select: [baseUrlId: string]
  manage: []
}>()

const options = computed(() =>
  props.provider.baseUrls.map((entry) => ({ label: entry.name, value: entry.id })),
)
const selectedEntry = computed(() =>
  props.provider.baseUrls.find((entry) => entry.id === props.provider.selectedBaseUrlId) ?? null,
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
  const entry = props.provider.connection.appliedBaseUrlName ?? '已应用地址'
  return `当前身份正在使用 ${source} 的「${entry}」；恢复自身连接后可管理自身 Base URL。`
})
const currentLabel = computed(() => {
  if (connectionLocked.value && props.provider.connection.status === 'active') {
    const source = props.provider.connection.sourceProviderName ?? '连接来源 Provider'
    const entry = props.provider.connection.appliedBaseUrlName ?? '已应用地址'
    return `当前连接：${source} · ${entry}`
  }
  if (selectedEntry.value) return `当前地址：${selectedEntry.value.name}`
  if (props.provider.baseUrlStatus === 'external') return '当前使用外部地址'
  return '尚未选择受管地址'
})

function select(value: string | number | boolean) {
  if (!connectionLocked.value && typeof value === 'string') emit('select', value)
}

function manage() {
  if (!connectionLocked.value) emit('manage')
}
</script>

<template>
  <section class="provider-control" aria-label="Base URL 设置">
    <header class="control-header">
      <div>
        <h2 class="control-title">Base URL</h2>
        <p class="current-label" role="status">{{ currentLabel }}</p>
      </div>
      <ElButton
        size="small"
        native-type="button"
        aria-label="管理 Base URL"
        :aria-describedby="connectionLocked ? 'base-url-connection-lock' : undefined"
        :disabled="busy || connectionLocked"
        @click="manage"
      >
        管理
      </ElButton>
    </header>

    <div class="segmented-scroll" role="group" aria-label="选择 Base URL">
      <ElSegmented
        v-if="options.length > 0"
        size="small"
        :model-value="provider.selectedBaseUrlId ?? undefined"
        :options="options"
        :aria-describedby="connectionLocked ? 'base-url-connection-lock' : undefined"
        :disabled="busy || connectionLocked"
        aria-label="选择 Base URL"
        @change="select"
      />
      <p v-else class="empty-message">没有受管地址，请先添加。</p>
    </div>

    <p v-if="lockedMessage" id="base-url-connection-lock" class="locked-message" role="note">
      {{ lockedMessage }}
    </p>

    <p class="actual-value">
      <span>当前实际地址</span>
      <code>{{ provider.baseUrl || '未设置' }}</code>
    </p>
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
.actual-value,
.empty-message,
.locked-message {
  margin: 0;
}

.control-title {
  font-size: 1rem;
}

.current-label,
.empty-message,
.actual-value span {
  color: var(--text-secondary);
  font-size: 0.78rem;
}

.locked-message {
  color: var(--warning-text);
  font-size: 0.82rem;
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

.actual-value {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.2rem 0.5rem;
}

.actual-value code {
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--text-primary);
}
</style>
