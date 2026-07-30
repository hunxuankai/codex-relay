<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import { ElButton } from 'element-plus'
import AppNotification from '../components/AppNotification.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ImportCurrentApiKeyDialog from '../components/ImportCurrentApiKeyDialog.vue'
import ProviderApiKeyManagerDialog from '../components/ProviderApiKeyManagerDialog.vue'
import ProviderEditor from '../components/ProviderEditor.vue'
import ProviderAvailabilityPanel from '../components/ProviderAvailabilityPanel.vue'
import ProviderBaseUrlManagerDialog from '../components/ProviderBaseUrlManagerDialog.vue'
import ProviderCredentialControls from '../components/ProviderCredentialControls.vue'
import ProviderEndpointControls from '../components/ProviderEndpointControls.vue'
import ProviderList from '../components/ProviderList.vue'
import ProviderPreferenceControls from '../components/ProviderPreferenceControls.vue'
import ProviderStatus from '../components/ProviderStatus.vue'
import { useProviderApiKeyManager } from '../composables/useProviderApiKeyManager'
import { useProviderAvailability } from '../composables/useProviderAvailability'
import { useProviders } from '../composables/useProviders'
import { RelayCommandError } from '../services/tauri'
import type {
  CreateProviderInput,
  ProviderBaseUrlDraft,
  UpdateProviderInput,
} from '../types/provider'
import type { ProviderTestKind } from '../types/providerAvailability'

const props = withDefaults(defineProps<{
  startCreating?: boolean
  networkProxyEnabled?: boolean
}>(), {
  startCreating: false,
  networkProxyEnabled: false,
})
const emit = defineEmits<{
  providerCreated: []
  createCancelled: []
}>()

const providerState = useProviders()
const availabilityState = useProviderAvailability()
const apiKeyManager = useProviderApiKeyManager({
  onSaved: async () => {
    if (!(await providerState.refresh())) {
      throw new RelayCommandError(
        'PROVIDER_REFRESH_FAILED',
        'API Key 已保存，但 Provider 状态刷新失败，请重新加载。',
      )
    }
  },
})
const editorMode = shallowRef<'create' | 'edit' | null>(props.startCreating ? 'create' : null)
const editingProviderId = shallowRef<string | null>(null)
const deleteProviderId = shallowRef<string | null>(null)
const confirmImportCurrentKey = shallowRef(false)
const confirmCodexTestRequest = shallowRef<{
  providerId: string
  useProxy: boolean
} | null>(null)
const baseUrlManagerProviderId = shallowRef<string | null>(null)
const apiKeyManagerProviderId = shallowRef<string | null>(null)
const apiKeyManagerSuccessMessage = shallowRef<string | null>(null)
const apiKeyManagerSuccessMessageId = shallowRef(0)

