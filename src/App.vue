<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, shallowRef, watch } from 'vue'
import { ElButton, ElConfigProvider } from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import aboutIcon from './assets/icons/about.svg'
import backupsIcon from './assets/icons/backups.svg'
import healthIcon from './assets/icons/health.svg'
import providersIcon from './assets/icons/providers.svg'
import settingsIcon from './assets/icons/settings.svg'
import AppNotification from './components/AppNotification.vue'
import HealthStatus from './components/HealthStatus.vue'
import SelfCheckErrorBanner from './components/SelfCheckErrorBanner.vue'
import UpdateAvailableBanner from './components/UpdateAvailableBanner.vue'
import { useHealth } from './composables/useHealth'
import { useProviders } from './composables/useProviders'
import { useSettings } from './composables/useSettings'
import { useUpdater } from './composables/useUpdater'
import * as relay from './services/tauri'
import AboutView from './views/AboutView.vue'
import BackupsView from './views/BackupsView.vue'
import OnboardingView from './views/OnboardingView.vue'
import ProvidersView from './views/ProvidersView.vue'
import SettingsView from './views/SettingsView.vue'

type AppView = 'providers' | 'health' | 'backups' | 'settings' | 'about'
type AppMessage = { level: 'success' | 'error'; message: string }

const providerState = useProviders()
const healthState = useHealth()
const settingsState = useSettings()
const updater = useUpdater({
  getProxy: () => {
    const proxy = settingsState.settings.value?.networkProxy
    return proxy?.enabled && proxy.url ? proxy.url : undefined
  },
})
const activeView = shallowRef<AppView>('providers')
const healthCheckTarget = shallowRef<{ id: string } | null>(null)
const onboardingDismissed = shallowRef(false)
const startCreatingProvider = shallowRef(false)
const pendingFirstProvider = shallowRef(false)
const lastOperation = shallowRef<string | null>(null)
const appMessage = shallowRef<AppMessage | null>(null)
const appMessageId = shallowRef(0)
const appVersion = shallowRef<string | null>(null)

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
const selfCheckErrorCount = computed(
  () => healthState.report.value?.checks.filter((item) => item.level === 'error').length ?? 0,
)
const firstSelfCheckErrorId = computed(
  () => healthState.report.value?.checks.find((item) => item.level === 'error')?.id ?? null,
)
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

