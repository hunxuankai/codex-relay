import { describe, expect, it } from 'vitest'
import {
  RELEASE_PROXY_PREFERENCE_KEY,
  useReleaseProxyPreference,
} from './useReleaseProxyPreference'

class MemoryStorage implements Pick<Storage, 'getItem' | 'setItem'> {
  readonly values = new Map<string, string>()

  getItem(key: string) {
    return this.values.get(key) ?? null
  }

  setItem(key: string, value: string) {
    this.values.set(key, value)
  }
}

const DEFAULT_SETTINGS = {
  enabled: false,
  proxyType: 'http',
  host: '',
  port: null,
} as const

describe('useReleaseProxyPreference', () => {
  it('restores a valid v1 proxy preference across application starts', () => {
    const storage = new MemoryStorage()
    storage.setItem(
      RELEASE_PROXY_PREFERENCE_KEY,
      JSON.stringify({
        version: 1,
        settings: {
          enabled: true,
          proxyType: 'socks5',
          host: '127.0.0.1',
          port: 1080,
        },
      }),
    )

    const preference = useReleaseProxyPreference({ storage })

    expect(preference.settings.value).toEqual({
      enabled: true,
      proxyType: 'socks5',
      host: '127.0.0.1',
      port: 1080,
    })
  })

  it.each([
    '{ invalid json',
    JSON.stringify({
      version: 2,
      settings: { enabled: true, proxyType: 'socks5', host: 'proxy.test', port: 1080 },
    }),
    JSON.stringify({
      version: 1,
      settings: { ...DEFAULT_SETTINGS, proxyType: 'ftp' },
    }),
    JSON.stringify({
      version: 1,
      settings: { ...DEFAULT_SETTINGS, port: 0 },
    }),
  ])('falls back safely when the stored proxy preference is invalid', (raw) => {
    const storage = new MemoryStorage()
    storage.setItem(RELEASE_PROXY_PREFERENCE_KEY, raw)

    const preference = useReleaseProxyPreference({ storage })

    expect(preference.settings.value).toEqual(DEFAULT_SETTINGS)
  })

  it('persists only the non-secret proxy fields and keeps them while disabled', () => {
    const storage = new MemoryStorage()
    const preference = useReleaseProxyPreference({ storage })

    preference.update({
      enabled: false,
      proxyType: 'socks5',
      host: 'proxy.example.test',
      port: 1080,
    })

    expect(preference.settings.value).toEqual({
      enabled: false,
      proxyType: 'socks5',
      host: 'proxy.example.test',
      port: 1080,
    })
    expect(JSON.parse(storage.getItem(RELEASE_PROXY_PREFERENCE_KEY) ?? '')).toEqual({
      version: 1,
      settings: {
        enabled: false,
        proxyType: 'socks5',
        host: 'proxy.example.test',
        port: 1080,
      },
    })
    expect(useReleaseProxyPreference({ storage }).settings.value).toEqual(
      preference.settings.value,
    )
  })

  it('keeps an in-memory preference when browser storage is unavailable', () => {
    const unreadableStorage = {
      getItem: () => {
        throw new Error('storage unavailable')
      },
      setItem: () => undefined,
    }
    expect(useReleaseProxyPreference({ storage: unreadableStorage }).settings.value).toEqual(
      DEFAULT_SETTINGS,
    )

    const unwritableStorage = {
      getItem: () => null,
      setItem: () => {
        throw new Error('storage unavailable')
      },
    }
    const preference = useReleaseProxyPreference({ storage: unwritableStorage })
    expect(() =>
      preference.update({
        enabled: true,
        proxyType: 'http',
        host: '127.0.0.1',
        port: 7890,
      }),
    ).not.toThrow()
    expect(preference.settings.value).toEqual({
      enabled: true,
      proxyType: 'http',
      host: '127.0.0.1',
      port: 7890,
    })
  })
})
