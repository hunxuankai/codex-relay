<script setup lang="ts">
import { shallowRef } from 'vue'
import { ElButton, ElInput } from 'element-plus'

defineProps<{
  disabled?: boolean
  invalid?: boolean
  describedBy?: string
}>()

const model = defineModel<string>({ required: true })
const visible = shallowRef(false)
</script>

<template>
  <div class="api-key-field">
    <label class="field-label" for="provider-api-key">API Key</label>
    <div class="input-row">
      <ElInput
        id="provider-api-key"
        v-model="model"
        class="text-input"
        :type="visible ? 'text' : 'password'"
        :disabled="disabled"
        placeholder="输入 API Key"
        :aria-invalid="invalid ? 'true' : undefined"
        :aria-describedby="describedBy"
        autocomplete="off"
        spellcheck="false"
      />
      <ElButton
        native-type="button"
        :aria-label="visible ? '隐藏 API Key' : '显示 API Key'"
        :disabled="disabled"
        @click="visible = !visible"
      >
        {{ visible ? '隐藏' : '显示' }}
      </ElButton>
    </div>
    <p class="field-help">密钥仅保存在本机明文配置文件中。</p>
  </div>
</template>

<style scoped>
.api-key-field {
  display: grid;
  gap: 0.45rem;
}

.input-row {
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
  color: var(--text-secondary);
  font-size: 0.82rem;
}

</style>