const interactionBusy = computed(() =>
  providerState.busy.value ||
  availabilityState.busy.value ||
  apiKeyManager.loading.value ||
  apiKeyManager.busy.value,
)

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
const baseUrlManagerProvider = computed(
  () => providerState.providers.value.find(
    (provider) => provider.id === baseUrlManagerProviderId.value,
  ) ?? null,
)
const apiKeyManagerProvider = computed(
  () => providerState.providers.value.find(
    (provider) => provider.id === apiKeyManagerProviderId.value,
  ) ?? null,
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

async function importCurrentKey(name: string) {
  const providerId = providerState.activeProvider.value?.id
  if (!providerId) return
  const outcome = await providerState.importCurrentKey(providerId, name)
  if (outcome) confirmImportCurrentKey.value = false
}

function openBaseUrlManager(providerId: string) {
  baseUrlManagerProviderId.value = providerId
}

async function saveBaseUrls(entries: ProviderBaseUrlDraft[]) {
  const providerId = baseUrlManagerProviderId.value
  if (!providerId) return
  const outcome = await providerState.saveBaseUrls(providerId, entries)
  if (outcome) baseUrlManagerProviderId.value = null
}

async function openApiKeyManager(providerId: string) {
  apiKeyManagerSuccessMessage.value = null
  apiKeyManagerProviderId.value = providerId
  await apiKeyManager.load(providerId)
}

function closeApiKeyManager() {
  apiKeyManagerProviderId.value = null
  apiKeyManager.clear()
}

async function saveApiKeys() {
  const outcome = await apiKeyManager.save()
  if (!outcome) return
  apiKeyManagerSuccessMessage.value = outcome.message
  apiKeyManagerSuccessMessageId.value += 1
  closeApiKeyManager()
}

function updateSelectedPreference(model: string, reasoningEffort: string) {
  const providerId = providerState.selectedProvider.value?.id
  if (!providerId) return
  void providerState.updatePreference(providerId, model, reasoningEffort)
}

function updateSelectedFast(enabled: boolean) {
  const providerId = providerState.selectedProvider.value?.id
  if (!providerId) return
  void providerState.updateFast(providerId, enabled)
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

function startApiTest(useProxy: boolean) {
  const providerId = providerState.selectedProvider.value?.id
  if (providerId) void availabilityState.testApi(providerId, useProxy)
}

function requestCodexTest(useProxy: boolean) {
  const providerId = providerState.selectedProvider.value?.id
  if (providerId) confirmCodexTestRequest.value = { providerId, useProxy }
}

async function confirmCodexTest() {
  const request = confirmCodexTestRequest.value
  confirmCodexTestRequest.value = null
  if (!request || !providerState.providers.value.some((provider) => provider.id === request.providerId)) {
    return
  }
  await availabilityState.testCodex(request.providerId, request.useProxy)
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

watch(baseUrlManagerProvider, (provider) => {
  if (baseUrlManagerProviderId.value && !provider) baseUrlManagerProviderId.value = null
})
watch(apiKeyManagerProvider, (provider) => {
  if (apiKeyManagerProviderId.value && !provider) closeApiKeyManager()
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
      @reorder="providerState.reorder"
    />

    <section class="provider-detail" aria-label="Provider 详情">
      <AppNotification
        :message="providerState.successMessage.value"
        level="success"
      />
      <AppNotification
        :message="apiKeyManagerSuccessMessage"
        :message-id="apiKeyManagerSuccessMessageId"
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
          <div><dt>Wire API</dt><dd>{{ providerState.selectedProvider.value.wireApi }}</dd></div>
          <div>
            <dt>配置状态</dt>
            <dd>
              {{
                providerState.selectedProvider.value.configurationComplete
                  ? '配置完整'
                  : providerState.selectedProvider.value.disabledReason ?? '配置不完整'
              }}
            </dd>
          </div>
        </dl>
        <div class="provider-switch-controls">
          <ProviderEndpointControls
            :provider="providerState.selectedProvider.value"
            :busy="interactionBusy"
            @select="providerState.selectBaseUrl(providerState.selectedProvider.value.id, $event)"
            @manage="openBaseUrlManager(providerState.selectedProvider.value.id)"
          />
          <ProviderCredentialControls
            :provider="providerState.selectedProvider.value"
            :busy="interactionBusy"
            @select="providerState.selectApiKey(providerState.selectedProvider.value.id, $event)"
            @manage="openApiKeyManager(providerState.selectedProvider.value.id)"
          />
        </div>
        <ProviderPreferenceControls
          :provider="providerState.selectedProvider.value"
          :model-catalog="providerState.modelCatalog.value"
          :busy="interactionBusy"
          @select="updateSelectedPreference"
          @update-fast="updateSelectedFast"
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
          :network-proxy-enabled="networkProxyEnabled"
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
              !providerState.selectedProvider.value.configurationComplete
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
    <ProviderBaseUrlManagerDialog
      v-if="baseUrlManagerProvider"
      :open="Boolean(baseUrlManagerProviderId)"
      :provider-name="baseUrlManagerProvider.name"
      :entries="baseUrlManagerProvider.baseUrls"
      :selected-base-url-id="baseUrlManagerProvider.selectedBaseUrlId"
      :external-url="baseUrlManagerProvider.baseUrlStatus === 'external' ? baseUrlManagerProvider.baseUrl : null"
      :busy="providerState.busy.value"
      @save="saveBaseUrls"
      @close="baseUrlManagerProviderId = null"
    />
    <ProviderApiKeyManagerDialog
      v-if="apiKeyManagerProvider"
      :open="Boolean(apiKeyManagerProviderId)"
      :provider-name="apiKeyManagerProvider.name"
      :entries="apiKeyManager.entries.value"
      :selected-api-key-id="apiKeyManager.selectedApiKeyId.value"
      :api-key-status="apiKeyManager.apiKeyStatus.value"
      :loading="apiKeyManager.loading.value"
      :busy="apiKeyManager.busy.value"
      :error-message="apiKeyManager.error.value?.message ?? null"
      :success-message="apiKeyManager.successMessage.value"
      @replace-entries="apiKeyManager.replaceEntries"
      @save="saveApiKeys"
      @close="closeApiKeyManager"
    />
    <ImportCurrentApiKeyDialog
      :open="confirmImportCurrentKey"
      :provider-name="providerState.activeProvider.value?.name ?? ''"
      :busy="providerState.busy.value"
      @import="importCurrentKey"
      @close="confirmImportCurrentKey = false"
    />
    <ConfirmDialog
      :open="Boolean(confirmCodexTestRequest)"
      title="确认运行 Codex 兼容性测试"
      message="这会在本机启动 Codex 并向 Provider 发送一次正常 Codex 回合，可能产生高于 API 测试的 token 消耗；不会修改当前 config.toml 或 auth.json。是否继续？"
      confirm-label="继续测试"
      tone="neutral"
      @confirm="confirmCodexTest"
      @cancel="confirmCodexTestRequest = null"
    />
  </main>
</template>

<style scoped>
.providers-view {
  display: grid;
  grid-template-columns: minmax(17rem, 0.8fr) minmax(25rem, 1.5fr);
  min-height: 100%;
}

.providers-column,
.provider-detail {
  padding: 1rem;
}

.providers-column {
  border-right: 1px solid var(--border);
  background: var(--surface-muted);
}

.provider-detail {
  display: grid;
  align-content: start;
  gap: 0.7rem;
}

.detail-placeholder {
  align-self: center;
  max-width: 34rem;
  color: var(--text-secondary);
}

.selected-provider-detail,
.selected-provider-fields,
.provider-switch-controls {
  display: grid;
  gap: 0.75rem;
  min-width: 0;
}

.selected-provider-fields {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.provider-switch-controls {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.selected-provider-header,
.detail-actions,
.import-key-callout {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.selected-provider-header h1,
.selected-provider-fields {
  margin: 0;
}

.selected-provider-header h1 {
  font-size: 1.75rem;
  line-height: 1.15;
}

.selected-provider-fields div {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 0.65rem;
  padding: 0.55rem 0.65rem;
  background: var(--surface);
}

.selected-provider-fields dt {
  color: var(--text-secondary);
  font-size: 0.78rem;
}

.selected-provider-fields dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.detail-actions {
  flex-wrap: wrap;
  justify-content: flex-start;
}

.import-key-callout {
  border: 1px solid var(--warning-border);
  border-radius: 0.7rem;
  padding: 0.7rem;
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

@media (max-width: 620px) {
  .selected-provider-fields,
  .provider-switch-controls {
    grid-template-columns: 1fr;
  }

  .selected-provider-header,
  .import-key-callout {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
