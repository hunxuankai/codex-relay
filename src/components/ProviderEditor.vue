<script setup lang="ts">
import { computed, nextTick, reactive, shallowRef, useTemplateRef, watch } from 'vue'
import { ElButton, ElCheckbox, ElInput, ElOption, ElSelect } from 'element-plus'
import type {
  ApiKeyChange,
  CreateProviderInput,
  FileSetFingerprint,
  ProviderProfile,
  ModelCatalogItem,
  UpdateProviderInput,
} from '../types/provider'
import ApiKeyInput from './ApiKeyInput.vue'

type EditorMode = 'create' | 'edit'

const props = withDefaults(
  defineProps<{
    mode: EditorMode
    provider: ProviderProfile | null
    fingerprints: FileSetFingerprint
    busy: boolean
    existingIds?: readonly string[]
    modelCatalog?: readonly ModelCatalogItem[]
  }>(),
  { existingIds: () => [], modelCatalog: () => [] },
)

const emit = defineEmits<{
  submit: [input: CreateProviderInput | UpdateProviderInput]
  cancel: []
}>()

const draft = reactive({
  id: '',
  name: '',
  baseUrl: '',
  models: [] as string[],
  apiKey: '',
  activateAfterSave: false,
  syncIfActive: false,
})
const errors = reactive({ id: '', name: '', baseUrl: '', models: '', apiKey: '' })
const apiKeyAction = shallowRef<ApiKeyChange['action']>('unchanged')
const form = useTemplateRef<HTMLFormElement>('form')
const activeFieldsChanged = computed(() => {
  if (props.mode !== 'edit' || !props.provider?.isActive) return false
  return (
    draft.baseUrl.trim() !== props.provider.baseUrl ||
    draft.name.trim() !== props.provider.name ||
    JSON.stringify(draft.models) !== JSON.stringify(props.provider.models) ||
    apiKeyAction.value === 'set' ||
    apiKeyAction.value === 'clear'
  )
})
const canSyncActiveChanges = computed(
  () => activeFieldsChanged.value && apiKeyAction.value !== 'clear',
)
const currentProviderWarning = computed(() =>
  apiKeyAction.value === 'clear'
    ? '清除当前 Provider 的 API Key 后无法立即同步；当前 auth.json 不会被改写，Codex 可能继续使用现有生效密钥。'
    : '这是当前 Provider。修改生效字段后，可以选择立即同步到 Codex。',
)

watch(
  () => [props.mode, props.provider] as const,
  () => {
    draft.id = props.provider?.id ?? ''
    draft.name = props.provider?.name ?? ''
    draft.baseUrl = props.provider?.baseUrl ?? ''
    draft.models = [...(props.provider?.models ?? [])]
    draft.apiKey = ''
    draft.activateAfterSave = false
    draft.syncIfActive = false
    apiKeyAction.value = props.mode === 'create' ? 'set' : 'unchanged'
    clearErrors()
  },
  { immediate: true },
)

watch(
  () => draft.apiKey,
  (value) => {
    if (props.mode === 'edit') {
      apiKeyAction.value = value ? 'set' : apiKeyAction.value === 'clear' ? 'clear' : 'unchanged'
    }
  },
)

watch(canSyncActiveChanges, (canSync) => {
  if (!canSync) draft.syncIfActive = false
})

function clearErrors() {
  errors.id = ''
  errors.name = ''
  errors.baseUrl = ''
  errors.models = ''
  errors.apiKey = ''
}

function normalizeId() {
  draft.id = draft.id.trim().toLowerCase()
}

function validate() {
  clearErrors()
  normalizeId()
  if (!draft.id) errors.id = 'Provider ID 为必填项。'
  else if (!/^[a-z0-9_-]+$/.test(draft.id)) errors.id = 'Provider ID 仅支持小写字母、数字、_ 和 -。'
  else if (props.mode === 'create' && props.existingIds.includes(draft.id)) {
    errors.id = 'Provider ID 已存在。'
  }
  if (!draft.name.trim()) errors.name = '名称为必填项。'
  if (!draft.baseUrl.trim()) {
    errors.baseUrl = 'Base URL 为必填项。'
  } else {
    try {
      const url = new URL(draft.baseUrl.trim())
      if (!['http:', 'https:'].includes(url.protocol)) {
        errors.baseUrl = 'Base URL 必须使用 HTTP 或 HTTPS。'
      }
    } catch {
      errors.baseUrl = 'Base URL 必须是有效的网址。'
    }
  }
  if (props.mode === 'create' && !draft.apiKey.trim()) errors.apiKey = 'API Key 为必填项。'
  if (draft.models.length === 0) errors.models = '请至少选择一个可用模型。'
  return !errors.id && !errors.name && !errors.baseUrl && !errors.models && !errors.apiKey
}

function handleClearKey() {
  apiKeyAction.value = 'clear'
  draft.apiKey = ''
}

async function submit() {
  if (!validate()) {
    await nextTick()
    form.value?.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus()
    return
  }
  const common = {
    id: draft.id,
    name: draft.name.trim(),
    baseUrl: draft.baseUrl.trim(),
    wireApi: 'responses',
    models: [...draft.models],
    expectedFiles: props.fingerprints,
  }
  if (props.mode === 'create') {
    emit('submit', {
      ...common,
      apiKey: draft.apiKey.trim(),
      activateAfterSave: draft.activateAfterSave,
    })
    return
  }
  let apiKeyChange: ApiKeyChange = { action: 'unchanged' }
  if (apiKeyAction.value === 'set') apiKeyChange = { action: 'set', value: draft.apiKey.trim() }
  if (apiKeyAction.value === 'clear') apiKeyChange = { action: 'clear' }
  emit('submit', {
    ...common,
    apiKeyChange,
    syncIfActive: canSyncActiveChanges.value ? draft.syncIfActive : false,
  })
}
</script>

