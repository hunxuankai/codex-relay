<script setup lang="ts">
import { ElButton, ElTag } from 'element-plus'
import type { TagProps } from 'element-plus'
import type { ProviderProfile } from '../types/provider'
import type {
  ProviderAvailabilityResult,
  ProviderTestKind,
  ProviderTestStatus,
} from '../types/providerAvailability'

defineProps<{
  provider: ProviderProfile
  apiResult: ProviderAvailabilityResult | null
  codexResult: ProviderAvailabilityResult | null
  runningKind: ProviderTestKind | null
  disabled: boolean
  cancelling: boolean
  disabledReason?: string | null
}>()

const emit = defineEmits<{
  testApi: []
  requestCodexTest: []
  cancel: []
}>()

const statusPresentation: Record<
  ProviderTestStatus,
  { label: string; type: TagProps['type'] }
> = {
  passed: { label: '通过', type: 'success' },
  failed: { label: '失败', type: 'danger' },
  unsupported: { label: '不支持', type: 'warning' },
  cancelled: { label: '已取消', type: 'info' },
}

function statusLabel(status: ProviderTestStatus) {
  return statusPresentation[status].label
}

function statusType(status: ProviderTestStatus) {
  return statusPresentation[status].type
}

function formatDuration(durationMs: number) {
  if (durationMs < 1_000) return `${Math.max(0, Math.round(durationMs))} ms`
  return `${(durationMs / 1_000).toFixed(1)} s`
}

function formatTestedAt(testedAt: string) {
  const date = new Date(testedAt)
  if (Number.isNaN(date.getTime())) return testedAt
  return new Intl.DateTimeFormat('zh-CN', {
    dateStyle: 'short',
    timeStyle: 'medium',
  }).format(date)
}

function providerReadinessReason(provider: ProviderProfile) {
  if (!provider.isValid) return 'Provider 配置无效，无法测试。'
  if (!provider.apiKeyConfigured) return '未配置 API Key，无法测试。'
  if (provider.preferenceConfigured === false || !provider.selectedModel) {
    return '未配置模型偏好，无法测试。'
  }
  return null
}

function isTestDisabled(
  provider: ProviderProfile,
  disabled: boolean,
  runningKind: ProviderTestKind | null,
) {
  return disabled || runningKind !== null || providerReadinessReason(provider) !== null
}

function blockedReason(
  provider: ProviderProfile,
  disabled: boolean,
  runningKind: ProviderTestKind | null,
  reason: string | null | undefined,
) {
  if (runningKind) return null
  if (reason) return reason
  if (disabled) return '当前有其他操作进行中，暂时不能开始测试。'
  return providerReadinessReason(provider)
}
</script>

