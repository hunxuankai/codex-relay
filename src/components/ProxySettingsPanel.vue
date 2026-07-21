<script setup lang="ts">
import { computed } from 'vue'
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

function updateEnabled(event: Event) {
  emit('update:modelValue', {
    ...props.modelValue,
    enabled: (event.target as HTMLInputElement).checked,
  })
}

function updateUrl(event: Event) {
  emit('update:modelValue', {
    ...props.modelValue,
    url: (event.target as HTMLInputElement).value,
  })
}
</script>

<template>
  <section class="proxy-panel">
    <h2 class="proxy-title">网络代理</h2>
    <label class="setting-row">
      <span>
        <strong>启用应用内代理</strong>
        <small>用于检查更新和下载安装包，不影响 Codex CLI。</small>
      </span>
      <input
        type="checkbox"
        aria-label="启用应用内代理"
        :checked="modelValue.enabled"
        :disabled="busy"
        @change="updateEnabled"
      />
    </label>
    <label class="proxy-address">
      <span>代理地址</span>
      <input
        name="proxy-url"
        type="url"
        autocomplete="off"
        placeholder="http://127.0.0.1:7890"
        :value="modelValue.url"
        :disabled="busy"
        @input="updateUrl"
      />
      <small>仅支持无认证的 HTTP/HTTPS 代理，必须包含协议。</small>
    </label>
    <div class="proxy-actions">
      <button
        v-if="trimmedUrl"
        data-action="test-proxy"
        type="button"
        :disabled="busy || testing || discovering"
        @click="emit('test')"
      >
        {{ testing ? '正在测试…' : '测试代理' }}
      </button>
      <button
        v-else
        data-action="discover-proxy"
        type="button"
        :disabled="busy || testing || discovering"
        @click="emit('discover')"
      >
        {{ discovering ? '正在检测…' : '一键设置本机代理' }}
      </button>
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

.proxy-address input {
  width: 100%;
  box-sizing: border-box;
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
