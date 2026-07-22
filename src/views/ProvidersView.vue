<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import { ElButton } from 'element-plus'
import AppNotification from '../components/AppNotification.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ProviderEditor from '../components/ProviderEditor.vue'
import ProviderAvailabilityPanel from '../components/ProviderAvailabilityPanel.vue'
import ProviderList from '../components/ProviderList.vue'
import ProviderPreferenceControls from '../components/ProviderPreferenceControls.vue'
import ProviderStatus from '../components/ProviderStatus.vue'
import { useProviderAvailability } from '../composables/useProviderAvailability'
import { useProviders } from '../composables/useProviders'
import type { CreateProviderInput, UpdateProviderInput } from '../types/provider'
import type { ProviderTestKind } from '../types/providerAvailability'

const props = withDefaults(defineProps<{ startCreating?: boolean }>(), {
  startCreating: false,
})
const emit = defineEmits<{
  providerCreated: []
  createCancelled: []
}>()

const providerState = useProviders()
const availabilityState = useProviderAvailability()
const editorMode = shallowRef<'create' | 'edit' | null>(props.startCreating ? 'create' : null)
const editingProviderId = shallowRef<string | null>(null)
const deleteProviderId = shallowRef<string | null>(null)
const confirmImportCurrentKey = shallowRef(false)
const confirmCodexProviderId = shallowRef<string | null>(null)

const interactionBusy = computed(() => providerState.busy.value || availabilityState.busy.value)

const editingProvider = computed(
  () =>
    providerState.providers.value.find((provider) => provider.id === editingProviderId.value) ??
    null,
)
const deleteProvider = computed(
  () =>
    providerState.providers.value.find((provider) => provider.id === deleteProviderId.value) ??
    null,
)

function openCreate() {
  editorMode.value = 'create'
  editingProviderId.value = null
}

function openEdit(providerId: string) {
  providerState.selectProvider(providerId)
  editingProviderId.value = providerId
  editorMode.value = 'edit'
}

function closeEditor() {
  editorMode.value = null
  editingProviderId.value = null
}

function cancelEditor() {
  const cancelledCreate = editorMode.value === 'create'
  closeEditor()
  if (cancelledCreate) emit('createCancelled')
}

async function submitEditor(input: CreateProviderInput | UpdateProviderInput) {
  if ('apiKey' in input) {
    const outcome = await providerState.create(input)
    if (outcome) emit('providerCreated')
  } else {
    await providerState.update(input)
  }
  if (!providerState.error.value) closeEditor()
}

function requestDelete(providerId: string) {
  deleteProviderId.value = providerId
}

async function confirmDelete() {
  const providerId = deleteProviderId.value
  if (!providerId) return
  await providerState.remove(providerId)
  deleteProviderId.value = null
}

async function importCurrentKey() {
  const providerId = providerState.activeProvider.value?.id
  if (!providerId) return
  await providerState.importCurrentKey(providerId)
  confirmImportCurrentKey.value = false
}

function updateSelectedPreference(model: string, reasoningEffort: string) {
  const providerId = providerState.selectedProvider.value?.id
  if (!providerId) return
  void providerState.updatePreference(providerId, model, reasoningEffort)
}

const selectedApiResult = computed(() => {
  const providerId = providerState.selectedProvider.value?.id
  return providerId ? availabilityState.resultFor(providerId, 'api') : null
})

const selectedCodexResult = computed(() => {
  const providerId = providerState.selectedProvider.value?.id
  return providerId ? availabilityState.resultFor(providerId, 'codex') : null
})

const selectedRunningKind = computed<ProviderTestKind | null>(() => {
  const providerId = providerState.selectedProvider.value?.id
  if (!providerId || availabilityState.runningProviderId.value !== providerId) return null
  return availabilityState.runningKind.value
})

const availabilityDisabledReason = computed(() => {
  const selectedId = providerState.selectedProvider.value?.id
  if (providerState.busy.value) return 'Provider 配置操作进行中，暂时不能开始测试。'
  if (
    availabilityState.busy.value &&
    availabilityState.runningProviderId.value !== selectedId
  ) {
    return '另一个 Provider 的测试正在运行。'
  }
  return null
})

function startApiTest() {
  const providerId = providerState.selectedProvider.value?.id
  if (providerId) void availabilityState.testApi(providerId)
}