<template>
  <section class="provider-availability-panel" aria-label="Provider 可用性测试">
    <header class="availability-header">
      <div>
        <p class="eyebrow">可用性测试</p>
        <h2 class="availability-title">验证当前 Provider 配置</h2>
      </div>
      <p class="availability-summary">仅在你点击后发起请求，测试结果不会持久化。</p>
    </header>

    <p v-if="runningKind" class="running-status" role="status" aria-live="polite">
      正在运行{{ runningKind === 'api' ? ' API 可用性测试' : ' Codex 兼容性测试' }}，可随时取消。
    </p>
    <p
      v-if="blockedReason(provider, disabled, runningKind, disabledReason)"
      class="disabled-reason"
      role="note"
    >
      {{ blockedReason(provider, disabled, runningKind, disabledReason) }}
    </p>

    <div class="availability-test-list">
      <section class="availability-test-card" aria-labelledby="api-availability-title">
        <div class="test-content">
          <div class="test-copy">
            <h3 id="api-availability-title">API 可用性</h3>
            <p>发送一次无工具、非流式、最多 16 个输出 token 的最小 Responses 请求，可能产生少量费用。</p>
          </div>
          <div
            v-if="apiResult"
            class="test-result"
            aria-label="API 测试结果"
            aria-live="polite"
          >
            <div class="result-heading">
              <ElTag :type="statusType(apiResult.status)" effect="plain">
                {{ statusLabel(apiResult.status) }}
              </ElTag>
              <span>{{ apiResult.message }}</span>
            </div>
            <div class="result-metadata">
              <span>模型 {{ apiResult.model }}</span>
              <span>{{ formatDuration(apiResult.durationMs) }}</span>
              <span>{{ formatTestedAt(apiResult.testedAt) }}</span>
              <span v-if="apiResult.httpStatus !== null">HTTP {{ apiResult.httpStatus }}</span>
            </div>
          </div>
        </div>
        <ElButton
          v-if="runningKind === 'api'"
          type="warning"
          plain
          native-type="button"
          :aria-label="`取消 ${provider.name} 的 API 可用性测试`"
          :disabled="cancelling"
          :loading="cancelling"
          @click="emit('cancel')"
        >
          {{ cancelling ? '正在取消…' : '取消 API 可用性测试' }}
        </ElButton>
        <ElButton
          v-else
          type="primary"
          plain
          native-type="button"
          :aria-label="`测试 ${provider.name} 的 API 可用性`"
          :disabled="isTestDisabled(provider, disabled, runningKind) || cancelling"
          @click="emit('testApi')"
        >
          测试 API 可用性
        </ElButton>
      </section>

      <section class="availability-test-card" aria-labelledby="codex-availability-title">
        <div class="test-content">
          <div class="test-copy">
            <div class="advanced-heading">
              <h3 id="codex-availability-title">Codex 兼容性</h3>
              <ElTag type="warning" effect="plain" size="small">高级</ElTag>
            </div>
            <p>启动本机 Codex 并发送一次正常 Codex 回合，token 消耗和等待时间可能明显高于 API 测试。</p>
          </div>
          <div
            v-if="codexResult"
            class="test-result"
            aria-label="Codex 兼容性测试结果"
            aria-live="polite"
          >
            <div class="result-heading">
              <ElTag :type="statusType(codexResult.status)" effect="plain">
                {{ statusLabel(codexResult.status) }}
              </ElTag>
              <span>{{ codexResult.message }}</span>
            </div>
            <div class="result-metadata">
              <span>模型 {{ codexResult.model }}</span>
              <span>{{ formatDuration(codexResult.durationMs) }}</span>
              <span>{{ formatTestedAt(codexResult.testedAt) }}</span>
              <span v-if="codexResult.codexVersion">Codex {{ codexResult.codexVersion }}</span>
            </div>
          </div>
        </div>
        <ElButton
          v-if="runningKind === 'codex'"
          type="warning"
          plain
          native-type="button"
          :aria-label="`取消 ${provider.name} 的 Codex 兼容性测试`"
          :disabled="cancelling"
          :loading="cancelling"
          @click="emit('cancel')"
        >
          {{ cancelling ? '正在取消…' : '取消 Codex 兼容性测试' }}
        </ElButton>
        <ElButton
          v-else
          plain
          native-type="button"
          :aria-label="`运行 ${provider.name} 的 Codex 兼容性测试`"
          :disabled="isTestDisabled(provider, disabled, runningKind) || cancelling"
          @click="emit('requestCodexTest')"
        >
          运行 Codex 兼容性测试
        </ElButton>
      </section>
    </div>
  </section>
</template>

<style scoped>
.provider-availability-panel,
.availability-test-list,
.test-content,
.test-copy {
  display: grid;
}

.provider-availability-panel,
.availability-test-list {
  gap: 1rem;
}

.availability-header,
.availability-test-card,
.advanced-heading,
.result-heading,
.result-metadata {
  display: flex;
  align-items: center;
}

.availability-header,
.availability-test-card {
  justify-content: space-between;
  gap: 1rem;
}

.availability-title,
.availability-summary,
.test-copy h3,
.test-copy p {
  margin: 0;
}

.availability-summary,
.running-status,
.disabled-reason,
.test-copy p {
  color: var(--text-secondary);
}

.availability-summary {
  max-width: 24rem;
  text-align: right;
}

.running-status {
  margin: 0;
}

.disabled-reason {
  margin: 0;
}

.availability-test-card {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  padding: 1rem;
  background: var(--surface-muted);
}

.test-copy {
  gap: 0.45rem;
  max-width: 34rem;
}

.test-content {
  gap: 0.75rem;
}

.advanced-heading,
.result-heading,
.result-metadata {
  gap: 0.5rem;
}

.test-result {
  display: grid;
  gap: 0.45rem;
}

.result-heading {
  align-items: flex-start;
}

.result-metadata {
  flex-wrap: wrap;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.eyebrow {
  margin: 0;
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

@media (max-width: 760px) {
  .availability-header,
  .availability-test-card {
    align-items: stretch;
    flex-direction: column;
  }

  .availability-summary {
    max-width: none;
    text-align: left;
  }

  .availability-test-card :deep(.el-button) {
    width: 100%;
  }
}
</style>