function showAppMessage(message: AppMessage) {
  appMessage.value = message
  appMessageId.value += 1
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

async function importCurrentKey(name: string) {
  const providerId = providerState.activeProvider.value?.id
  if (providerId) await providerState.importCurrentKey(providerId, name)
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
    showAppMessage({ level: 'error', message: '无法退出应用，请使用托盘菜单中的“退出”。' })
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
  showAppMessage({ level: 'error', message })
  lastOperation.value = message
}

function selectView(view: AppView) {
  activeView.value = view
  healthCheckTarget.value = null
  if (view !== 'providers') startCreatingProvider.value = false
}

function openSelfCheckErrorDetails() {
  const id = firstSelfCheckErrorId.value
  if (!id) return
  selectView('health')
  healthCheckTarget.value = { id }
}

let startupUpdateCheckStarted = false
const stopStartupUpdateWatch = watch(
  settingsState.loading,
  (loading) => {
    if (loading || startupUpdateCheckStarted) return
    startupUpdateCheckStarted = true
    void updater.checkSilently()
  },
  { immediate: true },
)

let stopNotification: (() => void) | undefined
let updateCheckTimer: ReturnType<typeof setInterval> | undefined
onMounted(async () => {
  updateCheckTimer = setInterval(() => {
    void updater.checkSilently()
  }, 60 * 60 * 1000)
  await nextTick()
  const versionPromise = relay.getCurrentVersion().then((version) => {
    appVersion.value = version
  }).catch(() => undefined)
  await Promise.all([healthState.runExtended(), versionPromise])
  try {
    stopNotification = await relay.onAppNotification((notification) => {
      showAppMessage(notification)
      lastOperation.value = notification.message
    })
  } catch {
    showAppMessage({ level: 'error', message: '无法监听应用通知。' })
  }
})

onUnmounted(() => {
  stopStartupUpdateWatch()
  if (updateCheckTimer !== undefined) clearInterval(updateCheckTimer)
  stopNotification?.()
})
</script>

<template>
  <ElConfigProvider :locale="zhCn" size="large" :z-index="3000">
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
        <ElButton
          text
          native-type="button"
          aria-label="打开 Providers"
          :aria-current="activeView === 'providers' ? 'page' : undefined"
          @click="selectView('providers')"
        ><img :src="providersIcon" alt="" />Providers</ElButton>
        <ElButton
          text
          native-type="button"
          aria-label="打开自检"
          :aria-current="activeView === 'health' ? 'page' : undefined"
          @click="selectView('health')"
        ><img :src="healthIcon" alt="" />自检</ElButton>
        <ElButton
          text
          native-type="button"
          aria-label="打开备份与恢复"
          :aria-current="activeView === 'backups' ? 'page' : undefined"
          @click="selectView('backups')"
        ><img :src="backupsIcon" alt="" />备份</ElButton>
        <ElButton
          text
          native-type="button"
          aria-label="打开设置"
          :aria-current="activeView === 'settings' ? 'page' : undefined"
          @click="selectView('settings')"
        ><img :src="settingsIcon" alt="" />设置</ElButton>
        <ElButton
          text
          native-type="button"
          aria-label="打开关于"
          :aria-current="activeView === 'about' ? 'page' : undefined"
          @click="selectView('about')"
        ><img :src="aboutIcon" alt="" />关于</ElButton>
      </nav>
    </header>

    <UpdateAvailableBanner
      v-if="updater.release.value"
      class="update-available-banner-slot"
      :version="updater.release.value.version"
      @view-update="selectView('settings')"
    />

    <SelfCheckErrorBanner
      v-if="selfCheckErrorCount > 0"
      class="self-check-error-banner-slot"
      :error-count="selfCheckErrorCount"
      @view-details="openSelfCheckErrorDetails"
    />

    <AppNotification
      class="app-notification-slot"
      :message="appMessage?.message ?? null"
      :level="appMessage?.level ?? 'success'"
      :message-id="appMessageId"
    />

    <section class="app-content">
      <ProvidersView
        v-if="activeView === 'providers'"
        :key="startCreatingProvider ? 'providers-create' : 'providers-list'"
        :start-creating="startCreatingProvider"
        :network-proxy-enabled="settingsState.settings.value?.networkProxy.enabled ?? false"
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
        :target-check="healthCheckTarget"
        @rerun="healthState.runExtended"
      />
      <BackupsView v-else-if="activeView === 'backups'" @restored="handleBackupRestored" />
      <SettingsView v-else-if="activeView === 'settings'" :updater="updater" />
      <AboutView
        v-else
        :app-version="appVersion"
        :config-directory="healthState.report.value?.configDirectory ?? null"
        @open-directory="settingsState.openDirectory"
      />
    </section>

    <footer class="status-bar" aria-label="应用状态" aria-live="polite" role="status">
      <span>配置目录：{{ healthState.report.value?.configDirectory ?? '正在检测' }}</span>
      <span>当前 Provider：{{ providerState.activeProvider.value?.name ?? '未设置' }}</span>
      <span>最近操作：{{ operationText }}</span>
      <span>自检：{{ healthLabel }}</span>
      <ElButton text native-type="button" aria-label="打开 Codex 配置目录" @click="settingsState.openDirectory">
        打开目录
      </ElButton>
    </footer>
    </div>
  </ElConfigProvider>
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
  grid-template-rows: auto auto auto auto minmax(0, 1fr) auto;
  min-height: 100vh;
}

.app-header {
  grid-row: 1;
}

.update-available-banner-slot {
  grid-row: 2;
}

.self-check-error-banner-slot {
  grid-row: 3;
}

.app-notification-slot {
  grid-row: 4;
  margin-block: 0.75rem;
  margin-inline: 1.25rem;
}

.app-content {
  grid-row: 5;
}

.status-bar {
  grid-row: 6;
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
  border-bottom: 1px solid var(--border);
  padding: 1rem 1.25rem;
  background: var(--surface);
}

.app-header h1,
.eyebrow {
  margin: 0;
}

.eyebrow {
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.app-content {
  min-height: 0;
  overflow: auto;
}

.app-nav :deep(.el-button) {
  display: inline-flex;
  align-items: center;
}

.app-nav :deep(.el-button > span) {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.app-nav :deep(.el-button[aria-current='page']) {
  border-color: var(--accent);
  color: var(--accent-strong);
  background: var(--accent-soft);
}

.app-nav img {
  width: 1.15rem;
  height: 1.15rem;
}

.health-view {
  padding: 1.25rem;
}

.status-bar {
  flex-wrap: wrap;
  border-top: 1px solid var(--border);
  padding: 0.65rem 1rem;
  font-size: 0.82rem;
  background: var(--surface);
}

.status-bar :deep(.el-button) {
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

  .status-bar :deep(.el-button) {
    margin-left: 0;
  }
}
</style>
