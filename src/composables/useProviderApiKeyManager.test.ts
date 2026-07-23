import { effectScope } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import { RelayCommandError } from '../services/tauri'
import type {
  ProviderApiKeyManagementState,
  ProviderMutationOutcome,
} from '../types/provider'
import {
  useProviderApiKeyManager,
  type ProviderApiKeyManagerClient,
} from './useProviderApiKeyManager'

const fingerprints = {
  config: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'config' },
  auth: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'auth' },
  providers: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'providers' },
  preferences: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'preferences' },
}

function managementState(
  providerId: string,
  key = 'test-key-management-not-real',
): ProviderApiKeyManagementState {
  return {
    providerId,
    entries: [{ id: `${providerId}-key`, name: '主用密钥', apiKey: key }],
    selectedApiKeyId: `${providerId}-key`,
    apiKeyStatus: 'managed',
    fingerprints,
  }
}

const mutation: ProviderMutationOutcome = {
  providers: [],
  message: 'API Key 已保存。',
}

function client(overrides: Partial<ProviderApiKeyManagerClient> = {}): ProviderApiKeyManagerClient {
  return {
    getProviderApiKeysForManagement: vi.fn().mockResolvedValue(managementState('provider-a')),
    saveProviderApiKeys: vi.fn().mockResolvedValue(mutation),
    ...overrides,
  }
}

describe('useProviderApiKeyManager', () => {
  it('loads complete keys only into local state and reloads authoritative IDs after saving', async () => {
    const reloaded = managementState('provider-a', 'test-key-reloaded-not-real')
    reloaded.entries[0]!.id = 'stable-key-id'
    const getProviderApiKeysForManagement = vi
      .fn()
      .mockResolvedValueOnce(managementState('provider-a'))
      .mockResolvedValueOnce(reloaded)
    const saveProviderApiKeys = vi.fn().mockResolvedValue(mutation)
    const onSaved = vi.fn().mockResolvedValue(undefined)
    const manager = useProviderApiKeyManager({
      client: client({ getProviderApiKeysForManagement, saveProviderApiKeys }),
      onSaved,
    })

    await manager.load('provider-a')
    expect(manager.entries.value).toHaveLength(1)
    expect(manager.entries.value[0]?.apiKey === 'test-key-management-not-real').toBe(true)

    manager.replaceEntries([
      ...manager.entries.value,
      { id: null, name: '备用密钥', apiKey: 'test-key-backup-not-real' },
    ])
    await manager.save()

    expect(saveProviderApiKeys).toHaveBeenCalledOnce()
    const saveInput = saveProviderApiKeys.mock.calls[0]?.[0]
    expect(saveInput?.providerId).toBe('provider-a')
    expect(saveInput?.expectedFiles).toEqual(fingerprints)
    expect(saveInput?.entries[1]?.apiKey === 'test-key-backup-not-real').toBe(true)
    expect(getProviderApiKeysForManagement).toHaveBeenCalledTimes(2)
    expect(manager.entries.value[0]?.id).toBe('stable-key-id')
    expect(manager.entries.value[0]?.apiKey === 'test-key-reloaded-not-real').toBe(true)
    expect(onSaved).toHaveBeenCalledWith(mutation)
    expect(manager.successMessage.value).toBe('API Key 已保存。')
  })

  it('discards a late load after clear and releases complete keys on scope dispose', async () => {
    let resolveLoad!: (state: ProviderApiKeyManagementState) => void
    const pending = new Promise<ProviderApiKeyManagementState>((resolve) => {
      resolveLoad = resolve
    })
    const getProviderApiKeysForManagement = vi
      .fn()
      .mockReturnValueOnce(pending)
      .mockResolvedValueOnce(managementState('provider-b'))
    const scope = effectScope()
    const manager = scope.run(() => useProviderApiKeyManager({
      client: client({ getProviderApiKeysForManagement }),
    }))!

    const loading = manager.load('provider-a')
    manager.clear()
    resolveLoad(managementState('provider-a'))
    await loading

    expect(manager.providerId.value).toBeNull()
    expect(manager.entries.value).toEqual([])

    await manager.load('provider-b')
    expect(manager.entries.value).toHaveLength(1)
    scope.stop()
    expect(manager.providerId.value).toBeNull()
    expect(manager.entries.value).toEqual([])
  })

  it('blocks duplicate saves while busy and exposes only safe command errors', async () => {
    let finishSave!: (outcome: ProviderMutationOutcome) => void
    const saveProviderApiKeys = vi.fn().mockReturnValue(
      new Promise<ProviderMutationOutcome>((resolve) => {
        finishSave = resolve
      }),
    )
    const api = client({ saveProviderApiKeys })
    const manager = useProviderApiKeyManager({ client: api })
    await manager.load('provider-a')

    const first = manager.save()
    const duplicate = manager.save()
    expect(saveProviderApiKeys).toHaveBeenCalledOnce()
    finishSave(mutation)
    await Promise.all([first, duplicate])

    vi.mocked(api.getProviderApiKeysForManagement).mockRejectedValueOnce(
      new RelayCommandError('INVALID_PROVIDER_SECRETS', 'Provider 密钥文件无效。'),
    )
    await manager.load('provider-a')

    expect(manager.error.value).toEqual({
      code: 'INVALID_PROVIDER_SECRETS',
      message: 'Provider 密钥文件无效。',
    })
    expect(JSON.stringify(manager.error.value)).not.toContain('test-key')
  })
})
