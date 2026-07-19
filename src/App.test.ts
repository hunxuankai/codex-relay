import { flushPromises, mount } from '@vue/test-utils'
import { computed, ref, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { HealthReport } from './types/health'
import type { ProviderProfile } from './types/provider'
import type { Settings, SettingsState } from './types/settings'
import App from './App.vue'

const mocks = vi.hoisted(() => ({
  useProviders: vi.fn(),
  useHealth: vi.fn(),
  useSettings: vi.fn(),
  exitApplication: vi.fn(),
  onAppNotification: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('./composables/useProviders', () => ({ useProviders: mocks.useProviders }))
vi.mock('./composables/useHealth', () => ({ useHealth: mocks.useHealth }))
vi.mock('./composables/useSettings', () => ({ useSettings: mocks.useSettings }))
vi.mock('./services/tauri', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./services/tauri')>()),
  exitApplication: mocks.exitApplication,
  onAppNotification: mocks.onAppNotification,
}))

const provider: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  wireApi: 'responses',
  model: null,
  apiKeyConfigured: true,
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

const stubs = {
  ProvidersView: {
    props: ['startCreating'],
    emits: ['providerCreated', 'createCancelled'],
    template: '<div data-view="providers">Providers {{ startCreating ? "create" : "list" }}<button aria-label="模拟首个 Provider 创建成功" @click="$emit(\'providerCreated\')">created</button><button aria-label="模拟取消首个 Provider" @click="$emit(\'createCancelled\')">cancel</button></div>',
  },
  BackupsView: {
    emits: ['restored'],
    template: '<div data-view="backups">Backups<button aria-label="模拟恢复完成" @click="$emit(\'restored\')">restore</button></div>',
  },
  SettingsView: { template: '<div data-view="settings">Settings</div>' },
}

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.onAppNotification.mockResolvedValue(() => {})
  })

  it('shows onboarding for missing configuration and opens the first Provider editor', async () => {
    const state = controllers({ onboarding: true })
    mocks.useProviders.mockReturnValue(state.providerState)
    mocks.useHealth.mockReturnValue(state.healthState)
    mocks.useSettings.mockReturnValue(state.settingsState)
    const wrapper = mount(App, { global: { stubs } })

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
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('Providers')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('自检')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('备份')
    expect(wrapper.get('[aria-label="主导航"]').text()).toContain('设置')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('C:\\safe-test\\codex')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('Provider A')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('正常')
    expect(wrapper.get('[aria-label="应用状态"]').text()).toContain('请重启 Codex 后生效')

    await wrapper.get('[aria-label="打开备份与恢复"]').trigger('click')
    await wrapper.get('[aria-label="模拟恢复完成"]').trigger('click')
    await flushPromises()
    expect(state.providerState.refresh).toHaveBeenCalledOnce()
    expect(state.healthState.runExtended).toHaveBeenCalledTimes(2)

    await wrapper.get('[aria-label="打开设置"]').trigger('click')
    expect(wrapper.find('[data-view="settings"]').exists()).toBe(true)
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
