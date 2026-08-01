import { readonly, shallowRef } from 'vue'
import {
  DEFAULT_RELEASE_PROXY_SETTINGS,
  type ReleaseProxySettings,
} from '../types/network'

export const RELEASE_PROXY_PREFERENCE_KEY =
  'codex-relay-release-console.proxy-preference.v1'

export type ReleaseProxyPreferenceStorage = Pick<Storage, 'getItem' | 'setItem'>

interface UseReleaseProxyPreferenceOptions {
  storage?: ReleaseProxyPreferenceStorage | null
}

interface StoredReleaseProxyPreference {
  version: 1
  settings: ReleaseProxySettings
}

function isProxyType(value: unknown): value is ReleaseProxySettings['proxyType'] {
  return value === 'http' || value === 'socks5'
}

function browserStorage(): ReleaseProxyPreferenceStorage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage
  } catch {
    return null
  }
}

function loadSettings(storage: ReleaseProxyPreferenceStorage | null): ReleaseProxySettings {
  if (!storage) return { ...DEFAULT_RELEASE_PROXY_SETTINGS }
  try {
    const raw = storage.getItem(RELEASE_PROXY_PREFERENCE_KEY)
    if (raw === null) return { ...DEFAULT_RELEASE_PROXY_SETTINGS }
    const stored = JSON.parse(raw) as Partial<StoredReleaseProxyPreference> | null
    const settings = stored?.settings
    if (
      stored?.version !== 1 ||
      typeof settings?.enabled !== 'boolean' ||
      !isProxyType(settings.proxyType) ||
      typeof settings.host !== 'string' ||
      (settings.port !== null &&
        (typeof settings.port !== 'number' ||
          !Number.isInteger(settings.port) ||
          settings.port < 1 ||
          settings.port > 65_535))
    ) {
      return { ...DEFAULT_RELEASE_PROXY_SETTINGS }
    }
    return { ...settings }
  } catch {
    return { ...DEFAULT_RELEASE_PROXY_SETTINGS }
  }
}

export function useReleaseProxyPreference(options: UseReleaseProxyPreferenceOptions = {}) {
  const storage = options.storage === undefined ? browserStorage() : options.storage
  const settings = shallowRef(loadSettings(storage))

  function update(value: ReleaseProxySettings) {
    const next = { ...value }
    settings.value = next
    try {
      storage?.setItem(
        RELEASE_PROXY_PREFERENCE_KEY,
        JSON.stringify({ version: 1, settings: next }),
      )
    } catch {
      // 代理偏好不含秘密，只是便利状态；存储不可用时保留当前会话内存值。
    }
  }

  return {
    settings: readonly(settings),
    update,
  }
}