<template>
  <section class="provider-editor" :aria-label="mode === 'create' ? '新增 Provider' : '编辑 Provider'">
    <header class="editor-header">
      <div>
        <p class="eyebrow">{{ mode === 'create' ? 'New Provider' : provider?.id }}</p>
        <h2 class="editor-title">{{ mode === 'create' ? '新增 Provider' : `编辑 ${provider?.name}` }}</h2>
      </div>
      <ElButton native-type="button" :disabled="busy" @click="emit('cancel')">取消</ElButton>
    </header>

    <p v-if="mode === 'edit' && provider?.isActive" class="current-warning" role="note">
      {{ currentProviderWarning }}
    </p>

    <form ref="form" class="editor-form" novalidate @submit.prevent="submit">
      <label class="field">
        <span>Provider ID</span>
        <ElInput
          v-model="draft.id"
          name="provider-id"
          :disabled="busy || mode === 'edit'"
          :aria-invalid="errors.id ? 'true' : undefined"
          :aria-describedby="errors.id ? 'provider-id-error' : undefined"
          autocomplete="off"
          @input="normalizeId"
        />
        <span v-if="errors.id" id="provider-id-error" class="field-error" role="alert">{{ errors.id }}</span>
      </label>

      <label class="field">
        <span>名称</span>
        <ElInput
          v-model="draft.name"
          name="provider-name"
          :disabled="busy"
          :aria-invalid="errors.name ? 'true' : undefined"
          :aria-describedby="errors.name ? 'provider-name-error' : undefined"
          autocomplete="off"
        />
        <span v-if="errors.name" id="provider-name-error" class="field-error" role="alert">{{ errors.name }}</span>
      </label>

      <label class="field">
        <span>Base URL</span>
        <ElInput
          v-model="draft.baseUrl"
          name="base-url"
          type="url"
          :disabled="busy"
          :aria-invalid="errors.baseUrl ? 'true' : undefined"
          :aria-describedby="errors.baseUrl ? 'base-url-error' : undefined"
          autocomplete="off"
        />
        <span v-if="errors.baseUrl" id="base-url-error" class="field-error" role="alert">{{ errors.baseUrl }}</span>
      </label>

      <label class="field">
        <span>Wire API</span>
        <ElInput name="wire-api" model-value="responses" disabled />
      </label>

      <div class="field">
        <label for="provider-models">可用模型</label>
        <ElSelect
          id="provider-models"
          v-model="draft.models"
          name="models"
          multiple
          :disabled="busy"
          placeholder="请选择一个或多个模型"
          :aria-invalid="errors.models ? 'true' : undefined"
          :aria-describedby="errors.models ? 'provider-models-error model-hint' : 'model-hint'"
        >
          <ElOption
            v-for="model in modelCatalog ?? []"
            :key="model.id"
            :label="model.id"
            :value="model.id"
          />
        </ElSelect>
        <span id="model-hint" class="field-hint">
          第一个选择的模型会成为初始偏好；推理强度可在 Provider 详情中设置。
        </span>
        <span v-if="provider?.selectedModel" class="field-hint">
          当前偏好：{{ provider.selectedModel }}
        </span>
        <span v-if="errors.models" id="provider-models-error" class="field-error" role="alert">
          {{ errors.models }}
        </span>
      </div>

      <div class="field">
        <ApiKeyInput
          v-model="draft.apiKey"
          :configured="mode === 'edit' && Boolean(provider?.apiKeyConfigured)"
          :disabled="busy"
          :invalid="Boolean(errors.apiKey)"
          :described-by="errors.apiKey ? 'api-key-error' : undefined"
          @clear="handleClearKey"
        />
        <span v-if="errors.apiKey" id="api-key-error" class="field-error" role="alert">{{ errors.apiKey }}</span>
      </div>

      <ElCheckbox v-if="mode === 'create'" v-model="draft.activateAfterSave" class="check-field" name="activate-after-save">
        保存后立即启用
      </ElCheckbox>
      <ElCheckbox v-if="canSyncActiveChanges" v-model="draft.syncIfActive" class="check-field" name="sync-if-active">
        保存后立即同步当前 Codex 配置
      </ElCheckbox>

      <ElButton class="submit-button" type="primary" native-type="submit" :disabled="busy">
        {{ busy ? '正在保存…' : '保存 Provider' }}
      </ElButton>
    </form>
  </section>
</template>

<style scoped>
.provider-editor,
.editor-form,
.field {
  display: grid;
  gap: 0.75rem;
}

.editor-header {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 1rem;
}

.eyebrow,
.editor-title {
  margin: 0;
}

.eyebrow {
  color: var(--text-secondary);
  font-size: 0.78rem;
}

.field {
  gap: 0.35rem;
  font-weight: 600;
}

.field :deep(.el-input__inner) {
  font: inherit;
  font-weight: 400;
}

.field :deep(.el-select) {
  width: 100%;
}

.field-error,
.current-warning {
  color: var(--danger);
}

.field-error {
  font-size: 0.82rem;
  font-weight: 400;
}

.field-hint {
  color: var(--text-secondary);
  font-size: 0.82rem;
  font-weight: 400;
}

.current-warning {
  margin: 0;
  border-left: 3px solid currentColor;
  padding-left: 0.75rem;
}

.check-field {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.submit-button {
  justify-self: start;
  font-weight: 700;
}
</style>
