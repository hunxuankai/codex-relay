import { flushPromises, mount } from '@vue/test-utils'
import { computed, nextTick, ref, shallowRef, type ShallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { HealthReport } from './types/health'
import type { ProviderProfile } from './types/provider'
import type { Settings, SettingsState } from './types/settings'
import type { UpdaterController, UseUpdaterOptions } from './composables/useUpdater'
import type { UpdateProgress, UpdateReleaseInfo } from './types/update'
import { providerConnection } from './test-utils/provider'
import App from './App.vue'

const mocks = vi.hoisted(() => ({
  useProviders: vi.fn(),
  useHealth: vi.fn(),
  useSettings: vi.fn(),
  useUpdater: vi.fn(),
  exitApplication: vi.fn(),
  getCurrentVersion: vi.fn().mockResolvedValue('0.1.2'),
  onAppNotification: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('./composables/useProviders', () => ({ useProviders: mocks.useProviders }))
vi.mock('./composables/useHealth', () => ({ useHealth: mocks.useHealth }))
vi.mock('./composables/useSettings', () => ({ useSettings: mocks.useSettings }))
vi.mock('./composables/useUpdater', () => ({ useUpdater: mocks.useUpdater }))
vi.mock('./services/tauri', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./services/tauri')>()),
  exitApplication: mocks.exitApplication,
  getCurrentVersion: mocks.getCurrentVersion,
  onAppNotification: mocks.onAppNotification,
}))

const provider: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  baseUrls: [{ id: 'url-primary', name: '主用地址', url: 'https://provider-a.example.test/v1' }],
  selectedBaseUrlId: 'url-primary',
  baseUrlStatus: 'managed',
  apiKeys: [{ id: 'key-primary', name: '主用密钥' }],
  selectedApiKeyId: 'key-primary',
  apiKeyStatus: 'managed',
  wireApi: 'responses',
  models: ['gpt-5.6-sol'],
  selectedModel: 'gpt-5.6-sol',
  reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
  fastEnabled: false,
  preferenceConfigured: true,
  apiKeyConfigured: true,
  configurationComplete: true,
  disabledReason: null,
  connection: providerConnection(),
  isActive: true,
  isValid: true,
  validationMessage: null,
}

const baseSettings: Settings = {
  autostartEnabled: false,
  trayOnlyOnAutostart: true,
  closeToTray: true,
  showWindowOnManualStart: true,
  window: { width: 900, height: 620, x: null, y: null },
  firstRunCompleted: true,
  networkProxy: { enabled: false, url: '' },
}

function healthReport(configExists = true): HealthReport {
  return {
    level: configExists ? 'normal' : 'warning',
    configDirectory: 'C:\\safe-test\\codex',
    currentProvider: configExists ? 'provider-a' : null,
    generatedAt: '2026-07-20T00:00:00+08:00',
    checks: [{
      id: 'config-file',
      label: 'config.toml',
      level: configExists ? 'normal' : 'warning',
      message: configExists ? 'config.toml 已就绪。' : 'config.toml 尚不存在。',
    }],
  }
}

function controllers(options: { onboarding?: boolean } = {}) {
  const providers = ref(options.onboarding ? [] : [provider])
  const activeProvider = computed(() => providers.value.find((item) => item.isActive) ?? null)
  const providerState = {
    providers,
    activeProvider,
    currentAuthImportAvailable: shallowRef(false),
    loading: shallowRef(false),
    error: shallowRef<{ code: string; message: string } | null>(null),
    successMessage: shallowRef<string | null>('已切换到「Provider A」。请重启 Codex 后生效。'),
    refresh: vi.fn().mockResolvedValue(true),
    importCurrentKey: vi.fn(),
  }
  const report = shallowRef(healthReport(!options.onboarding))
  const healthState = {
    report,
    loading: shallowRef(false),
    busy: shallowRef(false),
    error: shallowRef<{ code: string; message: string } | null>(null),
    refreshCritical: vi.fn(),
    runExtended: vi.fn(),
  }
  const state = shallowRef<SettingsState>({
    settings: { ...baseSettings, firstRunCompleted: !options.onboarding },
    autostart: { configuredEnabled: false, actualEnabled: false, isConsistent: true },
  })
  const settingsState = {
    state,
    settings: computed(() => state.value.settings),
    autostart: computed(() => state.value.autostart),
    loading: shallowRef(false),
    busy: shallowRef(false),
    error: shallowRef<{ code: string; message: string } | null>(null),
    successMessage: shallowRef<string | null>(null),
    refresh: vi.fn(),
    save: vi.fn(),
    setAutostart: vi.fn(),
    openDirectory: vi.fn(),
  }
  return { providerState, healthState, settingsState }
}

function updaterController(): UpdaterController {
  return {
    status: shallowRef('idle'),
    currentVersion: shallowRef<string | null>('0.1.2'),
    release: shallowRef<UpdateReleaseInfo | null>(null),
    error: shallowRef(null),
    progress: shallowRef<UpdateProgress | null>(null),
    check: vi.fn(),
    checkSilently: vi.fn(),
    reset: vi.fn(),
    requestInstall: vi.fn(),
    cancelInstall: vi.fn(),
    confirmInstall: vi.fn(),
  } as unknown as UpdaterController
}

const stubs = {
  ProvidersView: {
    props: ['startCreating', 'networkProxyEnabled'],
    emits: ['providerCreated', 'createCancelled'],
    template: '<div data-view="providers" :data-proxy-enabled="String(networkProxyEnabled)">Providers {{ startCreating ? "create" : "list" }}<button aria-label="模拟首个 Provider 创建成功" @click="$emit(\'providerCreated\')">created</button><button aria-label="模拟取消首个 Provider" @click="$emit(\'createCancelled\')">cancel</button></div>',
  },
  BackupsView: {
    emits: ['restored'],
    template: '<div data-view="backups">Backups<button aria-label="模拟恢复完成" @click="$emit(\'restored\')">restore</button></div>',
  },
  SettingsView: { props: ['updater'], template: '<div data-view="settings">Settings {{ updater ? "shared" : "missing" }}</div>' },
  AboutView: {
    props: ['appVersion', 'configDirectory'],
    emits: ['openDirectory'],
    template: '<div data-view="about">About {{ appVersion }} {{ configDirectory }}<button aria-label="模拟打开配置目录" @click="$emit(\'openDirectory\')">open</button></div>',
  },
}

describe('App', () => {
  let updater: UpdaterController

  beforeEach(() => {
    vi.useRealTimers()
    vi.clearAllMocks()
    updater = updaterController()
    mocks.useUpdater.mockReturnValue(updater)
    mocks.getCurrentVersion.mockResolvedValue('0.1.2')
    mocks.onAppNotification.mockResolvedValue(() => {})
  })

  it('shows onboarding for missing configuration and opens the first Provider editor', async () => {
    const state = controllers({ onboarding: true })
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)
    const wrapper = mount(App, { global: { stubs } })
    await flushPromises()

    expect(updater.checkSilently).toHaveBeenCalledOnce()
    expect(wrapper.find('[aria-label="软件更新提示"]').exists()).toBe(false)

    expect(wrapper.text()).toContain('首次设置')
    expect(wrapper.text()).toContain('打开 Codex 配置目录')
    expect(wrapper.text()).toContain('新增第一个 Provider')
    expect(wrapper.text()).toContain('稍后设置')
    expect(wrapper.text()).toContain('退出')

    await wrapper.get('[aria-label="新增第一个 Provider"]').trigger('click')
    await flushPromises()
    expect(state.settingsState.save).not.toHaveBeenCalled()
    expect(wrapper.get('[data-view="providers"]').text()).toContain('create')

    await wrapper.get('[aria-label="模拟取消首个 Provider"]').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('首次设置')
    expect(state.settingsState.save).not.toHaveBeenCalled()

    await wrapper.get('[aria-label="新增第一个 Provider"]').trigger('click')
    await wrapper.get('[aria-label="模拟首个 Provider 创建成功"]').trigger('click')
    await flushPromises()
    expect(state.settingsState.save).toHaveBeenCalledWith({
      ...state.settingsState.settings.value,
      firstRunCompleted: true,
      networkProxy: { enabled: false, url: '' },
    })
  })

  it('provides navigation, startup health, status fields, and post-restore refreshes', async () => {
    const state = controllers()
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)
    const wrapper = mount(App, { global: { stubs } })
    await flushPromises()

    expect(state.healthState.runExtended).toHaveBeenCalledTimes(1)
    expect(wrapper.get('.app-header').text()).toContain('Codex Relay v0.1.2')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('Providers')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('自检')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('备份')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('设置')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('关于')
    expect(wrapper.get('[aria-label="打开 Providers"]').attributes('aria-current')).toBe('page')
    expect(wrapper.get('[aria-label="主导航"]').findAll('img[alt=""]')).toHaveLength(5)
    expect(wrapper.get('[aria-label="应用状态"]').attributes('role')).toBe('status')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('C:\\safe-test\\codex')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('Provider A')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('正常')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('请重启 Codex 后生效')

    await wrapper.get('[aria-label="打开备份与恢复"]').trigger('click')
    expect(wrapper.get('[aria-label="打开备份与恢复"]').attributes('aria-current')).toBe('page')
    await wrapper.get('[aria-label="模拟恢复完成"]').trigger('click')
    await flushPromises()
    expect(state.providerState.refresh).toHaveBeenCalledOnce()
    expect(state.healthState.runExtended).toHaveBeenCalledTimes(2)

    await wrapper.get('[aria-label="打开设置"]').trigger('click')
    expect(wrapper.find('[data-view="settings"]').exists()).toBe(true)
    expect(wrapper.get('[data-view="settings"]').text()).toContain('shared')

    await wrapper.get('[aria-label="打开关于"]').trigger('click')
    expect(wrapper.get('[data-view="about"]').text()).toContain('0.1.2')
    expect(wrapper.get('[data-view="about"]').text()).toContain('C:\\safe-test\\codex')
    await wrapper.get('[aria-label="模拟打开配置目录"]').trigger('click')
    expect(state.settingsState.openDirectory).toHaveBeenCalledOnce()
  })

  it('keeps a usable title when the runtime version cannot be read', async () => {
    const state = controllers()
    mocks.getCurrentVersion.mockRejectedValueOnce(new Error('version unavailable'))
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)

    const wrapper = mount(App, { global: { stubs } })
    await flushPromises()

    expect(wrapper.get('.app-header').text()).toContain('Codex Relay')
    expect(wrapper.get('.app-header').text()).not.toContain('Codex Relay v')
  })

  it('checks on startup and hourly with the latest proxy, then opens the shared update in settings', async () => {
    vi.useFakeTimers()
    const state = controllers()
    state.settingsState.state.value = {
      ...state.settingsState.state.value,
      settings: {
        ...state.settingsState.state.value.settings,
        networkProxy: { enabled: true, url: 'http://127.0.0.1:7890' },
      },
    }
    state.healthState.report.value = {
      ...healthReport(),
      level: 'error',
      checks: [{
        id: 'config-file',
        label: 'config.toml',
        level: 'error',
        message: 'config.toml 无法解析。',
      }],
    }
    ;(updater.release as ShallowRef<UpdateReleaseInfo | null>).value = {
      currentVersion: '0.1.2',
      version: '0.2.0',
      date: null,
      notes: null,
    }
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)
    const wrapper = mount(App, { global: { stubs } })
    await flushPromises()

    expect(updater.checkSilently).toHaveBeenCalledOnce()
    const options = mocks.useUpdater.mock.calls[0]?.[0] as UseUpdaterOptions
    expect(options.getProxy?.()).toBe('http://127.0.0.1:7890')
    expect(wrapper.get('[data-view="providers"]').attributes('data-proxy-enabled')).toBe('true')

    state.settingsState.state.value = {
      ...state.settingsState.state.value,
      settings: {
        ...state.settingsState.state.value.settings,
        networkProxy: { enabled: true, url: 'http://127.0.0.1:7897' },
      },
    }
    await vi.advanceTimersByTimeAsync(60 * 60 * 1000)

    expect(updater.checkSilently).toHaveBeenCalledTimes(2)
    expect(options.getProxy?.()).toBe('http://127.0.0.1:7897')

    const updateBanner = wrapper.get('[aria-label="软件更新提示"]')
    const healthBanner = wrapper.get('[aria-label="系统自检错误提示"]')
    expect(updateBanner.text()).toContain('0.2.0')
    expect(updateBanner.element.nextElementSibling).toBe(healthBanner.element)

    await updateBanner.get('[aria-label="前往软件更新设置"]').trigger('click')

    expect(wrapper.get('[aria-label="打开设置"]').attributes('aria-current')).toBe('page')
    expect(wrapper.get('[data-view="settings"]').text()).toContain('shared')

    wrapper.unmount()
    await vi.advanceTimersByTimeAsync(60 * 60 * 1000)
    expect(updater.checkSilently).toHaveBeenCalledTimes(2)
  })

  it('shows self-check errors below the header and opens their details', async () => {
    const scrollIntoView = vi.fn()
    Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
      configurable: true,
      value: scrollIntoView,
    })
    const state = controllers()
    state.healthState.report.value = {
      ...healthReport(),
      level: 'error',
      checks: [
        {
          id: 'config-file',
          label: 'config.toml',
          level: 'error',
          message: 'config.toml 无法解析。',
        },
        {
          id: 'auth-file',
          label: 'auth.json',
          level: 'error',
          message: 'auth.json 与当前 Provider 不一致。',
        },
      ],
    }
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)
    const wrapper = mount(App, { attachTo: document.body, global: { stubs } })
    await flushPromises()

    const alert = wrapper.get('[aria-label="系统自检错误提示"]')
    expect(alert.attributes('role')).toBe('alert')
    expect(alert.text()).toContain('系统自检发现 2 个错误项')
    expect(alert.element.previousElementSibling).toBe(wrapper.get('.app-header').element)

    await alert.get('[aria-label="查看自检详情"]').trigger('click')

    expect(wrapper.get('[aria-label="自检状态"]').text()).toContain('系统自检')
    expect(wrapper.get('[aria-label="打开自检"]').attributes('aria-current')).toBe('page')
    const firstError = wrapper.get('[data-check-id="config-file"]')
    expect(firstError.attributes('data-targeted')).toBe('true')
    expect(document.activeElement).toBe(firstError.element)
    expect(scrollIntoView).toHaveBeenCalledWith({ block: 'center' })

    state.healthState.report.value = healthReport()
    await nextTick()

    expect(wrapper.find('[aria-label="系统自检错误提示"]').exists()).toBe(false)

    state.healthState.report.value = healthReport(false)
    await nextTick()

    expect(wrapper.find('[aria-label="系统自检错误提示"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('does not claim post-restore refresh success when a refresh fails', async () => {
    const state = controllers()
    state.providerState.refresh.mockResolvedValue(false)
    state.healthState.runExtended
      .mockResolvedValueOnce(undefined)
      .mockImplementationOnce(async () => {
        state.healthState.error.value = {
          code: 'HEALTH_REFRESH_FAILED',
          message: '自检刷新失败。',
        }
      })
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)
    const wrapper = mount(App, { global: { stubs } })
    await flushPromises()

    await wrapper.get('[aria-label="打开备份与恢复"]').trigger('click')
    await wrapper.get('[aria-label="模拟恢复完成"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('状态刷新未完全成功')
    expect(wrapper.get('[aria-label="应用状态"]').text()).not.toContain('Provider 与自检状态已刷新')
  })
})
