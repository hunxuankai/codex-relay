import { getCurrentScope, onScopeDispose, readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'
import type {
  UpdateClient,
  UpdateProgress,
  UpdateReleaseInfo,
  UpdateSession,
} from '../types/update'

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'upToDate'
  | 'available'
  | 'confirming'
  | 'downloading'
  | 'launching'
  | 'error'

export interface UseUpdaterOptions {
  client?: UpdateClient
}

const defaultClient: UpdateClient = {
  getCurrentVersion: relay.getCurrentVersion,
  checkForUpdate: relay.checkForUpdate,
}

export function useUpdater(options: UseUpdaterOptions = {}) {
  const client = options.client ?? defaultClient
  const status = shallowRef<UpdateStatus>('idle')
  const currentVersion = shallowRef<string | null>(null)
  const release = shallowRef<UpdateReleaseInfo | null>(null)
  const error = shallowRef<RelayUiError | null>(null)
  const progress = shallowRef<UpdateProgress | null>(null)
  const session = shallowRef<UpdateSession | null>(null)
  let requestSequence = 0
  const currentVersionPromise = client
    .getCurrentVersion()
    .then((version) => {
      currentVersion.value = version
      return version
    })
    .catch(() => null)
  void currentVersionPromise

  function setError(caught: unknown) {
    error.value = caught instanceof relay.RelayCommandError
      ? { code: caught.code, message: caught.message }
      : { code: 'UPDATE_CHECK_FAILED', message: '检查更新失败，请稍后重试。' }
    status.value = 'error'
  }

  async function closeSession(target: UpdateSession | null) {
    if (!target) return
    await target.close().catch(() => undefined)
  }

  async function check() {
    if (status.value === 'checking' || status.value === 'downloading' || status.value === 'launching') {
      return
    }
    const sequence = ++requestSequence
    const previousSession = session.value
    session.value = null
    release.value = null
    error.value = null
    progress.value = null
    status.value = 'checking'
    await closeSession(previousSession)

    try {
      const version = await currentVersionPromise
      if (sequence !== requestSequence) return
      if (version) currentVersion.value = version

      const nextSession = await client.checkForUpdate()
      if (sequence !== requestSequence) {
        await closeSession(nextSession)
        return
      }
      session.value = nextSession
      release.value = nextSession?.info ?? null
      status.value = nextSession ? 'available' : 'upToDate'
    } catch (caught) {
      if (sequence === requestSequence) {
        setError(caught)
      }
    }
  }

  function reset() {
    requestSequence += 1
    const previousSession = session.value
    session.value = null
    release.value = null
    error.value = null
    progress.value = null
    status.value = 'idle'
    void closeSession(previousSession)
  }

  function requestInstall() {
    if (status.value === 'available' && session.value) {
      status.value = 'confirming'
    }
  }

  function cancelInstall() {
    if (status.value === 'confirming') {
      status.value = 'available'
    }
  }

  async function confirmInstall() {
    const activeSession = session.value
    if (status.value !== 'confirming' || !activeSession) return
    status.value = 'downloading'
    progress.value = null
    error.value = null
    try {
      await activeSession.downloadAndInstall((nextProgress) => {
        progress.value = nextProgress
      })
      status.value = 'launching'
    } catch (caught) {
      setError(caught)
    }
  }

  if (getCurrentScope()) {
    onScopeDispose(reset)
  }

  return {
    status: readonly(status),
    currentVersion: readonly(currentVersion),
    release: readonly(release),
    error: readonly(error),
    progress: readonly(progress),
    check,
    reset,
    requestInstall,
    cancelInstall,
    confirmInstall,
  }
}
