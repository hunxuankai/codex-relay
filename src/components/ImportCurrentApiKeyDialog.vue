<script setup lang="ts">
import { nextTick, shallowRef, useTemplateRef, watch } from 'vue'
import { ElButton, ElDialog, ElInput } from 'element-plus'
import type { InputInstance } from 'element-plus'

const props = defineProps<{
  open: boolean
  providerName: string
  busy: boolean
}>()

const emit = defineEmits<{
  import: [name: string]
  close: []
}>()

const name = shallowRef('')
const error = shallowRef('')
const input = useTemplateRef<InputInstance>('input')
const previousFocus = shallowRef<HTMLElement | null>(null)

function scheduleFocus() {
  setTimeout(() => input.value?.focus(), 0)
}

watch(
  () => props.open,
  async (open) => {
    name.value = ''
    error.value = ''
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
      await nextTick()
      scheduleFocus()
      return
    }
    await nextTick()
    previousFocus.value?.focus()
    previousFocus.value = null
  },
  { immediate: true },
)

function submit() {
  const normalizedName = name.value.trim()
  if (!normalizedName) {
    error.value = '密钥名称为必填项。'
    input.value?.focus()
    return
  }
  error.value = ''
  emit('import', normalizedName)
}

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('close')
}
</script>

<template>
  <ElDialog
    class="import-key-dialog"
    :model-value="open"
    :title="`导入 ${providerName} 的当前密钥`"
    width="min(30rem, calc(100vw - 2rem))"
    :show-close="false"
    :close-on-click-modal="false"
    destroy-on-close
    @update:model-value="handleModelValue"
    @opened="scheduleFocus"
    @open-auto-focus="scheduleFocus"
  >
    <div class="dialog-content">
      <p>
        为 auth.json 中当前生效但尚未纳管的密钥填写名称。密钥仍会以明文保存在本机
        providers.json 中。
      </p>
      <label class="field">
        <span>密钥名称</span>
        <ElInput
          ref="input"
          v-model="name"
          name="import-api-key-name"
          :disabled="busy"
          :aria-invalid="error ? 'true' : undefined"
          :aria-describedby="error ? 'import-api-key-name-error' : undefined"
          autocomplete="off"
          @keyup.enter="submit"
        />
        <span v-if="error" id="import-api-key-name-error" class="field-error" role="alert">
          {{ error }}
        </span>
      </label>
    </div>

    <template #footer>
      <div class="dialog-actions">
        <ElButton
          native-type="button"
          aria-label="取消导入当前密钥"
          :disabled="busy"
          @click="emit('close')"
        >
          取消
        </ElButton>
        <ElButton
          type="primary"
          native-type="button"
          aria-label="确认导入当前密钥"
          :loading="busy"
          @click="submit"
        >
          导入
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.dialog-content,
.field {
  display: grid;
  gap: 0.75rem;
}

.dialog-content p {
  margin: 0;
  line-height: 1.6;
}

.field {
  gap: 0.35rem;
  font-weight: 600;
}

.field-error {
  color: var(--danger);
  font-size: 0.82rem;
  font-weight: 400;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
