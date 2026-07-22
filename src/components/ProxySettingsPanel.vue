<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElInput, ElSwitch } from 'element-plus'
import type { NetworkProxySettings } from '../types/settings'

const props = defineProps<{
  modelValue: NetworkProxySettings
  busy: boolean
  testing: boolean
  discovering: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: NetworkProxySettings]
  test: []
  discover: []
}>()

const trimmedUrl = computed(() => props.modelValue.url.trim())

</script>

<template>
  <section class="proxy-panel">
    <h2 class="proxy-title">网络代理</h2>
    <label class="setting-row">
      <span>
        <strong>启用应用内代理</strong>
        <small>用于检查更新和下载安装包，不影响 Codex CLI。</small>
      </span>
      <ElSwitch
        aria-label="启用应用内代理"
        :model-value="modelValue.enabled"
        :disabled="busy"
        @change="emit('update:modelValue', { ...modelValue, enabled: Boolean($event) })"
      />
    </label>
    <label class="proxy-address">
      <span>代理地址</span>
      <ElInput
        name="proxy-url"
        type="url"
        autocomplete="off"
        placeholder="http://127.0.0.1:7890"
        :model-value="modelValue.url"
        :disabled="busy"
        @update:model-value="emit('update:modelValue', { ...modelValue, url: String($event) })"
      />
      <small>仅支持无认证的 HTTP/HTTPS 代理，必须包含协议。</small>
    </label>
    <div class="proxy-actions">
      <ElButton
        v-if="trimmedUrl"
        data-action="test-proxy"
        type="primary"
        native-type="button"
        :disabled="busy || testing || discovering"
        @click="emit('test')"
      >
        {{ testing ? '正在测试…' : '测试代理' }}
      </ElButton>
      <ElButton
        v-else
        data-action="discover-proxy"
        type="primary"
        native-type="button"
        :disabled="busy || testing || discovering"
        @click="emit('discover')"
      >
        {{ discovering ? '正在检测…' : '一键设置本机代理' }}
      </ElButton>
    </div>
  </section>
</template>

<style scoped>
.proxy-panel,
.proxy-address {
  display: grid;
  gap: 0.75rem;
}

.proxy-panel {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  padding: 1rem;
}

.proxy-title {
  margin: 0;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.setting-row span {
  display: grid;
  gap: 0.25rem;
}

.proxy-address :deep(.el-input) {
  width: 100%;
}

.setting-row small,
.proxy-address small {
  color: var(--text-secondary);
}

.proxy-actions {
  display: flex;
  gap: 0.75rem;
}
</style>
