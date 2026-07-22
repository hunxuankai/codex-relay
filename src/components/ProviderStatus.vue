<script setup lang="ts">
import { ElTag } from 'element-plus'
import type { ProviderProfile } from '../types/provider'

defineProps<{
  provider: ProviderProfile
}>()
</script>

<template>
  <div class="provider-status" aria-label="Provider 状态">
    <ElTag v-if="provider.isActive" type="success" effect="plain" round>当前</ElTag>
    <ElTag
      :type="provider.apiKeyConfigured ? 'success' : 'warning'"
      effect="plain"
      round
    >
      {{ provider.apiKeyConfigured ? '密钥已配置' : '未配置密钥' }}
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
