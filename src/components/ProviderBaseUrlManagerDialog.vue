<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import { ElButton, ElDialog, ElInput } from 'element-plus'
import type { ProviderBaseUrlDraft, ProviderBaseUrlSummary } from '../types/provider'

interface DraftRow extends ProviderBaseUrlDraft {
  draftId: string
}

const props = defineProps<{
  open: boolean
  providerName: string
  entries: readonly ProviderBaseUrlSummary[]
  selectedBaseUrlId: string | null
  externalUrl: string | null
  busy: boolean
}>()

const emit = defineEmits<{
  save: [entries: ProviderBaseUrlDraft[]]
  close: []
}>()

const drafts = shallowRef<DraftRow[]>([])
let nextDraftId = 0

function newDraftId() {
  nextDraftId += 1
  return `new-${nextDraftId}`
}

function resetDrafts() {
  nextDraftId = 0
  drafts.value = props.entries.map((entry) => ({ ...entry, draftId: `saved-${entry.id}` }))
}

watch(
  () => props.open,
  (open) => {
    if (open) resetDrafts()
    else drafts.value = []
  },
  { immediate: true },
)

function normalizedUrl(value: string): string | null {
  try {
    const parsed = new URL(value.trim())
    if (!['http:', 'https:'].includes(parsed.protocol)) return null
    return parsed.toString()
  } catch {
    return null
  }
}

const errors = computed(() => {
  const next = drafts.value.map(() => ({ name: '', url: '' }))
  const nameCounts = new Map<string, number>()
  const urlCounts = new Map<string, number>()

  for (const row of drafts.value) {
    const name = row.name.trim().toLocaleLowerCase()
    if (name) nameCounts.set(name, (nameCounts.get(name) ?? 0) + 1)
    const url = normalizedUrl(row.url)
    if (url) urlCounts.set(url, (urlCounts.get(url) ?? 0) + 1)
  }

  drafts.value.forEach((row, index) => {
    const name = row.name.trim()
    if (!name) next[index]!.name = '地址名称为必填项。'
    else if ((nameCounts.get(name.toLocaleLowerCase()) ?? 0) > 1) {
      next[index]!.name = '地址名称不能重复。'
    }

    const rawUrl = row.url.trim()
    const url = normalizedUrl(rawUrl)
    if (!rawUrl) next[index]!.url = 'Base URL 为必填项。'
    else if (!url) next[index]!.url = 'Base URL 必须是有效的 HTTP(S) 地址。'
    else if ((urlCounts.get(url) ?? 0) > 1) next[index]!.url = 'Base URL 不能重复。'
  })

  return next
})
const hasErrors = computed(() =>
  drafts.value.length === 0 || errors.value.some((row) => row.name || row.url),
)
const externalAlreadyAdded = computed(() => {
  const external = props.externalUrl ? normalizedUrl(props.externalUrl) : null
  return Boolean(external && drafts.value.some((row) => normalizedUrl(row.url) === external))
})

function updateDraft(index: number, field: 'name' | 'url', value: string) {
  drafts.value = drafts.value.map((row, rowIndex) =>
    rowIndex === index ? { ...row, [field]: value } : row,
  )
}

function addDraft(url = '') {
  drafts.value = [
    ...drafts.value,
    { id: null, name: '', url, draftId: newDraftId() },
  ]
}

function canDelete(row: DraftRow) {
  return drafts.value.length > 1 && row.id !== props.selectedBaseUrlId
}

function removeDraft(index: number) {
  const row = drafts.value[index]
  if (!row || !canDelete(row)) return
  drafts.value = drafts.value.filter((_, rowIndex) => rowIndex !== index)
}

function save() {
  if (props.busy || hasErrors.value) return
  emit('save', drafts.value.map(({ id, name, url }) => ({
    id,
    name: name.trim(),
    url: url.trim(),
  })))
}

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('close')
}
</script>

<template>
  <ElDialog
    class="provider-manager-dialog"
    :model-value="open"
    :title="`管理 ${providerName} 的 Base URL`"
    width="min(44rem, calc(100vw - 2rem))"
    :close-on-click-modal="false"
    destroy-on-close
    @update:model-value="handleModelValue"
  >
    <div class="dialog-content">
      <aside v-if="externalUrl && !externalAlreadyAdded" class="external-callout">
        <p>当前外部地址：<code>{{ externalUrl }}</code></p>
        <ElButton
          native-type="button"
          aria-label="保存当前外部地址为命名项"
          :disabled="busy"
          @click="addDraft(externalUrl)"
        >
          保存为命名地址
        </ElButton>
      </aside>

      <ol class="draft-list">
        <li v-for="(row, index) in drafts" :key="row.draftId" class="draft-row">
          <label class="field">
            <span>地址名称</span>
            <ElInput
              :model-value="row.name"
              :name="`base-url-name-${index}`"
              :disabled="busy"
              :aria-invalid="errors[index]?.name ? 'true' : undefined"
              @input="updateDraft(index, 'name', String($event))"
            />
            <span v-if="errors[index]?.name" class="field-error" role="alert">
              {{ errors[index]?.name }}
            </span>
          </label>
          <label class="field url-field">
            <span>Base URL</span>
            <ElInput
              :model-value="row.url"
              :name="`base-url-value-${index}`"
              type="url"
              :disabled="busy"
              :aria-invalid="errors[index]?.url ? 'true' : undefined"
              @input="updateDraft(index, 'url', String($event))"
            />
            <span v-if="errors[index]?.url" class="field-error" role="alert">
              {{ errors[index]?.url }}
            </span>
          </label>
          <div class="row-actions">
            <span v-if="row.id === selectedBaseUrlId" class="current-badge">当前</span>
            <ElButton
              type="danger"
              plain
              native-type="button"
              :aria-label="`删除地址 ${row.name.trim() || index + 1}`"
              :disabled="busy || !canDelete(row)"
              @click="removeDraft(index)"
            >
              删除
            </ElButton>
          </div>
        </li>
      </ol>

      <ElButton
        native-type="button"
        aria-label="新增 Base URL"
        :disabled="busy"
        @click="addDraft()"
      >
        新增地址
      </ElButton>
    </div>

    <template #footer>
      <div class="dialog-actions">
        <ElButton native-type="button" :disabled="busy" @click="emit('close')">取消</ElButton>
        <ElButton
          type="primary"
          native-type="button"
          aria-label="保存 Base URL 列表"
          :loading="busy"
          :disabled="hasErrors"
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

.draft-list {
  max-height: min(52vh, 30rem);
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.draft-row {
  display: grid;
  grid-template-columns: minmax(9rem, 0.7fr) minmax(14rem, 1.4fr) auto;
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

.row-actions,
.dialog-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
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

.external-callout {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  border: 1px solid var(--warning-border);
  border-radius: 0.7rem;
  padding: 0.75rem;
  background: var(--warning-soft);
}

.external-callout p {
  margin: 0;
  overflow-wrap: anywhere;
}

@media (max-width: 760px) {
  .draft-row {
    grid-template-columns: 1fr;
  }

  .row-actions {
    justify-content: space-between;
  }

  .external-callout {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
