<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, shallowRef } from 'vue'
import AppNotification from './components/AppNotification.vue'
import HealthStatus from './components/HealthStatus.vue'
import { useHealth } from './composables/useHealth'
import { useProviders } from './composables/useProviders'
import { useSettings } from './composables/useSettings'
import * as relay from './services/tauri'
import BackupsView from './views/BackupsView.vue'
import OnboardingView from './views/OnboardingView.vue'
import ProvidersView from './views/ProvidersView.vue'
import SettingsView from './views/SettingsView.vue'

type AppView = 'providers' | 'health' | 'backups' | 'settings'

const providerState = useProviders()
const healthState = useHealth()
const settingsState = useSettings()
const activeView = shallowRef<AppView>('providers')
const onboardingDismissed = shallowRef(false)
const startCreatingProvider = shallowRef(false)
const pendingFirstProvider = shallowRef(false)
const lastOperation = shallowRef<string | null>(null)
const appMessage = shallowRef<{ level: 'success' | 'error'; message: string } | null>(null)

const configMissing = computed(() => {
  const check = healthState.report.value?.checks.find((item) => item.id === 'config-file')
  return Boolean(check && check.level !== 'normal')
})
const startupLoading = computed(
  () => providerState.loading.value || healthState.loading.value || settingsState.loading.value,
)
const showOnboarding = computed(() => {
  const settings = settingsState.settings.value
  if (startupLoading.value || !settings || onboardingDismissed.value) return false
  return !settings.firstRunCompleted &&
    (providerState.providers.value.length === 0 || configMissing.value)
})
const healthLabel = computed(() => {
  if (healthState.report.value?.level === 'normal') return '正常'
  if (healthState.report.value?.level === 'warning') return '警告'
  if (healthState.report.value?.level === 'error') return '错误'
  return '检查中'
})
const operationText = computed(
  () =>
    lastOperation.value ??
    appMessage.value?.message ??
    providerState.successMessage.value ??
    settingsState.successMessage.value ??
    providerState.error.value?.message ??
    settingsState.error.value?.message ??
    healthState.error.value?.message ??
    '暂无操作',
)

async function completeOnboarding() {
  const settings = settingsState.settings.value
  if (!settings) return false
  await settingsState.save({ ...settings, firstRunCompleted: true })
  if (settingsState.error.value) return false
  onboardingDismissed.value = true
  return true
}

async function addFirstProvider() {
  onboardingDismissed.value = true
  pendingFirstProvider.value = true
  startCreatingProvider.value = true
  activeView.value = 'providers'
}

async function configureLater() {
  if (!(await completeOnboarding())) return
  activeView.value = 'providers'
}

async function importCurrentKey() {
  const providerId = providerState.activeProvider.value?.id
  if (providerId) await providerState.importCurrentKey(providerId)
}

async function handleProviderCreated() {
  if (!pendingFirstProvider.value) return
  pendingFirstProvider.value = false
  startCreatingProvider.value = false
  await completeOnboarding()
}

function handleCreateCancelled() {
  if (!pendingFirstProvider.value) return
  pendingFirstProvider.value = false
  startCreatingProvider.value = false
  onboardingDismissed.value = false
}

async function exitApplication() {
  try {
    await relay.exitApplication()
  } catch {
    appMessage.value = { level: 'error', message: '无法退出应用，请使用托盘菜单中的“退出”。' }
  }
}

async function handleBackupRestored() {
  const [providersRefreshed] = await Promise.all([
    providerState.refresh(),
    healthState.runExtended(),
  ])
  if (providersRefreshed && !providerState.error.value && !healthState.error.value) {
    lastOperation.value = '配置备份已恢复，Provider 与自检状态已刷新。'
    return
  }
  const message = '配置备份已恢复，但状态刷新未完全成功，请手动重新加载。'
  appMessage.value = { level: 'error', message }
  lastOperation.value = message
}

function selectView(view: AppView) {
  activeView.value = view
  if (view !== 'providers') startCreatingProvider.value = false
}

let stopNotification: (() => void) | undefined
onMounted(async () => {
  await nextTick()
  await healthState.runExtended()
  try {
    stopNotification = await relay.onAppNotification((notification) => {
      appMessage.value = notification
      lastOperation.value = notification.message
    })
  } catch {
    appMessage.value = { level: 'error', message: '无法监听应用通知。' }
  }
})

