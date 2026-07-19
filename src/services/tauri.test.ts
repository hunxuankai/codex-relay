import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BackupSummary } from '../types/backup'
import type { HealthReport } from '../types/health'
import type {
  CreateProviderInput,
  ProviderListState,
  ProviderMutationOutcome,
  SwitchOutcome,
  UpdateProviderInput,
} from '../types/provider'
import type { Settings, SettingsState } from '../types/settings'
import {
  RelayCommandError,
  createProvider,
  deleteProvider,
  getProviderApiKey,
  getSettings,
  importCurrentAuthKey,
  listBackups,
  listProviders,
  onProvidersChanged,
  openCodexDirectory,
  restoreBackup,
  runCriticalSelfCheck,
  runExtendedSelfCheck,
  saveSettings,
  setAutostart,
  switchProvider,
  updateProvider,
} from './tauri'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))

const invokeMock = vi.mocked(invoke)
const listenMock = vi.mocked(listen)

const fingerprints = {
  config: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'config' },
  auth: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'auth' },
  providers: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'providers' },
}

const providerState: ProviderListState = {
  providers: [],
  activeProviderId: null,
  currentAuthImportAvailable: false,
  fingerprints,
}

const mutation: ProviderMutationOutcome = {
  providers: [],
  message: 'Provider 已保存。',
}

const switched: SwitchOutcome = {
  providers: [],
  activeProviderId: 'provider-a',
  message: '已切换到「Provider A」。配置已写入，请重启 Codex 后生效。',
}

const settings: Settings = {
  autostartEnabled: false,
  trayOnlyOnAutostart: true,
  closeToTray: true,
  showWindowOnManualStart: true,
  window: { width: 900, height: 620, x: null, y: null },
  firstRunCompleted: false,
}

const settingsState: SettingsState = {
  settings,
  autostart: { configuredEnabled: false, actualEnabled: false, isConsistent: true },
}

const health: HealthReport = {
  level: 'normal',
  checks: [],
  configDirectory: 'C:\\test\\codex',
  currentProvider: null,
  generatedAt: '2026-07-20T00:00:00+08:00',
}

const backups: BackupSummary[] = []

function success<T>(data: T) {
  return { success: true, data }
}

describe('Tauri service boundary', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    listenMock.mockReset()
  })

  it('calls the exact Rust command names and arguments', async () => {
    invokeMock
      .mockResolvedValueOnce(success(providerState))
      .mockResolvedValueOnce(success('test-key-not-real'))
      .mockResolvedValueOnce(success(mutation))
      .mockResolvedValueOnce(success(mutation))
      .mockResolvedValueOnce(success(mutation))
      .mockResolvedValueOnce(success(switched))
      .mockResolvedValueOnce(success(mutation))
      .mockResolvedValueOnce(success(settingsState))
      .mockResolvedValueOnce(success(settingsState))
      .mockResolvedValueOnce(success(settingsState))
      .mockResolvedValueOnce(success(undefined))
      .mockResolvedValueOnce(success(backups))
      .mockResolvedValueOnce(success(mutation))
      .mockResolvedValueOnce(success(health))
      .mockResolvedValueOnce(success(health))

    const createInput: CreateProviderInput = {
      id: 'provider-a',
      name: 'Provider A',
      baseUrl: 'https://provider-a.example.test/v1',
      wireApi: 'responses',
      model: 'model-a',
      apiKey: 'test-key-not-real',
      activateAfterSave: true,
      expectedFiles: fingerprints,
    }
    const updateInput: UpdateProviderInput = {
      id: 'provider-a',
      name: 'Provider A',
      baseUrl: 'https://provider-a.example.test/v1',
      wireApi: 'responses',
      model: null,
      apiKeyChange: { action: 'set', value: 'replacement-key-not-real' },
      syncIfActive: true,
      expectedFiles: fingerprints,
    }

    await listProviders()
    await getProviderApiKey('provider-a')
    await createProvider(createInput)
    await updateProvider(updateInput)
    await deleteProvider('provider-a', fingerprints)
    await switchProvider('provider-a')
    await importCurrentAuthKey('provider-a')
    await getSettings()
    await saveSettings(settings)
    await setAutostart(true)
    await openCodexDirectory()
    await listBackups()
    await restoreBackup('backup-1')
    await runCriticalSelfCheck()
    await runExtendedSelfCheck()

    expect(invokeMock.mock.calls).toEqual([
      ['list_providers'],
      ['get_provider_api_key', { providerId: 'provider-a' }],
      ['create_provider', { input: createInput }],
      ['update_provider', { input: updateInput }],
      ['delete_provider', { providerId: 'provider-a', expectedFiles: fingerprints }],
      ['switch_provider', { providerId: 'provider-a' }],
      ['import_current_auth_key', { providerId: 'provider-a' }],
      ['get_settings'],
      ['save_settings', { settings }],
      ['set_autostart', { enabled: true }],
      ['open_codex_directory'],
      ['list_backups'],
      ['restore_backup', { directoryName: 'backup-1' }],
      ['run_critical_self_check'],
      ['run_extended_self_check'],
    ])
  })

  it('throws only the safe command code and message', async () => {
    invokeMock.mockResolvedValue({
      success: false,
      error: { code: 'INVALID_PROVIDER', message: 'Provider 配置无效。' },
    })

    const error = await listProviders().catch((caught: unknown) => caught)

    expect(error).toBeInstanceOf(RelayCommandError)
    expect(error).toMatchObject({
      code: 'INVALID_PROVIDER',
      message: 'Provider 配置无效。',
    })
    expect(JSON.stringify(error)).not.toContain('stack')
    expect(JSON.stringify(error)).not.toContain('test-key')
  })

  it('subscribes to typed Provider refresh events', async () => {
    const unlisten = vi.fn()
    let eventHandler: ((event: { payload: ProviderListState }) => void) | undefined
    listenMock.mockImplementation(async (_event, handler) => {
      eventHandler = handler as (event: { payload: ProviderListState }) => void
      return unlisten
    })
    const handler = vi.fn()

    const stop = await onProvidersChanged(handler)
    eventHandler?.({ payload: providerState })

    expect(listenMock).toHaveBeenCalledWith('providers-changed', expect.any(Function))
    expect(handler).toHaveBeenCalledWith(providerState)
    stop()
    expect(unlisten).toHaveBeenCalledOnce()
  })
})