function requestCodexTest() {
  const providerId = providerState.selectedProvider.value?.id
  if (providerId) confirmCodexProviderId.value = providerId
}

async function confirmCodexTest() {
  const providerId = confirmCodexProviderId.value
  confirmCodexProviderId.value = null
  if (!providerId || !providerState.providers.value.some((provider) => provider.id === providerId)) return
  await availabilityState.testCodex(providerId)
}

function cancelAvailabilityTest() {
  void availabilityState.cancel()
}

const fingerprintKey = computed(() => JSON.stringify(providerState.fingerprints.value))
let initialFingerprintKey = fingerprintKey.value
watch(fingerprintKey, (next) => {
  if (next === initialFingerprintKey) return
  initialFingerprintKey = next
  availabilityState.invalidateAll()
})
</script>

<template>
  <main class="providers-view">
    <ProviderList
      class="providers-column"
      :providers="providerState.providers.value"
      :selected-provider-id="providerState.selectedProviderId.value"
      :busy="interactionBusy"
      @create="openCreate"
      @select="providerState.selectProvider"
      @edit="openEdit"
      @use="providerState.switchTo"
      @delete="requestDelete"
    />

    <section class="provider-detail" aria-label="Provider 详情">
      <AppNotification
        :message="providerState.successMessage.value"
        level="success"
      />
      <AppNotification
        :message="providerState.error.value?.message ?? null"
        level="error"
      />
      <AppNotification
        :message="availabilityState.error.value?.message ?? null"
        level="error"
      />

      <ProviderEditor
        v-if="editorMode && providerState.fingerprints.value"
        :key="`${editorMode}-${editingProviderId ?? 'new'}`"
        :mode="editorMode"
        :provider="editingProvider"
        :fingerprints="providerState.fingerprints.value"
        :existing-ids="providerState.providers.value.map((provider) => provider.id)"
        :busy="interactionBusy"
        :model-catalog="providerState.modelCatalog.value"
        @submit="submitEditor"
        @cancel="cancelEditor"
      />
      <article
        v-else-if="providerState.selectedProvider.value"
        class="selected-provider-detail"
        aria-label="所选 Provider 详情"
      >
        <header class="selected-provider-header">
          <div>
            <p class="eyebrow">{{ providerState.selectedProvider.value.id }}</p>
            <h1>{{ providerState.selectedProvider.value.name }}</h1>
          </div>
          <ProviderStatus :provider="providerState.selectedProvider.value" />
        </header>
        <dl class="selected-provider-fields">
          <div><dt>Provider ID</dt><dd>{{ providerState.selectedProvider.value.id }}</dd></div>
          <div><dt>Base URL</dt><dd>{{ providerState.selectedProvider.value.baseUrl }}</dd></div>
          <div><dt>Wire API</dt><dd>{{ providerState.selectedProvider.value.wireApi }}</dd></div>
          <div>
            <dt>API Key</dt>
            <dd>{{ providerState.selectedProvider.value.apiKeyConfigured ? '密钥已配置' : '未配置密钥' }}</dd>
          </div>
        </dl>
        <ProviderPreferenceControls
          :provider="providerState.selectedProvider.value"
          :model-catalog="providerState.modelCatalog.value"
          :busy="interactionBusy"
          @select="updateSelectedPreference"
          @configure="openEdit(providerState.selectedProvider.value.id)"
        />
        <ProviderAvailabilityPanel
          :provider="providerState.selectedProvider.value"
          :api-result="selectedApiResult"
          :codex-result="selectedCodexResult"
          :running-kind="selectedRunningKind"
          :disabled="interactionBusy"
          :disabled-reason="availabilityDisabledReason"
          :cancelling="availabilityState.cancelling.value"
          @test-api="startApiTest"
          @request-codex-test="requestCodexTest"
          @cancel="cancelAvailabilityTest"
        />
        <div class="detail-actions">
          <ElButton
            native-type="button"
            aria-label="编辑所选 Provider"
            :disabled="interactionBusy"
            @click="openEdit(providerState.selectedProvider.value.id)"
          >
            编辑
          </ElButton>
          <ElButton
            type="primary"
            native-type="button"
            aria-label="使用所选 Provider"
            :disabled="
              interactionBusy ||
              providerState.selectedProvider.value.isActive ||
              !providerState.selectedProvider.value.isValid ||
              !providerState.selectedProvider.value.apiKeyConfigured ||
              !providerState.selectedProvider.value.preferenceConfigured
            "
            @click="providerState.switchTo(providerState.selectedProvider.value.id)"
          >
            使用此 Provider
          </ElButton>
          <ElButton
            type="danger"
            plain
            native-type="button"
            aria-label="删除所选 Provider"
            :disabled="interactionBusy || providerState.selectedProvider.value.isActive"
            @click="requestDelete(providerState.selectedProvider.value.id)"
          >
            删除
          </ElButton>
        </div>
        <aside
          v-if="providerState.currentAuthImportAvailable.value && providerState.activeProvider.value"
          class="import-key-callout"
        >
          <p>检测到当前 auth.json 中存在尚未保存到当前 Provider 的 API Key。</p>
          <ElButton
            type="warning"
            plain
            native-type="button"
            aria-label="导入当前 auth.json 密钥"
            :disabled="interactionBusy"
            @click="confirmImportCurrentKey = true"
          >
            导入当前密钥
          </ElButton>
        </aside>
      </article>
      <div v-else class="detail-placeholder">
        <p class="eyebrow">Codex Relay</p>
        <h1>安全管理 Codex Provider</h1>
        <p>选择一个 Provider 查看详情，或新增 Provider 开始配置。</p>
      </div>
    </section>

    <ConfirmDialog
      :open="Boolean(deleteProviderId)"
      title="确认删除 Provider"
      :message="`确定删除「${deleteProvider?.name ?? ''}」吗？将从 config.toml 删除对应 Provider 配置，并从 providers.json 删除对应 API Key；其他 Provider 会保留，操作前会自动创建备份。`"
      confirm-label="删除"
      @confirm="confirmDelete"
      @cancel="deleteProviderId = null"
    />
    <ConfirmDialog
      :open="confirmImportCurrentKey"
      title="确认导入当前密钥"
      message="确定将 auth.json 中当前生效的 API Key 保存到当前 Provider 吗？密钥会继续以明文保存在本机 providers.json 中。"
      confirm-label="确认导入"
      @confirm="importCurrentKey"
      @cancel="confirmImportCurrentKey = false"
    />
    <ConfirmDialog
      :open="Boolean(confirmCodexProviderId)"
      title="确认运行 Codex 兼容性测试"
      message="这会在本机启动 Codex 并向 Provider 发送一次正常 Codex 回合，可能产生高于 API 测试的 token 消耗；不会修改当前 config.toml 或 auth.json。是否继续？"
      confirm-label="继续测试"
      tone="neutral"
      @confirm="confirmCodexTest"
      @cancel="confirmCodexProviderId = null"
    />
  </main>
