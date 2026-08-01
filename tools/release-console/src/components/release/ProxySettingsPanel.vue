<script setup lang="ts">
import { computed } from 'vue'
import {
  ElButton,
  ElCard,
  ElInput,
  ElInputNumber,
  ElOption,
  ElSelect,
  ElSwitch,
  ElTag,
} from 'element-plus'
import {
  releaseProxyValidationReason,
  type ReleaseConnectionTestResult,
  type ReleaseProxySettings,
} from '../../types/network'

const props = defineProps<{
  settings: ReleaseProxySettings
  result: ReleaseConnectionTestResult | null
  busy: boolean
}>()

const emit = defineEmits<{
  'update:settings': [settings: ReleaseProxySettings]
  test: []
}>()

const invalidReason = computed(() => releaseProxyValidationReason(props.settings) ?? '')

const connectionResults = computed(() => {
  if (!props.result) return []
  return [
    { key: 'git', label: 'Git 远端', value: props.result.git },
    { key: 'github', label: 'GitHub API', value: props.result.github },
  ]
})

function update(patch: Partial<ReleaseProxySettings>) {
  emit('update:settings', { ...props.settings, ...patch })
}
</script>

<template>
  <ElCard class="proxy-card" shadow="never">
    <template #header>
      <div class="proxy-heading">
        <div>
          <p class="section-kicker">网络</p>
          <h2>网络代理</h2>
        </div>
        <ElSwitch
          aria-label="启用发布代理"
          :model-value="settings.enabled"
          :disabled="busy"
          @change="update({ enabled: Boolean($event) })"
        />
      </div>
    </template>

    <p class="proxy-help">
      开启时 Git 与 GitHub CLI 使用同一代理；关闭时强制直连。
    </p>

    <div class="proxy-fields">
      <label class="proxy-field">
        <span>代理类型</span>
        <ElSelect
          aria-label="代理类型"
          :model-value="settings.proxyType"
          :disabled="busy"
          @update:model-value="update({ proxyType: $event === 'socks5' ? 'socks5' : 'http' })"
        >
          <ElOption label="HTTP" value="http" />
          <ElOption label="SOCKS5" value="socks5" />
        </ElSelect>
      </label>

      <label class="proxy-field proxy-address-field">
        <span>代理地址</span>
        <ElInput
          aria-label="代理地址"
          autocomplete="off"
          placeholder="例如 127.0.0.1"
          :model-value="settings.host"
          :disabled="busy"
          @update:model-value="update({ host: String($event) })"
        />
      </label>

      <label class="proxy-field proxy-port-field">
        <span>端口</span>
        <ElInputNumber
          aria-label="代理端口"
          :model-value="settings.port"
          :min="1"
          :max="65535"
          :controls="false"
          :disabled="busy"
          @update:model-value="update({ port: $event ?? null })"
        />
      </label>
    </div>

    <p v-if="invalidReason" class="proxy-warning">{{ invalidReason }}</p>

    <div v-if="connectionResults.length > 0" class="connection-results" aria-live="polite">
      <article
        v-for="item in connectionResults"
        :key="item.key"
        class="connection-result"
      >
        <div class="connection-result-heading">
          <strong>{{ item.label }}</strong>
          <ElTag :type="item.value.success ? 'success' : 'danger'" effect="plain">
            {{ item.value.success ? '连接正常' : '连接失败' }}
          </ElTag>
        </div>
        <p class="connection-message">{{ item.value.message }}</p>
        <div class="connection-meta">
          <span>{{ item.value.durationMillis }} ms</span>
          <code v-if="item.value.code">{{ item.value.code }}</code>
        </div>
      </article>
    </div>

    <div class="proxy-actions">
      <span class="proxy-mode">当前：{{ settings.enabled ? '使用代理' : '直连' }}</span>
      <ElButton
        data-testid="test-connection-button"
        type="primary"
        plain
        native-type="button"
        :loading="busy"
        :disabled="busy || invalidReason.length > 0"
        @click="emit('test')"
      >
        测试连接
      </ElButton>
    </div>
  </ElCard>
</template>

<style scoped>
.proxy-card :deep(.el-card__body) {
  display: grid;
  gap: 0.9rem;
  padding: 1.15rem;
}

.proxy-heading,
.proxy-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.proxy-heading h2,
.section-kicker,
.proxy-help,
.proxy-warning {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.proxy-help,
.proxy-mode {
  color: var(--text-muted);
  font-size: 0.82rem;
}

.proxy-fields {
  display: grid;
  grid-template-columns: minmax(8rem, 0.7fr) minmax(12rem, 1.6fr) minmax(7rem, 0.6fr);
  gap: 0.75rem;
}

.proxy-field {
  display: grid;
  gap: 0.4rem;
  font-size: 0.84rem;
  font-weight: 700;
}

.proxy-port-field :deep(.el-input-number) {
  width: 100%;
}

.proxy-warning {
  color: var(--danger-color);
  font-size: 0.8rem;
}

.connection-results {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
}

.connection-result {
  display: grid;
  gap: 0.45rem;
  min-width: 0;
  padding: 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 0.7rem;
}

.connection-result-heading,
.connection-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.connection-message {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.82rem;
}

.connection-meta {
  color: var(--text-muted);
  font-size: 0.76rem;
}

.connection-meta code {
  overflow-wrap: anywhere;
  font-family: var(--font-mono);
}

@media (max-width: 760px) {
  .proxy-fields,
  .connection-results {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
