import { flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { RelayCommandError } from '../services/tauri'
import type {
  CreateProviderInput,
  ProviderBaseUrlDraft,
  ProviderListState,
  ProviderMutationOutcome,
  ProviderProfile,
  SwitchOutcome,
} from '../types/provider'
import { useProviders, type ProviderClient } from './useProviders'

const fingerprints = {
  config: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'config' },
  auth: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'auth' },
  providers: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'providers' },
  preferences: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'preferences' },
}

function provider(id: string, active = false): ProviderProfile {
  const baseUrl = `https://${id}.example.test/v1`
  return {
    id,
    name: id === 'provider-a' ? 'Provider A' : 'Provider B',
    baseUrl,
    baseUrls: [{ id: `${id}-url`, name: '主用地址', url: baseUrl }],
    selectedBaseUrlId: `${id}-url`,
    baseUrlStatus: 'managed',
    apiKeys: [{ id: `${id}-key`, name: '主用密钥' }],
    selectedApiKeyId: `${id}-key`,
    apiKeyStatus: 'managed',
    wireApi: 'responses',
    models: ['gpt-5.6-sol'],
    selectedModel: 'gpt-5.6-sol',
    reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
    preferenceConfigured: true,
    apiKeyConfigured: true,
    configurationComplete: true,
    disabledReason: null,
    isActive: active,
    isValid: true,
    validationMessage: null,
  }
}

function state(providers: ProviderProfile[]): ProviderListState {
  return {
    providers,
    activeProviderId: providers.find((item) => item.isActive)?.id ?? null,
    currentAuthImportAvailable: false,
    fingerprints,
    modelCatalog: [],
  }
}

function mutation(message: string): ProviderMutationOutcome {
  return { providers: [], message }
}

function client(overrides: Partial<ProviderClient> = {}): ProviderClient {
  return {
    listProviders: vi.fn().mockResolvedValue(state([])),
    reorderProviders: vi.fn().mockResolvedValue(mutation('顺序已保存。')),
    createProvider: vi.fn().mockResolvedValue(mutation('已保存。')),
    updateProvider: vi.fn().mockResolvedValue(mutation('已更新。')),
    saveProviderBaseUrls: vi.fn().mockResolvedValue(mutation('地址已保存。')),
    selectProviderBaseUrl: vi.fn().mockResolvedValue(mutation('地址已切换。')),
    selectProviderApiKey: vi.fn().mockResolvedValue(mutation('密钥已切换。')),
    deleteProvider: vi.fn().mockResolvedValue(mutation('已删除。')),
    switchProvider: vi.fn().mockResolvedValue({
      providers: [],
      activeProviderId: 'provider-a',
      message: '已切换。',
    } satisfies SwitchOutcome),
    importCurrentAuthKey: vi.fn().mockResolvedValue(mutation('已导入。')),
    onProvidersChanged: vi.fn().mockResolvedValue(() => {}),
    ...overrides,
  }
}

