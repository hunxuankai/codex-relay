import { flushPromises, mount } from '@vue/test-utils'
import { computed, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Settings, SettingsState } from '../types/settings'
import type { UpdaterController } from '../composables/useUpdater'
import SettingsView from './SettingsView.vue'

vi.mock('../components/UpdatePanel.vue', () => ({
  default: { props: ['updater'], template: '<section data-update-panel :data-has-updater="Boolean(updater)" />' },
}))

const mockUseSettings = vi.hoisted(() => vi.fn())
vi.mock('../composables/useSettings', () => ({ useSettings: mockUseSettings }))
const mockUseProxyDiscovery = vi.hoisted(() => vi.fn())
vi.mock('../composables/useProxyDiscovery', () => ({ useProxyDiscovery: mockUseProxyDiscovery }))

const baseSettings: Settings = {
  autostartEnabled: true,
  trayOnlyOnAutostart: true,
  closeToTray: true,
  showWindowOnManualStart: true,
  window: { width: 900, height: 620, x: null, y: null },
  firstRunCompleted: true,
  networkProxy: { enabled: false, url: '' },
}

function controller(overrides: Partial<SettingsState> = {}) {
  const state = shallowRef<SettingsState>({
    settings: baseSettings,
    autostart: { configuredEnabled: true, actualEnabled: false, isConsistent: false },
    ...overrides,
  })
  return {
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
}

function proxyController() {
  return {
    confirmationOpen: shallowRef(false),
    resultsOpen: shallowRef(false),
    testing: shallowRef(false),
    discovering: shallowRef(false),
    availableProxies: shallowRef<string[]>([]),
    selectedProxy: shallowRef<string | null>(null),
    message: shallowRef<string | null>(null),
    error: shallowRef<{ code: string; message: string } | null>(null),
    requestDiscovery: vi.fn(),
    cancelDiscovery: vi.fn(),
    confirmDiscovery: vi.fn(),
    testCurrentProxy: vi.fn(),
    selectProxy: vi.fn(),
    closeResults: vi.fn(),
  }
}

function updaterController(): UpdaterController {
  return {
    status: shallowRef('idle'),
    currentVersion: shallowRef('0.1.2'),
    release: shallowRef(null),
    error: shallowRef(null),
    progress: shallowRef(null),
    check: vi.fn(),
    checkSilently: vi.fn(),
    reset: vi.fn(),
    requestInstall: vi.fn(),
    cancelInstall: vi.fn(),
    confirmInstall: vi.fn(),
  } as unknown as UpdaterController
}

describe('SettingsView', () => {
  beforeEach(() => {
    mockUseSettings.mockReset()
    mockUseProxyDiscovery.mockReset()
    mockUseProxyDiscovery.mockReturnValue(proxyController())
  })

  it('shows the actual Windows autostart state and inconsistency', async () => {
    const settings = controller()
    mockUseSettings.mockReturnValue(settings)
    const wrapper = mount(SettingsView, { props: { updater: updaterController() } })

    expect(wrapper.text()).toContain('Windows 实际状态：未启用')
    expect(wrapper.text()).toContain('设置与 Windows 实际状态不一致')

    await wrapper.get('[aria-label="登录 Windows 后自动启动"]').setValue(true)
    await flushPromises()
    expect(settings.setAutostart).toHaveBeenCalledWith(true)
  })

  it('saves tray-only, close-to-tray, and manual-start visibility settings', async () => {
    const settings = controller({
      autostart: { configuredEnabled: true, actualEnabled: true, isConsistent: true },
    })
    mockUseSettings.mockReturnValue(settings)
    const wrapper = mount(SettingsView, { props: { updater: updaterController() } })

    await wrapper.get('[name="tray-only-on-autostart"]').setValue(false)
    await wrapper.get('[name="close-to-tray"]').setValue(false)
    await wrapper.get('[name="show-window-on-manual-start"]').setValue(false)
    await wrapper.get('form').trigger('submit')

    expect(settings.save).toHaveBeenCalledWith({
      ...baseSettings,
      trayOnlyOnAutostart: false,
      closeToTray: false,
      showWindowOnManualStart: false,
    })
  })

  it('surfaces safe autostart plugin errors', () => {
    const settings = controller()
    settings.error.value = { code: 'AUTOSTART_FAILED', message: '无法更新 Windows 开机启动。' }
    mockUseSettings.mockReturnValue(settings)

    expect(mount(SettingsView, { props: { updater: updaterController() } }).text()).toContain('无法更新 Windows 开机启动。')
  })

  it('places updater actions outside the settings form', () => {
    mockUseSettings.mockReturnValue(controller())
    const updater = updaterController()
    const wrapper = mount(SettingsView, { props: { updater } })

    expect(wrapper.find('[data-update-panel]').exists()).toBe(true)
    expect(wrapper.get('[data-update-panel]').attributes('data-has-updater')).toBe('true')
    expect(wrapper.get('form').find('[data-update-panel]').exists()).toBe(false)
  })

  it('tests, discovers, and immediately saves the selected local proxy', async () => {
    const settings = controller()
    const proxy = proxyController()
    proxy.resultsOpen.value = true
    proxy.availableProxies.value = ['http://127.0.0.1:7890', 'http://127.0.0.1:7897']
    proxy.selectedProxy.value = 'http://127.0.0.1:7897'
    mockUseSettings.mockReturnValue(settings)
    mockUseProxyDiscovery.mockReturnValue(proxy)
    const wrapper = mount(SettingsView, { props: { updater: updaterController() } })

    await wrapper.get('[name="proxy-url"]').setValue('http://127.0.0.1:7890')
    await wrapper.get('[data-action="test-proxy"]').trigger('click')
    expect(proxy.testCurrentProxy).toHaveBeenCalledWith('http://127.0.0.1:7890')

    await wrapper.get('[data-action="apply-proxy"]').trigger('click')
    await flushPromises()
    expect(settings.save).toHaveBeenCalledWith({
      ...baseSettings,
      networkProxy: { enabled: true, url: 'http://127.0.0.1:7897' },
    })
    expect(proxy.closeResults).toHaveBeenCalledOnce()
  })
})
