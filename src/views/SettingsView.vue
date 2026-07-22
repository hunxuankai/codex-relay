<script setup lang="ts">
import { reactive, watch } from 'vue'
import { ElButton, ElCard, ElSwitch } from 'element-plus'
import AppNotification from '../components/AppNotification.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import ProxyDiscoveryDialog from '../components/ProxyDiscoveryDialog.vue'
import ProxySettingsPanel from '../components/ProxySettingsPanel.vue'
import UpdatePanel from '../components/UpdatePanel.vue'
import { useProxyDiscovery } from '../composables/useProxyDiscovery'
import { useSettings } from '../composables/useSettings'
import type { UpdaterController } from '../composables/useUpdater'

defineProps<{ updater: UpdaterController }>()

const settingsState = useSettings()
const proxyDiscovery = useProxyDiscovery()
const draft = reactive({
  trayOnlyOnAutostart: true,
  closeToTray: true,
  showWindowOnManualStart: true,
  networkProxy: { enabled: false, url: '' },
})

watch(
  settingsState.settings,
  (settings) => {
    if (!settings) return
    draft.trayOnlyOnAutostart = settings.trayOnlyOnAutostart
    draft.closeToTray = settings.closeToTray
    draft.showWindowOnManualStart = settings.showWindowOnManualStart
    draft.networkProxy = { ...settings.networkProxy }
  },
  { immediate: true },
)

function toggleAutostart(value: boolean | string | number) {
  settingsState.setAutostart(Boolean(value))
}

function save() {
  const current = settingsState.settings.value
  if (!current) return
  settingsState.save({
    ...current,
    trayOnlyOnAutostart: draft.trayOnlyOnAutostart,
    closeToTray: draft.closeToTray,
    showWindowOnManualStart: draft.showWindowOnManualStart,
    networkProxy: { ...draft.networkProxy },
  })
}

function testProxy() {
  const proxy = draft.networkProxy.url.trim()
  if (proxy) void proxyDiscovery.testCurrentProxy(proxy)
}

async function applyDetectedProxy() {
  const current = settingsState.settings.value
  const proxy = proxyDiscovery.selectedProxy.value
  if (!current || !proxy) return
  await settingsState.save({
    ...current,
    networkProxy: { enabled: true, url: proxy },
  })
  if (!settingsState.error.value) {
    draft.networkProxy = { enabled: true, url: proxy }
    proxyDiscovery.closeResults()
  }
}
</script>

<template>
  <main class="settings-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Settings</p>
        <h1>应用设置</h1>
      </div>
      <ElButton native-type="button" :disabled="settingsState.loading.value" @click="settingsState.refresh">
        刷新状态
      </ElButton>
    </header>

    <AppNotification :message="settingsState.successMessage.value" level="success" />
    <AppNotification :message="settingsState.error.value?.message ?? null" level="error" />
    <AppNotification :message="proxyDiscovery.message.value" level="success" />
    <AppNotification :message="proxyDiscovery.error.value?.message ?? null" level="error" />

    <p v-if="settingsState.loading.value && !settingsState.settings.value">正在加载设置…</p>
    <form v-else-if="settingsState.settings.value" class="settings-form" @submit.prevent="save">
      <ElCard class="settings-section" shadow="never">
        <h2>开机启动</h2>
        <label class="setting-row">
          <span>
            <strong>登录 Windows 后自动启动</strong>
            <small>
              Windows 实际状态：{{ settingsState.autostart.value?.actualEnabled ? '已启用' : '未启用' }}
            </small>
          </span>
          <ElSwitch
            aria-label="登录 Windows 后自动启动"
            :model-value="settingsState.autostart.value?.actualEnabled ?? false"
            :disabled="settingsState.busy.value"
            @change="toggleAutostart"
          />
        </label>
        <p v-if="settingsState.autostart.value && !settingsState.autostart.value.isConsistent" class="warning" role="status">
          设置与 Windows 实际状态不一致，请重新切换或刷新后重试。
        </p>
      </ElCard>

      <ProxySettingsPanel
        v-model="draft.networkProxy"
        :busy="settingsState.busy.value"
        :testing="proxyDiscovery.testing.value"
        :discovering="proxyDiscovery.discovering.value"
        @test="testProxy"
        @discover="proxyDiscovery.requestDiscovery"
      />

      <ElCard class="settings-section" shadow="never">
        <h2>窗口与托盘</h2>
        <label class="setting-row">
          <span>开机自动启动时仅显示托盘</span>
          <ElSwitch
            v-model="draft.trayOnlyOnAutostart"
            name="tray-only-on-autostart"
            aria-label="开机自动启动时仅显示托盘"
          />
        </label>
        <label class="setting-row">
          <span>关闭窗口时隐藏到托盘</span>
          <ElSwitch v-model="draft.closeToTray" name="close-to-tray" aria-label="关闭窗口时隐藏到托盘" />
        </label>
        <label class="setting-row">
          <span>手动启动时显示主窗口</span>
          <ElSwitch
            v-model="draft.showWindowOnManualStart"
            name="show-window-on-manual-start"
            aria-label="手动启动时显示主窗口"
          />
        </label>
      </ElCard>

      <div class="settings-actions">
        <ElButton type="primary" native-type="submit" :disabled="settingsState.busy.value">保存设置</ElButton>
        <ElButton native-type="button" :disabled="settingsState.busy.value" @click="settingsState.openDirectory">
          打开 Codex 配置目录
        </ElButton>
      </div>
    </form>

    <UpdatePanel :updater="updater" />

    <ConfirmDialog
      :open="proxyDiscovery.confirmationOpen.value"
      title="检测本机代理"
      message="将并行检测 127.0.0.1 上的六个常用 HTTP 代理端口，并通过每个候选访问 GitHub 更新源。检测会产生少量网络请求，确认前不会修改设置。"
      confirm-label="开始检测"
      tone="neutral"
      @confirm="proxyDiscovery.confirmDiscovery"
      @cancel="proxyDiscovery.cancelDiscovery"
    />
    <ProxyDiscoveryDialog
      :open="proxyDiscovery.resultsOpen.value"
      :candidates="proxyDiscovery.availableProxies.value"
      :selected="proxyDiscovery.selectedProxy.value"
      @select="proxyDiscovery.selectProxy"
      @confirm="applyDetectedProxy"
      @cancel="proxyDiscovery.closeResults"
    />
  </main>
</template>

<style scoped>
.settings-view,
.settings-form {
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
}

.settings-section :deep(.el-card__body) {
  display: grid;
  gap: 1rem;
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
