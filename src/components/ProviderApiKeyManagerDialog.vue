<script setup lang="ts">
import { computed, nextTick, shallowRef, watch } from 'vue'
import { ElButton, ElDialog, ElInput } from 'element-plus'
import type {
  ProviderApiKeyDraft,
  ProviderApiKeyStatus,
} from '../types/provider'

const props = defineProps<{
  open: boolean
  providerName: string
  entries: readonly ProviderApiKeyDraft[]
  selectedApiKeyId: string | null
  apiKeyStatus: ProviderApiKeyStatus | null
  loading: boolean
  busy: boolean
  errorMessage: string | null
  successMessage?: string | null
}>()

const emit = defineEmits<{
  replaceEntries: [entries: ProviderApiKeyDraft[]]
  save: []
  close: []
}>()

const hidden = shallowRef(false)
const copyMessage = shallowRef<string | null>(null)
const previousFocus = shallowRef<HTMLElement | null>(null)

watch(
  () => props.open,
  async (open) => {
    hidden.value = false
    copyMessage.value = null
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
      return
    }
    await nextTick()
    previousFocus.value?.focus()
    previousFocus.value = null
  },
  { immediate: true },
)

const errors = computed(() => {
  const next = props.entries.map(() => ({ name: '', apiKey: '' }))
  const nameCounts = new Map<string, number>()
  const keyCounts = new Map<string, number>()

  for (const entry of props.entries) {
    const name = entry.name.trim().toLocaleLowerCase()
    const apiKey = entry.apiKey.trim()
    if (name) nameCounts.set(name, (nameCounts.get(name) ?? 0) + 1)
    if (apiKey) keyCounts.set(apiKey, (keyCounts.get(apiKey) ?? 0) + 1)
  }

  props.entries.forEach((entry, index) => {
    const name = entry.name.trim()
    const apiKey = entry.apiKey.trim()
    if (!name) next[index]!.name = '密钥名称为必填项。'
    else if ((nameCounts.get(name.toLocaleLowerCase()) ?? 0) > 1) {
      next[index]!.name = '密钥名称不能重复。'
    }
    if (!apiKey) next[index]!.apiKey = 'API Key 为必填项。'
    else if ((keyCounts.get(apiKey) ?? 0) > 1) next[index]!.apiKey = 'API Key 不能重复。'
  })
  return next
})
const hasErrors = computed(() =>
  props.entries.length === 0 || errors.value.some((entry) => entry.name || entry.apiKey),
)

function replaceEntry(index: number, field: 'name' | 'apiKey', value: string) {
  emit('replaceEntries', props.entries.map((entry, entryIndex) =>
    entryIndex === index ? { ...entry, [field]: value } : { ...entry },
  ))
}

function addEntry() {
  emit('replaceEntries', [
    ...props.entries.map((entry) => ({ ...entry })),
    { id: null, name: '', apiKey: '' },
  ])
}

function canDelete(entry: ProviderApiKeyDraft) {
  return props.entries.length > 1 && entry.id !== props.selectedApiKeyId
}

function removeEntry(index: number) {
  const entry = props.entries[index]
  if (!entry || !canDelete(entry)) return
  emit('replaceEntries', props.entries
    .filter((_, entryIndex) => entryIndex !== index)
    .map((item) => ({ ...item })))
}

async function copyEntry(entry: ProviderApiKeyDraft) {
  copyMessage.value = null
  try {
    await navigator.clipboard.writeText(entry.apiKey)
    copyMessage.value = `${entry.name.trim() || 'API Key'}已复制。`
  } catch {
    copyMessage.value = '复制失败，请重试。'
  }
}

function save() {
  if (!props.loading && !props.busy && !hasErrors.value) emit('save')
}

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('close')
}
</script>

