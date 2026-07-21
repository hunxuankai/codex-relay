import { invoke } from '@tauri-apps/api/core'
import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event'
import { getVersion } from '@tauri-apps/api/app'
import { check, type DownloadEvent } from '@tauri-apps/plugin-updater'
import type { BackupSummary } from '../types/backup'
import type { CommandResult } from '../types/command'
import type { HealthReport } from '../types/health'
import type {
  CreateProviderInput,
  FileSetFingerprint,
  ProviderListState,
  ProviderMutationOutcome,
  SwitchOutcome,
  UpdateProviderInput,
} from '../types/provider'
import type { Settings, SettingsState } from '../types/settings'
import type { UpdateProgress, UpdateSession } from '../types/update'

export const PROVIDERS_CHANGED_EVENT = 'providers-changed'
export const CONFIG_FILES_CHANGED_EVENT = 'config-files-changed'
export const SELF_CHECK_COMPLETED_EVENT = 'self-check-completed'
export const SETTINGS_CHANGED_EVENT = 'settings-changed'
export const APP_NOTIFICATION_EVENT = 'app-notification'

export class RelayCommandError extends Error {
  readonly code: string

  constructor(code: string, message: string) {
    super(message)
    this.name = 'RelayCommandError'
    this.code = code
  }

  toJSON() {
    return { code: this.code, message: this.message }
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const result = args
    ? await invoke<CommandResult<T>>(command, args)
    : await invoke<CommandResult<T>>(command)
  if (result.success) {
    return result.data as T
  }
  throw new RelayCommandError(
    result.error?.code ?? 'UNKNOWN_ERROR',
    result.error?.message ?? '操作失败，请重试。',
  )
}

function subscribe<T>(eventName: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(eventName, (event: Event<T>) => handler(event.payload))
}

export function listProviders(): Promise<ProviderListState> {
  return call('list_providers')
}

export function getProviderApiKey(providerId: string): Promise<string | null> {
  return call('get_provider_api_key', { providerId })
}

export function createProvider(input: CreateProviderInput): Promise<ProviderMutationOutcome> {
  return call('create_provider', { input })
}

export function updateProvider(input: UpdateProviderInput): Promise<ProviderMutationOutcome> {
  return call('update_provider', { input })
}

export function deleteProvider(
  providerId: string,
  expectedFiles: FileSetFingerprint,
): Promise<ProviderMutationOutcome> {
  return call('delete_provider', { providerId, expectedFiles })
}

export function switchProvider(providerId: string): Promise<SwitchOutcome> {
  return call('switch_provider', { providerId })
}

export function importCurrentAuthKey(providerId: string): Promise<ProviderMutationOutcome> {
  return call('import_current_auth_key', { providerId })
}

export function getSettings(): Promise<SettingsState> {
  return call('get_settings')
}

export function saveSettings(settings: Settings): Promise<SettingsState> {
  return call('save_settings', { settings })
}

export function setAutostart(enabled: boolean): Promise<SettingsState> {
  return call('set_autostart', { enabled })
}

export function openCodexDirectory(): Promise<void> {
  return call('open_codex_directory')
}

export function exitApplication(): Promise<void> {
  return call('exit_application')
}

export function listBackups(): Promise<BackupSummary[]> {
  return call('list_backups')
}

export function restoreBackup(directoryName: string): Promise<ProviderMutationOutcome> {
  return call('restore_backup', { directoryName })
}

export function runCriticalSelfCheck(): Promise<HealthReport> {
  return call('run_critical_self_check')
}

export function runExtendedSelfCheck(): Promise<HealthReport> {
  return call('run_extended_self_check')
}

export function getCurrentVersion(): Promise<string> {
  return getVersion()
}

export async function checkForUpdate(): Promise<UpdateSession | null> {
  let update
  try {
    update = await check()
  } catch {
    throw new RelayCommandError('UPDATE_CHECK_FAILED', '检查更新失败，请稍后重试。')
  }
  if (!update) {
    return null
  }

  const date = update.date && !Number.isNaN(Date.parse(update.date)) ? update.date : null

  return {
    info: {
      currentVersion: update.currentVersion,
      version: update.version,
      date,
      notes: update.body ?? null,
    },
    async downloadAndInstall(onProgress) {
      let downloadedBytes = 0
      let totalBytes: number | null = null

      const report = () => {
        const progress: UpdateProgress = {
          downloadedBytes,
          totalBytes,
          percent: totalBytes === null ? null : (downloadedBytes / totalBytes) * 100,
        }
        onProgress(progress)
      }

      try {
        await update.downloadAndInstall((event: DownloadEvent) => {
          if (event.event === 'Started') {
            totalBytes = event.data.contentLength ?? null
            report()
            return
          }
          if (event.event === 'Progress') {
            downloadedBytes += event.data.chunkLength
            report()
            return
          }
          report()
        })
      } catch {
        throw new RelayCommandError(
          'UPDATE_INSTALL_FAILED',
          '下载或安装更新失败，请稍后重试。',
        )
      }
    },
    close: () => update.close(),
  }
}

export function onProvidersChanged(
  handler: (payload: ProviderListState) => void,
): Promise<UnlistenFn> {
  return subscribe(PROVIDERS_CHANGED_EVENT, handler)
}

export function onSelfCheckCompleted(
  handler: (payload: HealthReport) => void,
): Promise<UnlistenFn> {
  return subscribe(SELF_CHECK_COMPLETED_EVENT, handler)
}

export function onSettingsChanged(handler: (payload: SettingsState) => void): Promise<UnlistenFn> {
  return subscribe(SETTINGS_CHANGED_EVENT, handler)
}

export interface AppNotification {
  level: 'success' | 'error'
  message: string
}

export function onAppNotification(
  handler: (payload: AppNotification) => void,
): Promise<UnlistenFn> {
  return subscribe(APP_NOTIFICATION_EVENT, handler)
}
