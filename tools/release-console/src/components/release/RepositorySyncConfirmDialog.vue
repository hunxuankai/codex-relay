<script setup lang="ts">
import { computed, nextTick, useTemplateRef, watch } from 'vue'
import { ElButton, ElDialog } from 'element-plus'
import type { SafeRepositoryPushPreview } from '../../types/release'

const props = defineProps<{
  remoteUrl: string
  preview: SafeRepositoryPushPreview
  busy: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

const model = defineModel<boolean>({ required: true })
const cancelButton = useTemplateRef<InstanceType<typeof ElButton>>('cancelButton')
const repositoryName = computed(() =>
  props.remoteUrl.replace(/^.*github\.com[/:]/, '').replace(/\.git$/, ''),
)

async function focusCancel() {
  await nextTick()
  const element = cancelButton.value?.$el as HTMLElement | undefined
  element?.focus()
}

function cancel() {
  model.value = false
  emit('cancel')
}

watch(model, (open) => {
  if (open) void focusCancel()
})
</script>

<template>
  <ElDialog
    v-model="model"
    title="确认推送本地提交"
    width="min(38rem, calc(100vw - 2rem))"
    :close-on-click-modal="false"
    :close-on-press-escape="!busy"
    :show-close="!busy"
    destroy-on-close
    @opened="focusCancel"
  >
    <div class="confirm-content">
      <p class="warning-title">仅同步当前已审核的本地提交。</p>
      <dl class="identity-list">
        <div>
          <dt>远端</dt>
          <dd>{{ repositoryName }}</dd>
        </div>
        <div>
          <dt>本地 HEAD</dt>
          <dd class="mono">{{ preview.expectedHeadSha.slice(0, 12) }}</dd>
        </div>
        <div>
          <dt>远端 main</dt>
          <dd class="mono">{{ preview.expectedRemoteMainSha.slice(0, 12) }}</dd>
        </div>
        <div>
          <dt>提交数量</dt>
          <dd>{{ preview.commitCount }} 个提交</dd>
        </div>
      </dl>

      <ol class="commit-list" aria-label="待推送提交">
        <li v-for="commit in preview.commits" :key="commit.sha">
          <code>{{ commit.sha.slice(0, 12) }}</code>
          <span>{{ commit.subject }}</span>
        </li>
      </ol>

      <p class="scope-note">
        后端会重新 Fetch 和复核两端 SHA，只推送确认的 SHA 到远端 main；不会推送 Tag 或其他分支。
      </p>
    </div>

    <template #footer>
      <div class="dialog-actions">
        <ElButton ref="cancelButton" :disabled="busy" @click="cancel">取消</ElButton>
        <ElButton
          data-testid="confirm-repository-push-button"
          type="primary"
          :loading="busy"
          @click="emit('confirm')"
        >
          确认并推送
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.confirm-content,
.identity-list,
.commit-list {
  display: grid;
  gap: 0.8rem;
}

.confirm-content p,
.identity-list,
.identity-list dt,
.identity-list dd,
.commit-list {
  margin: 0;
}

.warning-title {
  font-weight: 800;
}

.identity-list,
.commit-list {
  padding: 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  background: var(--surface-muted);
}

.identity-list div,
.commit-list li {
  display: grid;
  grid-template-columns: 7rem minmax(0, 1fr);
  gap: 0.75rem;
}

.identity-list dt,
.scope-note {
  color: var(--text-muted);
}

.identity-list dd,
.commit-list span {
  overflow-wrap: anywhere;
  font-weight: 700;
}

.mono,
.commit-list code {
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
