import { computed, getCurrentScope, onScopeDispose, readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'
import type { Settings, SettingsState } from '../types/settings'

export interface SettingsClient {
  getSettings(): Promise<SettingsState>
  saveSettings(settings: Settings): Promise<SettingsState>
  setAutostart(enabled: boolean): Promise<SettingsState>
  openCodexDirectory(): Promise<void>
  onSettingsChanged(handler: (state: SettingsState) => void): Promise<() => void>
}

export interface UseSettingsOptions {
  client?: SettingsClient
  subscribe?: boolean
}

const defaultClient: SettingsClient = {
  getSettings: relay.getSettings,
  saveSettings: relay.saveSettings,
  setAutostart: relay.setAutostart,
  openCodexDirectory: relay.openCodexDirectory,
  onSettingsChanged: relay.onSettingsChanged,
}

export function useSettings(options: UseSettingsOptions = {}) {
  const client = options.client ?? defaultClient
  const state = shallowRef<SettingsState | null>(null)
  const loading = shallowRef(true)
  const busy = shallowRef(false)
  const error = shallowRef<RelayUiError | null>(null)
  const successMessage = shallowRef<string | null>(null)
  const settings = computed(() => state.value?.settings ?? null)
  const autostart = computed(() => state.value?.autostart ?? null)
  let stateSequence = 0

  function setError(caught: unknown) {
    error.value = caught instanceof relay.RelayCommandError
      ? { code: caught.code, message: caught.message }
      : { code: 'UNEXPECTED_ERROR', message: '设置操作失败，请重试。' }
  }

  async function refresh() {
    const requestSequence = ++stateSequence
    loading.value = true
    error.value = null
    try {
      const next = await client.getSettings()
      if (requestSequence === stateSequence) {
        state.value = next
      }
    } catch (caught) {
      if (requestSequence === stateSequence) {
        setError(caught)
      }
    } finally {
      if (requestSequence === stateSequence) {
        loading.value = false
      }
    }
  }

  async function runStateAction(action: () => Promise<SettingsState>, message: string) {
    if (busy.value) return
    const requestSequence = ++stateSequence
    busy.value = true
    error.value = null
    successMessage.value = null
    try {
      const next = await action()
      if (requestSequence === stateSequence) {
        state.value = next
      }
      successMessage.value = message
    } catch (caught) {
      setError(caught)
    } finally {
      busy.value = false
    }
  }

  function save(next: Settings) {
    return runStateAction(() => client.saveSettings(next), '设置已保存。')
  }

  function setAutostart(enabled: boolean) {
    return runStateAction(
      () => client.setAutostart(enabled),
      enabled ? '已启用开机自动启动。' : '已关闭开机自动启动。',
    )
  }

  async function openDirectory() {
    if (busy.value) return
    busy.value = true
    error.value = null
    try {
      await client.openCodexDirectory()
    } catch (caught) {
      setError(caught)
    } finally {
      busy.value = false
    }
  }

  let disposed = false
  let stopSubscription: (() => void) | undefined
  if (options.subscribe ?? true) {
    void client
      .onSettingsChanged((next) => {
        stateSequence += 1
        state.value = next
        loading.value = false
      })
      .then((stop) => {
        if (disposed) stop()
        else stopSubscription = stop
      })
      .catch((caught: unknown) => {
        if (!disposed) setError(caught)
      })
  }
  if (getCurrentScope()) {
    onScopeDispose(() => {
      disposed = true
      stopSubscription?.()
    })
  }

  void refresh()

  return {
    state: readonly(state),
    settings,
    autostart,
    loading: readonly(loading),
    busy: readonly(busy),
    error: readonly(error),
    successMessage: readonly(successMessage),
    refresh,
    save,
    setAutostart,
    openDirectory,
  }
}
