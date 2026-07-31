<script setup lang="ts">
import { computed } from 'vue'
import { ElTag } from 'element-plus'
import type { ProviderProfile } from '../types/provider'

const props = defineProps<{
  provider: ProviderProfile
}>()

const activeLabel = computed(() =>
  props.provider.connection.role === 'identity' ? '当前身份' : '当前',
)
const connectionState = computed<{ label: string; type: 'success' | 'warning' | 'danger' } | null>(
  () => {
    const connection = props.provider.connection
    if (connection.status === 'stale') return { label: '连接已失效', type: 'danger' }
    if (connection.role !== 'source' || connection.status !== 'active') return null
    if (connection.action === 'update') return { label: '选择已变化', type: 'warning' }
    return { label: '当前连接', type: 'success' }
  },
)
</script>

<template>
  <div class="provider-status" aria-label="Provider 状态">
    <ElTag v-if="provider.isActive" type="success" effect="plain" round>
      {{ activeLabel }}
    </ElTag>
    <ElTag v-if="connectionState" :type="connectionState.type" effect="plain" round>
      {{ connectionState.label }}
    </ElTag>
    <ElTag
      :type="provider.configurationComplete ? 'success' : 'warning'"
      effect="plain"
      round
    >
      {{ provider.configurationComplete ? '配置完整' : '配置不完整' }}
    </ElTag>
    <ElTag v-if="!provider.isValid" type="danger" effect="plain" round>配置无效</ElTag>
  </div>
</template>

<style scoped>
.provider-status {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.provider-status :deep(.el-tag) {
  height: auto;
  min-height: 1.6rem;
}
</style>
