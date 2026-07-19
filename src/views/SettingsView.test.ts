import { flushPromises, mount } from '@vue/test-utils'
import { computed, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Settings, SettingsState } from '../types/settings'
import SettingsView from './SettingsView.vue'

const mockUseSettings = vi.hoisted(() => vi.fn())
vi.mock('../composables/useSettings', () => ({ useSettings: mockUseSettings }))

const baseSettings: Settings = {
  autostartEnabled: true,
  trayOnlyOnAutostart: true,
  closeToTray: true,
  showWindowOnManualStart: true,
  window: { width: 900, height: 620, x: null, y: null },
  firstRunCompleted: true,
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

describe('SettingsView', () => {
  beforeEach(() => mockUseSettings.mockReset())

  it('shows the actual Windows autostart state and inconsistency', async () => {
    const settings = controller()
    mockUseSettings.mockReturnValue(settings)
    const wrapper = mount(SettingsView)

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
    const wrapper = mount(SettingsView)

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

    expect(mount(SettingsView).text()).toContain('无法更新 Windows 开机启动。')
  })
})
