<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import AppNotification from '../components/AppNotification.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ProviderEditor from '../components/ProviderEditor.vue'
import ProviderList from '../components/ProviderList.vue'
import ProviderStatus from '../components/ProviderStatus.vue'
import { useProviders } from '../composables/useProviders'
import type { CreateProviderInput, UpdateProviderInput } from '../types/provider'

const props = withDefaults(defineProps<{ startCreating?: boolean }>(), {
  startCreating: false,
})
const emit = defineEmits<{
  providerCreated: []
  createCancelled: []
}>()

const providerState = useProviders()
const editorMode = shallowRef<'create' | 'edit' | null>(props.startCreating ? 'create' : null)
const editingProviderId = shallowRef<string | null>(null)
const deleteProviderId = shallowRef<string | null>(null)
const confirmImportCurrentKey = shallowRef(false)

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
</script>

<template>
  <main class="providers-view">
    <ProviderList
      class="providers-column"
      :providers="providerState.providers.value"
      :selected-provider-id="providerState.selectedProviderId.value"
      :busy="providerState.busy.value"
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

      <ProviderEditor
        v-if="editorMode && providerState.fingerprints.value"
        :key="`${editorMode}-${editingProviderId ?? 'new'}`"
        :mode="editorMode"
        :provider="editingProvider"
        :fingerprints="providerState.fingerprints.value"
        :existing-ids="providerState.providers.value.map((provider) => provider.id)"
        :busy="providerState.busy.value"
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
            <dt>默认模型</dt>
            <dd>{{ providerState.selectedProvider.value.model || '跟随 Codex 当前设置' }}</dd>
          </div>
          <div>
            <dt>API Key</dt>
            <dd>{{ providerState.selectedProvider.value.apiKeyConfigured ? '密钥已配置' : '未配置密钥' }}</dd>
          </div>
        </dl>
        <div class="detail-actions">
          <button
            type="button"
            aria-label="编辑所选 Provider"
            :disabled="providerState.busy.value"
            @click="openEdit(providerState.selectedProvider.value.id)"
          >
            编辑
          </button>
          <button
            type="button"
            aria-label="使用所选 Provider"
            :disabled="
              providerState.busy.value ||
              providerState.selectedProvider.value.isActive ||
              !providerState.selectedProvider.value.isValid ||
              !providerState.selectedProvider.value.apiKeyConfigured
            "
            @click="providerState.switchTo(providerState.selectedProvider.value.id)"
          >
            使用此 Provider
          </button>
          <button
            type="button"
            aria-label="删除所选 Provider"
            :disabled="providerState.busy.value || providerState.selectedProvider.value.isActive"
            @click="requestDelete(providerState.selectedProvider.value.id)"
          >
            删除
          </button>
        </div>
        <aside
          v-if="providerState.currentAuthImportAvailable.value && providerState.activeProvider.value"
          class="import-key-callout"
        >
          <p>检测到当前 auth.json 中存在尚未保存到当前 Provider 的 API Key。</p>
          <button
            type="button"
            aria-label="导入当前 auth.json 密钥"
            @click="confirmImportCurrentKey = true"
          >
            导入当前密钥
          </button>
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
  border-right: 1px solid #e4e7ec;
  background: #f8fafc;
}

.provider-detail {
  display: grid;
  align-content: start;
  gap: 0.9rem;
}

.detail-placeholder {
  align-self: center;
  max-width: 34rem;
  color: #475467;
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
  border-bottom: 1px solid #e4e7ec;
  padding-bottom: 0.75rem;
}

.selected-provider-fields dt {
  color: #667085;
}

.selected-provider-fields dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.detail-actions {
  justify-content: flex-start;
}

.import-key-callout {
  border: 1px solid #f3c969;
  border-radius: 0.8rem;
  padding: 0.8rem;
  background: #fffaeb;
}

.import-key-callout p {
  margin: 0;
}

.eyebrow {
  margin: 0;
  color: #356ae6;
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
    border-bottom: 1px solid #e4e7ec;
  }
}
</style>
