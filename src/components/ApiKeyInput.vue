<script setup lang="ts">
import { shallowRef } from 'vue'

defineProps<{
  configured: boolean
  disabled?: boolean
  invalid?: boolean
  describedBy?: string
}>()

const model = defineModel<string>({ required: true })
const emit = defineEmits<{
  clear: []
}>()
const visible = shallowRef(false)
const confirmingClear = shallowRef(false)

function confirmClear() {
  model.value = ''
  confirmingClear.value = false
  emit('clear')
}
</script>

<template>
  <div class="api-key-field">
    <label class="field-label" for="provider-api-key">API Key</label>
    <div class="input-row">
      <input
        id="provider-api-key"
        v-model="model"
        class="text-input"
        :type="visible ? 'text' : 'password'"
        :disabled="disabled"
        :placeholder="configured ? '已配置，留空保持不变' : '输入 API Key'"
        :aria-invalid="invalid ? 'true' : undefined"
        :aria-describedby="describedBy"
        autocomplete="off"
        spellcheck="false"
      />
      <button
        type="button"
        :aria-label="visible ? '隐藏 API Key' : '显示 API Key'"
        :disabled="disabled"
        @click="visible = !visible"
      >
        {{ visible ? '隐藏' : '显示' }}
      </button>
    </div>
    <p class="field-help">
      {{ configured ? '未输入新值时不会覆盖现有密钥。' : '密钥仅保存在本机明文配置文件中。' }}
    </p>
    <button
      v-if="configured && !confirmingClear"
      type="button"
      class="danger-link"
      aria-label="清空 API Key"
      :disabled="disabled"
      @click="confirmingClear = true"
    >
      清空密钥
    </button>
    <div v-if="confirmingClear" class="clear-confirmation" role="alert">
      <span>确认清空已保存的 API Key？</span>
      <button type="button" aria-label="确认清空 API Key" @click="confirmClear">确认清空</button>
      <button type="button" aria-label="取消清空 API Key" @click="confirmingClear = false">
        取消
      </button>
    </div>
  </div>
</template>

<style scoped>
.api-key-field {
  display: grid;
  gap: 0.45rem;
}

.input-row,
.clear-confirmation {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.text-input {
  min-width: 0;
  flex: 1;
}

.field-label {
  font-weight: 700;
}

.field-help {
  margin: 0;
  color: #667085;
  font-size: 0.82rem;
}

.danger-link {
  width: fit-content;
  color: #b42318;
}

.clear-confirmation {
  flex-wrap: wrap;
  color: #b42318;
}
</style>