<template>
  <ElDialog
    class="provider-manager-dialog"
    :model-value="open"
    :title="`管理与查看 ${providerName} 的 API Key`"
    width="min(46rem, calc(100vw - 2rem))"
    :close-on-click-modal="false"
    destroy-on-close
    @update:model-value="handleModelValue"
  >
    <div class="dialog-content">
      <div class="viewer-toolbar">
        <p>打开管理器即显示全部密钥；关闭后会清空本次查看状态。</p>
        <ElButton
          native-type="button"
          :aria-label="hidden ? '显示全部 API Key' : '隐藏全部 API Key'"
          :disabled="loading"
          @click="hidden = !hidden"
        >
          {{ hidden ? '显示全部' : '隐藏全部' }}
        </ElButton>
      </div>

      <p v-if="apiKeyStatus === 'external'" class="external-message" role="note">
        当前 auth.json 使用外部密钥；保存本列表不会自动认领该值，请使用命名导入入口。
      </p>
      <p v-if="errorMessage" class="field-error" role="alert">{{ errorMessage }}</p>
      <p v-if="successMessage" class="copy-message" role="status">{{ successMessage }}</p>
      <p v-if="copyMessage" class="copy-message" role="status">{{ copyMessage }}</p>
      <p v-if="loading" class="loading-message">正在读取密钥…</p>

      <ol v-else class="draft-list">
        <li v-for="(entry, index) in entries" :key="entry.id ?? `new-${index}`" class="draft-row">
          <label class="field">
            <span>密钥名称</span>
            <ElInput
              :model-value="entry.name"
              :name="`api-key-name-${index}`"
              :disabled="busy"
              :aria-invalid="errors[index]?.name ? 'true' : undefined"
              autocomplete="off"
              @input="replaceEntry(index, 'name', String($event))"
            />
            <span v-if="errors[index]?.name" class="field-error" role="alert">
              {{ errors[index]?.name }}
            </span>
          </label>
          <label class="field key-field">
            <span>API Key</span>
            <ElInput
              :model-value="entry.apiKey"
              :name="`api-key-value-${index}`"
              :type="hidden ? 'password' : 'text'"
              :disabled="busy"
              :aria-invalid="errors[index]?.apiKey ? 'true' : undefined"
              autocomplete="off"
              spellcheck="false"
              @input="replaceEntry(index, 'apiKey', String($event))"
            />
            <span v-if="errors[index]?.apiKey" class="field-error" role="alert">
              {{ errors[index]?.apiKey }}
            </span>
          </label>
          <div class="row-actions">
            <span v-if="entry.id === selectedApiKeyId" class="current-badge">当前</span>
            <ElButton
              native-type="button"
              :aria-label="`复制 ${entry.name.trim() || index + 1}`"
              :disabled="busy || !entry.apiKey"
              @click="copyEntry(entry)"
            >
              复制
            </ElButton>
            <ElButton
              type="danger"
              plain
              native-type="button"
              :aria-label="`删除密钥 ${entry.name.trim() || index + 1}`"
              :disabled="busy || !canDelete(entry)"
              @click="removeEntry(index)"
            >
              删除
            </ElButton>
          </div>
        </li>
      </ol>

      <ElButton
        native-type="button"
        aria-label="新增 API Key"
        :disabled="loading || busy"
        @click="addEntry"
      >
        新增密钥
      </ElButton>
    </div>

    <template #footer>
      <div class="dialog-actions">
        <ElButton
          native-type="button"
          aria-label="关闭 API Key 管理器"
          :disabled="busy"
          @click="emit('close')"
        >
          取消
        </ElButton>
        <ElButton
          type="primary"
          native-type="button"
          aria-label="保存 API Key 列表"
          :loading="busy"
          :disabled="loading || hasErrors"
          @click="save"
        >
          保存
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.dialog-content,
.field,
.draft-list {
  display: grid;
  gap: 0.75rem;
}

.viewer-toolbar,
.row-actions,
.dialog-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.viewer-toolbar {
  justify-content: space-between;
}

.viewer-toolbar p,
.external-message,
.copy-message,
.loading-message {
  margin: 0;
}

.draft-list {
  max-height: min(52vh, 30rem);
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.draft-row {
  display: grid;
  grid-template-columns: minmax(9rem, 0.65fr) minmax(15rem, 1.4fr) auto;
  gap: 0.75rem;
  align-items: start;
  border: 1px solid var(--border);
  border-radius: 0.7rem;
  padding: 0.75rem;
}

.field {
  gap: 0.3rem;
  font-weight: 600;
}

.field-error {
  color: var(--danger);
  font-size: 0.8rem;
  font-weight: 400;
}

.copy-message {
  color: var(--success-text);
}

.external-message {
  color: var(--warning-text);
}

.row-actions {
  align-self: end;
}

.dialog-actions {
  justify-content: flex-end;
}

.current-badge {
  color: var(--accent-strong);
  font-size: 0.8rem;
  font-weight: 700;
}

@media (max-width: 760px) {
  .draft-row {
    grid-template-columns: 1fr;
  }

  .row-actions,
  .viewer-toolbar {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
