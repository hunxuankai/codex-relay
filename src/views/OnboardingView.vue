<script setup lang="ts">
import { shallowRef } from 'vue'
import AppNotification from '../components/AppNotification.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'

defineProps<{
  busy: boolean
  currentProviderName: string | null
  canImportCurrentKey: boolean
  successMessage: string | null
  errorMessage: string | null
}>()

const emit = defineEmits<{
  openDirectory: []
  addProvider: []
  later: []
  exit: []
  importCurrentKey: []
}>()

const confirmImport = shallowRef(false)

function importCurrentKey() {
  confirmImport.value = false
  emit('importCurrentKey')
}
</script>

<template>
  <main class="onboarding-view">
    <section class="onboarding-card" aria-label="首次设置">
      <p class="eyebrow">Codex Relay</p>
      <h1>首次设置</h1>
      <p>尚未检测到可用的 Codex Provider 配置。你可以现在新增，也可以稍后再设置。</p>

      <AppNotification :message="successMessage" level="success" />
      <AppNotification :message="errorMessage" level="error" />

      <aside v-if="canImportCurrentKey && currentProviderName" class="import-callout">
        <p>检测到当前 Codex 配置中存在 API Key，是否将其保存到当前 Provider「{{ currentProviderName }}」？</p>
        <button
          type="button"
          aria-label="保存当前 auth.json 密钥"
          :disabled="busy"
          @click="confirmImport = true"
        >
          保存到当前 Provider
        </button>
      </aside>

      <div class="onboarding-actions">
        <button
          type="button"
          data-onboarding-action
          aria-label="打开 Codex 配置目录"
          :disabled="busy"
          @click="emit('openDirectory')"
        >
          打开 Codex 配置目录
        </button>
        <button
          type="button"
          data-onboarding-action
          aria-label="新增第一个 Provider"
          :disabled="busy"
          @click="emit('addProvider')"
        >
          新增第一个 Provider
        </button>
        <button
          type="button"
          data-onboarding-action
          aria-label="稍后设置"
          :disabled="busy"
          @click="emit('later')"
        >
          稍后设置
        </button>
        <button
          type="button"
          data-onboarding-action
          aria-label="退出"
          :disabled="busy"
          @click="emit('exit')"
        >
          退出
        </button>
      </div>
    </section>

    <ConfirmDialog
      :open="confirmImport"
      title="确认保存当前密钥"
      :message="`检测到当前 Codex 配置中存在 API Key，是否将其保存到当前 Provider「${currentProviderName ?? ''}」？密钥会以明文写入本机 providers.json。`"
      confirm-label="确认保存"
      @confirm="importCurrentKey"
      @cancel="confirmImport = false"
    />
  </main>
</template>

<style scoped>
.onboarding-view {
  display: grid;
  min-height: 100vh;
  place-items: center;
  padding: 1.5rem;
}

.onboarding-card,
.onboarding-actions {
  display: grid;
  gap: 1rem;
}

.onboarding-card {
  width: min(100%, 38rem);
  border: 1px solid var(--border);
  border-radius: 1rem;
  padding: 1.5rem;
  background: var(--surface);
  box-shadow: var(--shadow);
}

.onboarding-card h1,
.onboarding-card p,
.eyebrow {
  margin: 0;
}

.eyebrow {
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.import-callout {
  display: grid;
  gap: 0.75rem;
  border: 1px solid var(--warning-border);
  border-radius: 0.8rem;
  padding: 1rem;
  background: var(--warning-soft);
}
</style>