</template>

<style scoped>
.providers-view {
  display: grid;
  grid-template-columns: minmax(18rem, 0.9fr) minmax(24rem, 1.4fr);
  min-height: 100%;
}

.providers-column,
.provider-detail {
  padding: 1.25rem;
}

.providers-column {
  border-right: 1px solid var(--border);
  background: var(--surface-muted);
}

.provider-detail {
  display: grid;
  align-content: start;
  gap: 0.9rem;
}

.detail-placeholder {
  align-self: center;
  max-width: 34rem;
  color: var(--text-secondary);
}

.selected-provider-detail,
.selected-provider-fields {
  display: grid;
  gap: 1rem;
}

.selected-provider-header,
.detail-actions,
.import-key-callout {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.selected-provider-header h1,
.selected-provider-fields {
  margin: 0;
}

.selected-provider-fields div {
  display: grid;
  grid-template-columns: 8rem minmax(0, 1fr);
  gap: 1rem;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.75rem;
}

.selected-provider-fields dt {
  color: var(--text-secondary);
}

.selected-provider-fields dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.detail-actions {
  justify-content: flex-start;
}

.import-key-callout {
  border: 1px solid var(--warning-border);
  border-radius: 0.8rem;
  padding: 0.8rem;
  background: var(--warning-soft);
}

.import-key-callout p {
  margin: 0;
}

.eyebrow {
  margin: 0;
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

@media (max-width: 760px) {
  .providers-view {
    grid-template-columns: 1fr;
  }

  .providers-column {
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }
}
</style>
