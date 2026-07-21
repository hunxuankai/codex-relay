<script setup lang="ts">
import { reactive, watch } from 'vue'
import AppNotification from '../components/AppNotification.vue'
import UpdatePanel from '../components/UpdatePanel.vue'
import { useSettings } from '../composables/useSettings'

const settingsState = useSettings()
const draft = reactive({
  trayOnlyOnAutostart: true,
  closeToTray: true,
  showWindowOnManualStart: true,
})

watch(
  settingsState.settings,
  (settings) => {
    if (!settings) return
    draft.trayOnlyOnAutostart = settings.trayOnlyOnAutostart
    draft.closeToTray = settings.closeToTray
    draft.showWindowOnManualStart = settings.showWindowOnManualStart
  },
  { immediate: true },
)

function toggleAutostart(event: Event) {
  settingsState.setAutostart((event.target as HTMLInputElement).checked)
}

function save() {
  const current = settingsState.settings.value
  if (!current) return
  settingsState.save({
    ...current,
    trayOnlyOnAutostart: draft.trayOnlyOnAutostart,
    closeToTray: draft.closeToTray,
    showWindowOnManualStart: draft.showWindowOnManualStart,
  })
}
</script>

<template>
  <main class="settings-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Settings</p>
        <h1>应用设置</h1>
      </div>
      <button type="button" :disabled="settingsState.loading.value" @click="settingsState.refresh">
        刷新状态
      </button>
    </header>

    <AppNotification :message="settingsState.successMessage.value" level="success" />
    <AppNotification :message="settingsState.error.value?.message ?? null" level="error" />

    <p v-if="settingsState.loading.value && !settingsState.settings.value">正在加载设置…</p>
    <form v-else-if="settingsState.settings.value" class="settings-form" @submit.prevent="save">
      <section class="settings-section">
        <h2>开机启动</h2>
        <label class="setting-row">
          <span>
            <strong>登录 Windows 后自动启动</strong>
            <small>
              Windows 实际状态：{{ settingsState.autostart.value?.actualEnabled ? '已启用' : '未启用' }}
            </small>
          </span>
          <input
            type="checkbox"
            aria-label="登录 Windows 后自动启动"
            :checked="settingsState.autostart.value?.actualEnabled ?? false"
            :disabled="settingsState.busy.value"
            @change="toggleAutostart"
          />
        </label>
        <p v-if="settingsState.autostart.value && !settingsState.autostart.value.isConsistent" class="warning" role="status">
          设置与 Windows 实际状态不一致，请重新切换或刷新后重试。
        </p>
      </section>

      <section class="settings-section">
        <h2>窗口与托盘</h2>
        <label class="setting-row">
          <span>开机自动启动时仅显示托盘</span>
          <input
            v-model="draft.trayOnlyOnAutostart"
            name="tray-only-on-autostart"
            type="checkbox"
          />
        </label>
        <label class="setting-row">
          <span>关闭窗口时隐藏到托盘</span>
          <input v-model="draft.closeToTray" name="close-to-tray" type="checkbox" />
        </label>
        <label class="setting-row">
          <span>手动启动时显示主窗口</span>
          <input
            v-model="draft.showWindowOnManualStart"
            name="show-window-on-manual-start"
            type="checkbox"
          />
        </label>
      </section>

      <div class="settings-actions">
        <button type="submit" :disabled="settingsState.busy.value">保存设置</button>
        <button type="button" :disabled="settingsState.busy.value" @click="settingsState.openDirectory">
          打开 Codex 配置目录
        </button>
      </div>
    </form>

    <UpdatePanel />
  </main>
</template>

<style scoped>
.settings-view,
.settings-form,
.settings-section {
  display: grid;
  gap: 1rem;
}

.settings-view {
  padding: 1.25rem;
}

.view-header,
.setting-row,
.settings-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.view-header h1,
.eyebrow,
.settings-section h2,
.warning {
  margin: 0;
}

.eyebrow {
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.settings-section {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  padding: 1rem;
}

.setting-row span {
  display: grid;
  gap: 0.25rem;
}

.setting-row small {
  color: var(--text-secondary);
}

.warning {
  color: var(--warning);
}

.settings-actions {
  justify-content: flex-start;
}
</style>