onUnmounted(() => stopNotification?.())
</script>

<template>
  <div v-if="startupLoading" class="startup-screen" aria-live="polite">
    <strong>Codex Relay</strong>
    <span>正在加载本机配置…</span>
  </div>

  <OnboardingView
    v-else-if="showOnboarding"
    :busy="providerState.busy?.value || settingsState.busy.value"
    :current-provider-name="providerState.activeProvider.value?.name ?? null"
    :can-import-current-key="providerState.currentAuthImportAvailable.value"
    :success-message="providerState.successMessage.value"
    :error-message="providerState.error.value?.message ?? settingsState.error.value?.message ?? null"
    @open-directory="settingsState.openDirectory"
    @add-provider="addFirstProvider"
    @later="configureLater"
    @exit="exitApplication"
    @import-current-key="importCurrentKey"
  />

  <div v-else class="app-shell">
    <header class="app-header">
      <div>
        <p class="eyebrow">Codex Relay</p>
        <h1>Provider 控制台</h1>
      </div>
      <nav class="app-nav" aria-label="主导航">
        <button type="button" aria-label="打开 Providers" @click="selectView('providers')">Providers</button>
        <button type="button" aria-label="打开自检" @click="selectView('health')">自检</button>
        <button type="button" aria-label="打开备份与恢复" @click="selectView('backups')">备份</button>
        <button type="button" aria-label="打开设置" @click="selectView('settings')">设置</button>
      </nav>
    </header>

    <AppNotification :message="appMessage?.message ?? null" :level="appMessage?.level ?? 'success'" />

    <section class="app-content">
      <ProvidersView
        v-if="activeView === 'providers'"
        :key="startCreatingProvider ? 'providers-create' : 'providers-list'"
        :start-creating="startCreatingProvider"
        @provider-created="handleProviderCreated"
        @create-cancelled="handleCreateCancelled"
      />
      <HealthStatus
        v-else-if="activeView === 'health'"
        class="health-view"
        :report="healthState.report.value"
        :loading="healthState.loading.value"
        :busy="healthState.busy.value"
        :error-message="healthState.error.value?.message ?? null"
        @rerun="healthState.runExtended"
      />
      <BackupsView v-else-if="activeView === 'backups'" @restored="handleBackupRestored" />
      <SettingsView v-else />
    </section>

    <footer class="status-bar" aria-label="应用状态" aria-live="polite">
      <span>配置目录：{{ healthState.report.value?.configDirectory ?? '正在检测' }}</span>
      <span>当前 Provider：{{ providerState.activeProvider.value?.name ?? '未设置' }}</span>
      <span>最近操作：{{ operationText }}</span>
      <span>自检：{{ healthLabel }}</span>
      <button type="button" aria-label="打开 Codex 配置目录" @click="settingsState.openDirectory">
        打开目录
      </button>
    </footer>
  </div>
</template>

<style scoped>
.startup-screen {
  display: grid;
  min-height: 100vh;
  place-content: center;
  gap: 0.5rem;
  text-align: center;
}

.app-shell {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
  min-height: 100vh;
}

.app-header,
.app-nav,
.status-bar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.app-header {
  justify-content: space-between;
  border-bottom: 1px solid #d0d5dd;
  padding: 1rem 1.25rem;
}

.app-header h1,
.eyebrow {
  margin: 0;
}

.eyebrow {
  color: #356ae6;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.app-content {
  min-height: 0;
  overflow: auto;
}

.health-view {
  padding: 1.25rem;
}

.status-bar {
  flex-wrap: wrap;
  border-top: 1px solid #d0d5dd;
  padding: 0.65rem 1rem;
  font-size: 0.82rem;
}

.status-bar button {
  margin-left: auto;
}

@media (max-width: 720px) {
  .app-header {
    align-items: stretch;
    flex-direction: column;
  }

  .app-nav {
    flex-wrap: wrap;
  }

  .status-bar {
    align-items: stretch;
    flex-direction: column;
  }

  .status-bar button {
    margin-left: 0;
  }
}
</style>
