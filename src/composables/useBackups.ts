import { readonly, ref, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { BackupFileName, BackupInventory, BackupSummary, UnavailableBackup } from '../types/backup'
import type { RelayUiError } from '../types/command'
import type { ProviderMutationOutcome } from '../types/provider'

export interface BackupClient {
  listBackups(): Promise<BackupInventory>
  openBackupFile(directoryName: string, fileName: BackupFileName): Promise<void>
  restoreBackup(directoryName: string): Promise<ProviderMutationOutcome>
}

export interface UseBackupsOptions {
  client?: BackupClient
}

const defaultClient: BackupClient = {
  listBackups: relay.listBackups,
  openBackupFile: relay.openBackupFile,
  restoreBackup: relay.restoreBackup,
}

export function useBackups(options: UseBackupsOptions = {}) {
  const client = options.client ?? defaultClient
  const backupList = ref<BackupSummary[]>([])
  const unavailableBackupList = ref<UnavailableBackup[]>([])
  const loading = shallowRef(true)
  const busy = shallowRef(false)
  const error = shallowRef<RelayUiError | null>(null)
  const successMessage = shallowRef<string | null>(null)
  let requestSequence = 0

  function setError(caught: unknown) {
    error.value = caught instanceof relay.RelayCommandError
      ? { code: caught.code, message: caught.message }
      : { code: 'UNEXPECTED_ERROR', message: '备份操作失败，请重试。' }
  }

  async function refresh(): Promise<boolean> {
    const request = ++requestSequence
    loading.value = true
    error.value = null
    try {
      const next = await client.listBackups()
      if (request === requestSequence) {
        backupList.value = [...next.backups]
        unavailableBackupList.value = [...next.unavailableBackups]
      }
      return true
    } catch (caught) {
      if (request === requestSequence) {
        setError(caught)
        return false
      }
      return true
    } finally {
      if (request === requestSequence) {
        loading.value = false
      }
    }
  }

  async function restore(directoryName: string) {
    if (busy.value) return
    busy.value = true
    error.value = null
    successMessage.value = null
    try {
      const outcome = await client.restoreBackup(directoryName)
      const refreshed = await refresh()
      if (refreshed) {
        successMessage.value = outcome.message
      }
    } catch (caught) {
      setError(caught)
    } finally {
      busy.value = false
    }
  }

  async function openFile(directoryName: string, fileName: BackupFileName) {
    if (busy.value) return
    busy.value = true
    error.value = null
    successMessage.value = null
    try {
      await client.openBackupFile(directoryName, fileName)
    } catch (caught) {
      setError(caught)
    } finally {
      busy.value = false
    }
  }

  void refresh()

  return {
    backups: readonly(backupList),
    unavailableBackups: readonly(unavailableBackupList),
    loading: readonly(loading),
    busy: readonly(busy),
    error: readonly(error),
    successMessage: readonly(successMessage),
    refresh,
    openFile,
    restore,
  }
}
