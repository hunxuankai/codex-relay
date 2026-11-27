<script setup lang="ts">
import { ElButton, ElDialog } from 'element-plus'
import { onBeforeUnmount, shallowRef, useId, useTemplateRef, watch } from 'vue'
import type { ProfileKeyEdit } from '../types/cliProfile'

const props = defineProps<{
  edit: Readonly<ProfileKeyEdit> | null
  saving: boolean
  save: (newKey: string) => Promise<void>
  applyProviderKey: (apiKeyId: string) => Promise<void>
}>()

const emit = defineEmits<{ cancel: [] }>()
const keyInput = useTemplateRef<HTMLInputElement>('keyInput')
const providerKeyOptions = useTemplateRef<HTMLFieldSetElement>('providerKeyOptions')
const inputId = useId()
const noteId = useId()
const providerKeysHelpId = useId()
const hasDraft = shallowRef(false)

function clearDraft() {
  if (keyInput.value) keyInput.value.value = ''
  hasDraft.value = false
}

watch(() => props.edit?.token, clearDraft)
onBeforeUnmount(clearDraft)

function submit() {
  if (!props.edit || props.saving || !keyInput.value?.value) return
  const key = keyInput.value.value
  clearDraft()
  void props.save(key)
}

function applySavedKey(apiKeyId: string) {
  if (!props.edit || props.saving) return
  clearDraft()
  void props.applyProviderKey(apiKeyId)
}

function cancel() {
  if (!props.edit || props.saving) return
  clearDraft()
  emit('cancel')
}

function focusEditor() {
  (providerKeyOptions.value ?? keyInput.value)?.focus()
}
</script>

<template>
  <ElDialog
    :model-value="Boolean(edit)"
    :title="`${edit?.replacesValue ? '更换' : '设置'} Profile Key`"
    width="min(31rem, calc(100vw - 2rem))"
    top="min(8vh, 3rem)"
    :close-on-click-modal="false"
    :close-on-press-escape="!saving"
    :show-close="false"
    destroy-on-close
    :aria-describedby="noteId"
    @opened="focusEditor"
    @close="cancel"
  >
    <form v-if="edit" class="key-form" aria-label="设置 Profile Key" :aria-busy="saving" @submit.prevent="submit">
      <dl class="key-target">
        <dt>Profile / 供应商</dt>
        <dd>{{ edit.profileName }} / {{ edit.providerId }}<template v-if="edit.host"> · {{ edit.host }}</template></dd>
        <dt>目标变量</dt>
        <dd><code>{{ edit.variable }}</code></dd>
      </dl>
      <p :id="noteId" class="key-note">
        仅写入 Windows 当前用户变量，不改 Codex 配置。用户环境变量不是加密密钥库。
      </p>
      <p v-if="!edit.replacesValue && edit.relayProcessHasValue" class="key-note">
        将创建用户变量，不会导入当前 Relay 进程中的值。
      </p>
      <fieldset
        v-if="edit.providerKeys.length"
        ref="providerKeyOptions"
        class="provider-key-options"
        tabindex="-1"
        :aria-describedby="providerKeysHelpId"
      >
        <legend>使用供应商已保存的 Key</legend>
        <p :id="providerKeysHelpId" class="key-note">点击名称，立即保存到当前 Profile 的目标变量。</p>
        <div class="provider-key-list">
          <ElButton
            v-for="entry in edit.providerKeys"
            :key="entry.id"
            class="provider-key-button"
            native-type="button"
            :aria-label="`使用已保存的 Key：${entry.name}`"
            :disabled="saving"
            @click="applySavedKey(entry.id)"
          >{{ entry.name }}</ElButton>
        </div>
      </fieldset>
      <p v-else class="key-note">没有可复用的同供应商 Key，可手动输入。</p>
      <label :for="inputId" class="key-label">手动输入新 Key</label>
      <input
        :id="inputId"
        ref="keyInput"
        class="profile-key-field"
        type="password"
        name="profile-new-key"
        autocomplete="off"
        :spellcheck="false"
        maxlength="4096"
        :disabled="saving"
        @input="hasDraft = Boolean(keyInput?.value)"
      />
      <div class="key-form-actions">
        <ElButton native-type="button" aria-label="取消编辑 Key" :disabled="saving" @click="cancel">取消</ElButton>
        <ElButton type="primary" native-type="submit" :loading="saving" :disabled="!hasDraft">
          保存到 Windows 用户变量
        </ElButton>
      </div>
    </form>
  </ElDialog>
</template>

<style scoped>
.key-form {
  display: grid;
  gap: 0.65rem;
  overflow-wrap: anywhere;
}

.key-target {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: 0.4rem 0.75rem;
  margin: 0;
}

.key-target dd,
.key-note {
  margin: 0;
}

.key-label {
  color: var(--text-primary);
  font-weight: 600;
}

.provider-key-options {
  display: grid;
  gap: 0.4rem;
  min-inline-size: 0;
  margin: 0;
  padding: 0.45rem 0.65rem;
  border: 1px solid var(--border);
  border-radius: var(--el-border-radius-base);
}

.provider-key-options legend {
  padding-inline: 0.25rem;
  color: var(--text-primary);
  font-weight: 600;
}

.provider-key-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  max-block-size: min(8rem, 25vh);
  padding: 0.15rem;
  overflow: auto;
}

.provider-key-button {
  max-inline-size: 100%;
  min-block-size: 36px;
  height: auto;
  white-space: normal;
  text-align: start;
}

.provider-key-list :deep(.el-button + .el-button) {
  margin-left: 0;
}

.profile-key-field {
  box-sizing: border-box;
  width: 100%;
  min-height: 36px;
  padding: 0.35rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: var(--el-border-radius-base);
  color: var(--text-primary);
  background: var(--surface);
  font: inherit;
}

.profile-key-field:focus-visible,
.provider-key-options:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.key-form-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.65rem;
  margin-top: 0.4rem;
}

.key-form-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}
</style>
