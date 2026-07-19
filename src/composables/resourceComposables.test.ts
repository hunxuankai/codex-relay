import { flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { RelayCommandError } from '../services/tauri'
import type { BackupSummary } from '../types/backup'
import type { HealthReport } from '../types/health'
import type { ProviderMutationOutcome } from '../types/provider'
import type { Settings, SettingsState } from '../types/settings'
import { useBackups, type BackupClient } from './useBackups'
import { useHealth, type HealthClient } from './useHealth'
import { useSettings, type SettingsClient } from './useSettings'

const normalHealth: HealthReport = {
  level: 'normal',
  checks: [],
  configDirectory: 'C:\\test\\codex',
  currentProvider: 'provider-a',
  generatedAt: '2026-07-20T00:00:00+08:00',
}

const warningHealth: HealthReport = {
  ...normalHealth,
  level: 'warning',
  generatedAt: '2026-07-20T00:01:00+08:00',
}

const backup: BackupSummary = {
  directoryName: 'backup-1',
  metadata: {
    transactionId: 'tx-1',
    createdAt: '2026-07-20T00:00:00+08:00',
    operation: 'switch_provider',
    providerId: 'provider-a',
    configExisted: true,
    authExisted: true,
    providersExisted: true,
    appVersion: '0.1.0',
  },
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

describe('resource composables', () => {
  it('loads critical health and replaces it with an extended report', async () => {
    const client: HealthClient = {
      runCriticalSelfCheck: vi.fn().mockResolvedValue(normalHealth),
      runExtendedSelfCheck: vi.fn().mockResolvedValue(warningHealth),
      onSelfCheckCompleted: vi.fn().mockResolvedValue(() => {}),
    }
    const health = useHealth({ client, subscribe: false })
    await flushPromises()

    expect(health.report.value).toEqual(normalHealth)
    await health.runExtended()
    expect(health.report.value).toEqual(warningHealth)
    expect(health.busy.value).toBe(false)
  })

  it('restores a backup, refreshes the list, and keeps backend success text', async () => {
    const restored: ProviderMutationOutcome = { providers: [], message: '配置备份已恢复。' }
    const listBackups = vi
      .fn()
      .mockResolvedValueOnce([backup])
      .mockResolvedValueOnce([backup])
    const client: BackupClient = {
      listBackups,
      restoreBackup: vi.fn().mockResolvedValue(restored),
    }
    const backups = useBackups({ client })
    await flushPromises()

    await backups.restore('backup-1')

    expect(client.restoreBackup).toHaveBeenCalledWith('backup-1')
    expect(listBackups).toHaveBeenCalledTimes(2)
    expect(backups.successMessage.value).toBe('配置备份已恢复。')
  })

  it('saves settings and replaces state with the verified backend state', async () => {
    const enabledState: SettingsState = {
      settings: { ...settings, autostartEnabled: true },
      autostart: { configuredEnabled: true, actualEnabled: true, isConsistent: true },
    }
    const client: SettingsClient = {
      getSettings: vi.fn().mockResolvedValue(settingsState),
      saveSettings: vi.fn().mockResolvedValue(settingsState),
      setAutostart: vi.fn().mockResolvedValue(enabledState),
      openCodexDirectory: vi.fn().mockResolvedValue(undefined),
      onSettingsChanged: vi.fn().mockResolvedValue(() => {}),
    }
    const state = useSettings({ client, subscribe: false })
    await flushPromises()

    await state.setAutostart(true)

    expect(client.setAutostart).toHaveBeenCalledWith(true)
    expect(state.settings.value?.autostartEnabled).toBe(true)
    expect(state.autostart.value?.actualEnabled).toBe(true)
  })

  it('maps unexpected settings failures to safe UI errors', async () => {
    const client: SettingsClient = {
      getSettings: vi
        .fn()
        .mockRejectedValue(new RelayCommandError('SETTINGS_INVALID', '设置文件无效。')),
      saveSettings: vi.fn(),
      setAutostart: vi.fn(),
      openCodexDirectory: vi.fn(),
      onSettingsChanged: vi.fn().mockResolvedValue(() => {}),
    }

    const state = useSettings({ client, subscribe: false })
    await flushPromises()

    expect(state.error.value).toEqual({ code: 'SETTINGS_INVALID', message: '设置文件无效。' })
  })

  it('does not report restore success when backup refresh fails', async () => {
    const client: BackupClient = {
      listBackups: vi
        .fn()
        .mockResolvedValueOnce([backup])
        .mockRejectedValueOnce(new RelayCommandError('BACKUP_REFRESH_FAILED', '刷新备份失败。')),
      restoreBackup: vi
        .fn()
        .mockResolvedValue({ providers: [], message: '配置备份已恢复。' }),
    }
    const backups = useBackups({ client })
    await flushPromises()

    await backups.restore('backup-1')

    expect(backups.successMessage.value).toBeNull()
    expect(backups.error.value).toEqual({
      code: 'BACKUP_REFRESH_FAILED',
      message: '刷新备份失败。',
    })
  })

  it('keeps a newer health event when the initial request finishes later', async () => {
    let resolveInitial!: (report: HealthReport) => void
    let eventHandler!: (report: HealthReport) => void
    const client: HealthClient = {
      runCriticalSelfCheck: vi.fn().mockReturnValue(new Promise((resolve) => {
        resolveInitial = resolve
      })),
      runExtendedSelfCheck: vi.fn(),
      onSelfCheckCompleted: vi.fn().mockImplementation(async (handler) => {
        eventHandler = handler
        return () => {}
      }),
    }
    const health = useHealth({ client })
    eventHandler(warningHealth)

    resolveInitial(normalHealth)
    await flushPromises()

    expect(health.report.value).toEqual(warningHealth)
  })

  it('keeps a newer settings event when the initial request finishes later', async () => {
    let resolveInitial!: (state: SettingsState) => void
    let eventHandler!: (state: SettingsState) => void
    const enabledState: SettingsState = {
      settings: { ...settings, autostartEnabled: true },
      autostart: { configuredEnabled: true, actualEnabled: true, isConsistent: true },
    }
    const client: SettingsClient = {
      getSettings: vi.fn().mockReturnValue(new Promise((resolve) => {
        resolveInitial = resolve
      })),
      saveSettings: vi.fn(),
      setAutostart: vi.fn(),
      openCodexDirectory: vi.fn(),
      onSettingsChanged: vi.fn().mockImplementation(async (handler) => {
        eventHandler = handler
        return () => {}
      }),
    }
    const state = useSettings({ client })
    eventHandler(enabledState)

    resolveInitial(settingsState)
    await flushPromises()

    expect(state.settings.value?.autostartEnabled).toBe(true)
  })

  it('maps health and settings subscription failures without unhandled rejections', async () => {
    const health = useHealth({
      client: {
        runCriticalSelfCheck: vi.fn().mockResolvedValue(normalHealth),
        runExtendedSelfCheck: vi.fn(),
        onSelfCheckCompleted: vi
          .fn()
          .mockRejectedValue(new RelayCommandError('HEALTH_LISTEN_FAILED', '自检监听失败。')),
      },
    })
    const state = useSettings({
      client: {
        getSettings: vi.fn().mockResolvedValue(settingsState),
        saveSettings: vi.fn(),
        setAutostart: vi.fn(),
        openCodexDirectory: vi.fn(),
        onSettingsChanged: vi
          .fn()
          .mockRejectedValue(new RelayCommandError('SETTINGS_LISTEN_FAILED', '设置监听失败。')),
      },
    })
    await flushPromises()

    expect(health.error.value).toEqual({
      code: 'HEALTH_LISTEN_FAILED',
      message: '自检监听失败。',
    })
    expect(state.error.value).toEqual({
      code: 'SETTINGS_LISTEN_FAILED',
      message: '设置监听失败。',
    })
  })
})
