<script setup lang="ts">
import type { ProviderProfile } from '../types/provider'

defineProps<{
  provider: ProviderProfile
}>()
</script>

<template>
  <div class="provider-status" aria-label="Provider 状态">
    <span v-if="provider.isActive" class="status-badge status-current">当前</span>
    <span
      class="status-badge"
      :class="provider.apiKeyConfigured ? 'status-ready' : 'status-warning'"
    >
      {{ provider.apiKeyConfigured ? '密钥已配置' : '未配置密钥' }}
    </span>
    <span v-if="!provider.isValid" class="status-badge status-error">配置无效</span>
  </div>
</template>

<style scoped>
.provider-status {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.status-badge {
  border: 1px solid currentColor;
  border-radius: 999px;
  padding: 0.12rem 0.5rem;
  font-size: 0.75rem;
}

.status-current,
.status-ready {
  color: #16734a;
}

.status-warning {
  color: #9a5b00;
}

.status-error {
  color: #b42318;
}
</style>