describe('useProviders', () => {
  it('starts loading immediately and selects the active Provider', async () => {
    let resolveState!: (value: ProviderListState) => void
    const pending = new Promise<ProviderListState>((resolve) => {
      resolveState = resolve
    })
    const api = client({ listProviders: vi.fn().mockReturnValue(pending) })

    const providers = useProviders({ client: api, subscribe: false })
    expect(providers.loading.value).toBe(true)

    resolveState(state([provider('provider-a', true), provider('provider-b')]))
    await flushPromises()

    expect(providers.loading.value).toBe(false)
    expect(providers.providers.value).toHaveLength(2)
    expect(providers.selectedProviderId.value).toBe('provider-a')
  })

  it('retains an existing selection across refreshes', async () => {
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([provider('provider-a', true), provider('provider-b')]))
      .mockResolvedValueOnce(state([provider('provider-a'), provider('provider-b', true)]))
    const providers = useProviders({ client: client({ listProviders }), subscribe: false })
    await flushPromises()
    providers.selectProvider('provider-a')

    await providers.refresh()

    expect(providers.selectedProviderId.value).toBe('provider-a')
  })

  it('refreshes after creating and keeps the backend success text', async () => {
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([]))
      .mockResolvedValueOnce(state([provider('provider-a')]))
    const createProvider = vi.fn().mockResolvedValue(mutation('Provider「A」已保存。'))
    const providers = useProviders({
      client: client({ listProviders, createProvider }),
      subscribe: false,
    })
    await flushPromises()
    const input = { id: 'provider-a', apiKey: 'editor-only-key' } as CreateProviderInput

    await providers.create(input)

    expect(createProvider).toHaveBeenCalledWith(input)
    expect(listProviders).toHaveBeenCalledTimes(2)
    expect(providers.successMessage.value).toBe('Provider「A」已保存。')
    expect(
      JSON.stringify({
        providers: providers.providers.value,
        error: providers.error.value,
        successMessage: providers.successMessage.value,
      }),
    ).not.toContain('editor-only-key')
  })

  it('refreshes after deleting a Provider', async () => {
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([provider('provider-a')]))
      .mockResolvedValueOnce(state([]))
    const deleteProvider = vi.fn().mockResolvedValue(mutation('Provider「A」已删除。'))
    const providers = useProviders({
      client: client({ listProviders, deleteProvider }),
      subscribe: false,
    })
    await flushPromises()

    await providers.remove('provider-a')

    expect(deleteProvider).toHaveBeenCalledWith('provider-a', fingerprints)
    expect(providers.providers.value).toEqual([])
  })

  it('persists a complete Provider order with the current fingerprint and refreshes', async () => {
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([provider('provider-a'), provider('provider-b')]))
      .mockResolvedValueOnce(state([provider('provider-b'), provider('provider-a')]))
    const reorderProviders = vi.fn().mockResolvedValue(mutation('Provider 顺序已保存。'))
    const providers = useProviders({
      client: client({ listProviders, reorderProviders }),
      subscribe: false,
    })
    await flushPromises()

    await providers.reorder(['provider-b', 'provider-a'])

    expect(reorderProviders).toHaveBeenCalledWith({
      providerIds: ['provider-b', 'provider-a'],
      expectedFiles: fingerprints,
    })
    expect(providers.providers.value.map((item) => item.id)).toEqual([
      'provider-b',
      'provider-a',
    ])
    expect(providers.successMessage.value).toBe('Provider 顺序已保存。')
  })

  it('shows the dropped Provider order immediately while persistence is pending', async () => {
    let finish!: (value: ProviderMutationOutcome) => void
    const pending = new Promise<ProviderMutationOutcome>((resolve) => {
      finish = resolve
    })
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([provider('provider-a'), provider('provider-b')]))
      .mockResolvedValueOnce(state([provider('provider-b'), provider('provider-a')]))
    const providers = useProviders({
      client: client({
        listProviders,
        reorderProviders: vi.fn().mockReturnValue(pending),
      }),
      subscribe: false,
    })
    await flushPromises()

    const saving = providers.reorder(['provider-b', 'provider-a'])

    expect(providers.providers.value.map((item) => item.id)).toEqual([
      'provider-b',
      'provider-a',
    ])
    finish(mutation('Provider 顺序已保存。'))
    await saving
  })

  it('restores the previous Provider order when persistence fails', async () => {
    const providers = useProviders({
      client: client({
        listProviders: vi.fn().mockResolvedValue(
          state([provider('provider-a'), provider('provider-b')]),
        ),
        reorderProviders: vi.fn().mockRejectedValue(
          new RelayCommandError('EXTERNAL_MODIFICATION_CONFLICT', '配置文件已变化，请重试。'),
        ),
      }),
      subscribe: false,
    })
    await flushPromises()

    await providers.reorder(['provider-b', 'provider-a'])

    expect(providers.providers.value.map((item) => item.id)).toEqual([
      'provider-a',
      'provider-b',
    ])
    expect(providers.error.value).toEqual({
      code: 'EXTERNAL_MODIFICATION_CONFLICT',
      message: '配置文件已变化，请重试。',
    })
  })

  it('refreshes after switching a Provider', async () => {
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([provider('provider-a', true), provider('provider-b')]))
      .mockResolvedValueOnce(state([provider('provider-a'), provider('provider-b', true)]))
    const switchProvider = vi.fn().mockResolvedValue({
      providers: [],
      activeProviderId: 'provider-b',
      message: '已切换到「Provider B」。',
    } satisfies SwitchOutcome)
    const providers = useProviders({
      client: client({ listProviders, switchProvider }),
      subscribe: false,
    })
    await flushPromises()

    await providers.switchTo('provider-b')

    expect(switchProvider).toHaveBeenCalledWith('provider-b')
    expect(providers.activeProvider.value?.id).toBe('provider-b')
    expect(providers.successMessage.value).toBe('已切换到「Provider B」。')
  })

  it('saves Base URLs and independently selects URL and API Key using the current fingerprint', async () => {
    const listProviders = vi.fn().mockResolvedValue(
      state([provider('provider-a', true)]),
    )
    const saveProviderBaseUrls = vi.fn().mockResolvedValue(mutation('地址已保存。'))
    const selectProviderBaseUrl = vi.fn().mockResolvedValue(mutation('地址已切换。'))
    const selectProviderApiKey = vi.fn().mockResolvedValue(mutation('密钥已切换。'))
    const providers = useProviders({
      client: client({
        listProviders,
        saveProviderBaseUrls,
        selectProviderBaseUrl,
        selectProviderApiKey,
      }),
      subscribe: false,
    })
    await flushPromises()
    const entries: ProviderBaseUrlDraft[] = [
      {
        id: 'provider-a-url',
        name: '主用地址',
        url: 'https://provider-a.example.test/v1',
      },
      { id: null, name: '备用地址', url: 'https://backup.example.test/v1' },
    ]

    await providers.saveBaseUrls('provider-a', entries)
    await providers.selectBaseUrl('provider-a', 'provider-a-url')
    await providers.selectApiKey('provider-a', 'provider-a-key')

    expect(saveProviderBaseUrls).toHaveBeenCalledWith({
      providerId: 'provider-a',
      entries,
      expectedFiles: fingerprints,
    })
    expect(selectProviderBaseUrl).toHaveBeenCalledWith({
      providerId: 'provider-a',
      baseUrlId: 'provider-a-url',
      expectedFiles: fingerprints,
    })
    expect(selectProviderApiKey).toHaveBeenCalledWith({
      providerId: 'provider-a',
      apiKeyId: 'provider-a-key',
      expectedFiles: fingerprints,
    })
    expect(listProviders).toHaveBeenCalledTimes(4)
  })

  it('imports the current auth key only with an explicit name and current fingerprint', async () => {
    const importCurrentAuthKey = vi.fn().mockResolvedValue(mutation('当前密钥已导入。'))
    const providers = useProviders({
      client: client({
        listProviders: vi.fn().mockResolvedValue(state([provider('provider-a', true)])),
        importCurrentAuthKey,
      }),
      subscribe: false,
    })
    await flushPromises()

    await providers.importCurrentKey('provider-a', '从 Codex 导入')

    expect(importCurrentAuthKey).toHaveBeenCalledWith({
      providerId: 'provider-a',
      name: '从 Codex 导入',
      expectedFiles: fingerprints,
    })
    expect(providers.successMessage.value).toBe('当前密钥已导入。')
  })

  it('blocks repeated actions while busy', async () => {
    let finish!: (value: SwitchOutcome) => void
    const pending = new Promise<SwitchOutcome>((resolve) => {
      finish = resolve
    })
    const switchProvider = vi.fn().mockReturnValue(pending)
    const providers = useProviders({
      client: client({
        listProviders: vi.fn().mockResolvedValue(state([provider('provider-a', true)])),
        switchProvider,
      }),
      subscribe: false,
    })
    await flushPromises()

    const first = providers.switchTo('provider-b')
    const second = providers.switchTo('provider-b')

    expect(switchProvider).toHaveBeenCalledOnce()
    finish({ providers: [], activeProviderId: 'provider-b', message: '已切换。' })
    await Promise.all([first, second])
  })

  it('exposes only safe command error state', async () => {
    const api = client({
      listProviders: vi
        .fn()
        .mockRejectedValue(new RelayCommandError('CONFIG_INVALID', '配置文件无效。')),
    })

    const providers = useProviders({ client: api, subscribe: false })
    await flushPromises()

    expect(providers.error.value).toEqual({
      code: 'CONFIG_INVALID',
      message: '配置文件无效。',
    })
    expect(JSON.stringify(providers.error.value)).not.toContain('stack')
  })

  it('does not report mutation success when the required refresh fails', async () => {
    const listProviders = vi
      .fn()
      .mockResolvedValueOnce(state([]))
      .mockRejectedValueOnce(new RelayCommandError('REFRESH_FAILED', '刷新失败。'))
    const providers = useProviders({
      client: client({
        listProviders,
        createProvider: vi.fn().mockResolvedValue(mutation('Provider 已保存。')),
      }),
      subscribe: false,
    })
    await flushPromises()

    await providers.create({ id: 'provider-a' } as CreateProviderInput)

    expect(providers.successMessage.value).toBeNull()
    expect(providers.error.value).toEqual({ code: 'REFRESH_FAILED', message: '刷新失败。' })
  })

  it('maps Provider subscription failures to safe error state', async () => {
    const providers = useProviders({
      client: client({
        onProvidersChanged: vi
          .fn()
          .mockRejectedValue(new RelayCommandError('LISTEN_FAILED', '监听刷新事件失败。')),
      }),
    })
    await flushPromises()

    expect(providers.error.value).toEqual({
      code: 'LISTEN_FAILED',
      message: '监听刷新事件失败。',
    })
  })

  it('does not let an older request overwrite a newer Provider event', async () => {
    let resolveInitial!: (value: ProviderListState) => void
    let eventHandler!: (value: ProviderListState) => void
    const api = client({
      listProviders: vi.fn().mockReturnValue(new Promise((resolve) => {
        resolveInitial = resolve
      })),
      onProvidersChanged: vi.fn().mockImplementation(async (handler) => {
        eventHandler = handler
        return () => {}
      }),
    })
    const providers = useProviders({ client: api })
    eventHandler(state([provider('provider-b', true)]))

    resolveInitial(state([provider('provider-a', true)]))
    await flushPromises()

    expect(providers.activeProvider.value?.id).toBe('provider-b')
    expect(providers.fingerprints.value).toEqual(fingerprints)
  })
})
