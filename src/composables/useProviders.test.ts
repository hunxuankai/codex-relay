import { flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { RelayCommandError } from '../services/tauri'
import type {
  CreateProviderInput,
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
}

function provider(id: string, active = false): ProviderProfile {
  return {
    id,
    name: id === 'provider-a' ? 'Provider A' : 'Provider B',
    baseUrl: `https://${id}.example.test/v1`,
    wireApi: 'responses',
    model: null,
    apiKeyConfigured: true,
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
  }
}

function mutation(message: string): ProviderMutationOutcome {
  return { providers: [], message }
}

function client(overrides: Partial<ProviderClient> = {}): ProviderClient {
  return {
    listProviders: vi.fn().mockResolvedValue(state([])),
    createProvider: vi.fn().mockResolvedValue(mutation('已保存。')),
    updateProvider: vi.fn().mockResolvedValue(mutation('已更新。')),
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
