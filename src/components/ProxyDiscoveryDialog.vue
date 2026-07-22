<script setup lang="ts">
import { ElButton, ElDialog, ElEmpty, ElRadio, ElRadioGroup } from 'element-plus'

const props = defineProps<{
  open: boolean
  candidates: readonly string[]
  selected: string | null
}>()

const emit = defineEmits<{
  select: [proxy: string]
  confirm: []
  cancel: []
}>()

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('cancel')
}
</script>

<template>
  <ElDialog
    class="proxy-dialog"
    :model-value="open"
    title="本机代理检测结果"
    width="min(32rem, calc(100vw - 2rem))"
    :show-close="false"
    :close-on-click-modal="false"
    @update:model-value="handleModelValue"
  >
    <fieldset v-if="candidates.length" class="proxy-options">
      <legend>选择要启用的代理</legend>
      <ElRadioGroup
        :model-value="selected ?? undefined"
        aria-label="选择要启用的代理"
        @change="emit('select', String($event))"
      >
        <ElRadio v-for="candidate in candidates" :key="candidate" :value="candidate" border>
          <code>{{ candidate }}</code>
        </ElRadio>
      </ElRadioGroup>
    </fieldset>
    <ElEmpty v-else description="未检测到可用于访问更新源的本机代理。" />
    <template #footer>
      <div class="dialog-actions">
        <ElButton native-type="button" @click="emit('cancel')">关闭</ElButton>
        <ElButton
          v-if="candidates.length"
          data-action="apply-proxy"
          type="primary"
          native-type="button"
          :disabled="!selected"
          @click="emit('confirm')"
        >
          一键填入并启用
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.proxy-options {
  display: grid;
  gap: 0.75rem;
  border: 0;
  padding: 0;
}

.proxy-options :deep(.el-radio-group) {
  display: grid;
  gap: 0.75rem;
}

.proxy-options :deep(.el-radio) {
  width: 100%;
  margin-right: 0;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
