<script setup lang="ts">
import { nextTick, watch, useTemplateRef } from 'vue'
import { ElButton, ElDialog } from 'element-plus'
import type { DraftIdentity } from '../../types/release'

defineProps<{
  identity: DraftIdentity
  busy: boolean
}>()

const emit = defineEmits<{
  confirm: []
}>()

const model = defineModel<boolean>({ required: true })
const cancelButton = useTemplateRef<InstanceType<typeof ElButton>>('cancelButton')

async function focusCancel() {
  await nextTick()
  const element = cancelButton.value?.$el as HTMLElement | undefined
  element?.focus()
}

watch(model, (open) => {
  if (open) void focusCancel()
})
</script>

<template>
  <ElDialog
    v-model="model"
    title="确认正式公开 Release"
    width="min(34rem, calc(100vw - 2rem))"
    :close-on-click-modal="false"
    :close-on-press-escape="!busy"
    :show-close="!busy"
    destroy-on-close
    @opened="focusCancel"
  >
    <div class="confirm-content">
      <p class="warning-title">此操作不可撤销。</p>
      <p>
        控制台会重新审计同一 Draft，然后按 Release ID
        <strong>{{ identity.releaseId }}</strong> 公开 <strong>{{ identity.tagName }}</strong>。
      </p>
      <dl class="identity-list">
        <div>
          <dt>Release ID</dt>
          <dd>{{ identity.releaseId }}</dd>
        </div>
        <div>
          <dt>Tag</dt>
          <dd>{{ identity.tagName }}</dd>
        </div>
        <div>
          <dt>候选提交</dt>
          <dd class="mono">{{ identity.targetCommitSha }}</dd>
        </div>
      </dl>
      <p class="scope-note">
        本控制台不会验证 Sandbox、真实安装、UAC 或应用内升级，这些事项不会出现在成功声明中。
      </p>
    </div>

    <template #footer>
      <div class="dialog-actions">
        <ElButton ref="cancelButton" :disabled="busy" @click="model = false">取消</ElButton>
        <ElButton
          data-testid="confirm-publish-button"
          type="danger"
          :loading="busy"
          @click="emit('confirm')"
        >
          重新审计并正式公开
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.confirm-content,
.identity-list {
  display: grid;
  gap: 0.85rem;
}

.confirm-content p,
.identity-list,
.identity-list dt,
.identity-list dd {
  margin: 0;
}

.warning-title {
  color: var(--danger-color);
  font-weight: 800;
}

.identity-list {
  padding: 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  background: var(--surface-muted);
}

.identity-list div {
  display: grid;
  grid-template-columns: 7rem minmax(0, 1fr);
  gap: 0.75rem;
}

.identity-list dt,
.scope-note {
  color: var(--text-muted);
}

.identity-list dd {
  overflow-wrap: anywhere;
  font-weight: 700;
}

.mono {
  font-family: var(--font-mono);
  font-size: 0.78rem;
}

.scope-note {
  font-size: 0.82rem;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.65rem;
}
</style>
