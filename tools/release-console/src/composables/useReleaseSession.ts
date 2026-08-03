import { readonly, shallowRef } from 'vue'
import {
  releaseConsoleTauri,
  type ReleaseConsoleClient,
} from '../services/tauri'
import type { ReleaseProxySettings } from '../types/network'
import type {
  CommandError,
  DraftIdentity,
  ReleaseEvent,
  ReleaseLogEntry,
  ReleaseLogPage,
  ReleaseLogViewMode,
  ReleasePlanSummary,
  ReleasePreflightResult,
  ReleaseSession,
} from '../types/release'

const LOG_PAGE_SIZE = 2_000

function emptyLogPage(): ReleaseLogPage {
  return {
    entries: [],
    nextBeforeSequence: null,
    hasEarlier: false,
    totalEntries: 0,
    totalBytes: 0,
    truncated: false,
    warning: null,
  }
}

interface UseReleaseSessionOptions {
  client?: Omit<ReleaseConsoleClient, 'testConnection'>
}
function safeError(error: unknown): CommandError {
  if (typeof error === 'object' && error !== null) {
    const code = 'code' in error ? error.code : undefined
    const message = 'message' in error ? error.message : undefined
    if (typeof code === 'string' && typeof message === 'string') {
      return { code, message }
    }
  }
  return {
    code: 'RELEASE_COMMAND_FAILED',
    message: '发布控制台操作失败。',
  }
}

export function useReleaseSession(options: UseReleaseSessionOptions = {}) {
  const client = options.client ?? releaseConsoleTauri
  const inspection = shallowRef<ReleasePreflightResult | null>(null)
  const plan = shallowRef<ReleasePlanSummary | null>(null)
  const session = shallowRef<ReleaseSession | null>(null)
  const events = shallowRef<ReleaseEvent[]>([])
  const logPage = shallowRef<ReleaseLogPage>(emptyLogPage())
  const latestLogPage = shallowRef<ReleaseLogPage>(emptyLogPage())
  const logViewMode = shallowRef<ReleaseLogViewMode>('latest')
  const unreadLogCount = shallowRef(0)
  const logRequestPending = shallowRef(false)
  const logError = shallowRef<CommandError | null>(null)
  const busy = shallowRef(false)
  const error = shallowRef<CommandError | null>(null)
  let operationSequence = 0
  let channelGeneration = 0
  let logRequestSequence = 0
  let historyBeforeSequence: number | null = null

  function beginOperation() {
    const sequence = ++operationSequence
    busy.value = true
    error.value = null
    return sequence
  }

  function finishOperation(sequence: number) {
    if (sequence === operationSequence) {
      busy.value = false
    }
  }

  function recordError(sequence: number, cause: unknown) {
    if (sequence === operationSequence) {
      error.value = safeError(cause)
    }
  }

  function openEventStream() {
    const generation = ++channelGeneration
    invalidateLogRequests()
    return (event: ReleaseEvent) => {
      if (generation !== channelGeneration) return
      if (event.kind === 'stepLog') {
        appendRealtimeLog(event.entry, event.page)
        return
      }
      events.value = [...events.value, event]
      if (event.kind === 'sessionUpdated') {
        session.value = event.session
      }
    }
  }

  function appendRealtimeLog(entry: ReleaseLogEntry, authoritativePage?: ReleaseLogPage) {
    if (session.value?.id !== entry.sessionId) return
    if (authoritativePage) {
      if (authoritativePage.entries.some((item) => item.sessionId !== entry.sessionId)) return
      invalidateLogRequests()
      latestLogPage.value = authoritativePage
      if (logViewMode.value === 'latest') {
        logPage.value = authoritativePage
      } else {
        logPage.value = {
          ...logPage.value,
          totalEntries: authoritativePage.totalEntries,
          totalBytes: authoritativePage.totalBytes,
          truncated: authoritativePage.truncated || logPage.value.truncated,
          warning: authoritativePage.warning ?? logPage.value.warning,
        }
        unreadLogCount.value += 1
      }
      return
    }
    const current = latestLogPage.value
    const lastEntry = current.entries[current.entries.length - 1]
    if (lastEntry && entry.sequence <= lastEntry.sequence) return
    const appended = [...current.entries, entry]
    const overflowed = appended.length > LOG_PAGE_SIZE
    const entries = overflowed ? appended.slice(-LOG_PAGE_SIZE) : appended
    const hasEarlier = current.hasEarlier || overflowed
    const nextPage = {
      ...current,
      entries,
      nextBeforeSequence: hasEarlier ? (entries[0]?.sequence ?? null) : null,
      hasEarlier,
      totalEntries: current.totalEntries + 1,
    }
    latestLogPage.value = nextPage
    if (logViewMode.value === 'latest') {
      logPage.value = nextPage
    } else {
      unreadLogCount.value += 1
    }
  }

  function resetLogs() {
    invalidateLogRequests()
    const empty = emptyLogPage()
    logPage.value = empty
    latestLogPage.value = empty
    logViewMode.value = 'latest'
    unreadLogCount.value = 0
    logError.value = null
    historyBeforeSequence = null
  }

  function invalidateLogRequests() {
    logRequestSequence += 1
    logRequestPending.value = false
  }

  async function requestLogPage(
    beforeSequence: number | null,
    viewMode: ReleaseLogViewMode,
  ) {
    const sessionId = session.value?.id
    if (!sessionId) return null
    const sequence = ++logRequestSequence
    logRequestPending.value = true
    logError.value = null
    try {
      const value = await client.getReleaseLogs(sessionId, beforeSequence)
      if (sequence !== logRequestSequence || session.value?.id !== sessionId) return null
      if (viewMode === 'latest') {
        const reconciled = reconcileLatestLogPage(value, sessionId)
        latestLogPage.value = reconciled
        logPage.value = reconciled
        logViewMode.value = 'latest'
        unreadLogCount.value = 0
        historyBeforeSequence = null
      } else {
        logPage.value = value
        logViewMode.value = 'history'
        historyBeforeSequence = beforeSequence
      }
      return value
    } catch (cause) {
      if (sequence === logRequestSequence && session.value?.id === sessionId) {
        logError.value = safeError(cause)
      }
      return null
    } finally {
      if (sequence === logRequestSequence) {
        logRequestPending.value = false
      }
    }
  }

  function loadEarlierLogs() {
    const beforeSequence = logPage.value.nextBeforeSequence
    if (!logPage.value.hasEarlier || beforeSequence === null) return Promise.resolve(null)
    return requestLogPage(beforeSequence, 'history')
  }

  function refreshLogPage() {
    return requestLogPage(
      logViewMode.value === 'history' ? historyBeforeSequence : null,
      logViewMode.value,
    )
  }

  function returnToLatestLogs() {
    return requestLogPage(null, 'latest')
  }

  function reconcileLatestLogPage(loaded: ReleaseLogPage, sessionId: string): ReleaseLogPage {
    const current = latestLogPage.value
    const loadedLast = loaded.entries[loaded.entries.length - 1]?.sequence
    const realtimeEntries = current.entries.filter(
      (entry) =>
        entry.sessionId === sessionId
        && (loadedLast === undefined || entry.sequence > loadedLast),
    )
    const combined = [...loaded.entries, ...realtimeEntries]
    const overflowed = combined.length > LOG_PAGE_SIZE
    const entries = overflowed ? combined.slice(-LOG_PAGE_SIZE) : combined
    const hasEarlier = loaded.hasEarlier || overflowed
    return {
      ...loaded,
      entries,
      nextBeforeSequence: hasEarlier
        ? (entries[0]?.sequence ?? loaded.nextBeforeSequence)
        : null,
      hasEarlier,
      totalEntries: Math.max(loaded.totalEntries, current.totalEntries),
      totalBytes: Math.max(loaded.totalBytes, current.totalBytes),
      truncated: loaded.truncated || current.truncated,
      warning: loaded.warning ?? current.warning,
    }
  }

  async function inspect(repositoryPath: string, proxy: ReleaseProxySettings) {
    const sequence = beginOperation()
    try {
      const value = await client.inspectRepository(repositoryPath, proxy)
      if (sequence === operationSequence) inspection.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function preparePlan(
    repositoryPath: string,
    targetVersion: string,
    proxy: ReleaseProxySettings,
    notes?: string,
  ) {
    const sequence = beginOperation()
    try {
      const value = await client.preparePlan(repositoryPath, targetVersion, proxy, notes)
      if (sequence === operationSequence) plan.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function pushRepository(proxy: ReleaseProxySettings) {
    const currentInspection = inspection.value
    const preview = currentInspection?.safePush
    if (!currentInspection || !preview) {
      error.value = {
        code: 'GIT_SAFE_PUSH_FORBIDDEN',
        message: '当前仓库状态不允许安全推送。',
      }
      return null
    }
    const sequence = beginOperation()
    try {
      const value = await client.pushRepository({
        repositoryPath: currentInspection.repositoryPath,
        expectedHeadSha: preview.expectedHeadSha,
        expectedRemoteMainSha: preview.expectedRemoteMainSha,
        proxy,
      })
      if (sequence === operationSequence) {
        inspection.value = value
        plan.value = null
      }
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function start(planId: string, proxy: ReleaseProxySettings) {
    const sequence = beginOperation()
    const onEvent = openEventStream()
    events.value = []
    resetLogs()
    try {
      const value = await client.startRelease(planId, proxy, onEvent)
      if (sequence === operationSequence) session.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function load(repositoryPath: string) {
    const sequence = beginOperation()
    channelGeneration += 1
    events.value = []
    resetLogs()
    try {
      const value = await client.getReleaseSession(repositoryPath)
      if (sequence === operationSequence) {
        session.value = value?.session ?? null
        const logs = value?.logs ?? emptyLogPage()
        logPage.value = logs
        latestLogPage.value = logs
      }
      return value?.session ?? null
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function resume(sessionId: string, proxy: ReleaseProxySettings) {
    const sequence = beginOperation()
    const onEvent = openEventStream()
    try {
      const value = await client.resumeRelease(sessionId, proxy, onEvent)
      if (sequence === operationSequence) session.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function cancel(sessionId: string) {
    const sequence = beginOperation()
    channelGeneration += 1
    try {
      const value = await client.cancelRelease(sessionId)
      if (sequence === operationSequence) session.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function publish(
    sessionId: string,
    expectedDraftIdentity: DraftIdentity,
    proxy: ReleaseProxySettings,
  ) {
    const sequence = beginOperation()
    const onEvent = openEventStream()
    try {
      const value = await client.publishRelease(
        sessionId,
        expectedDraftIdentity,
        proxy,
        onEvent,
      )
      if (sequence === operationSequence) session.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function exportSummary(sessionId: string, destinationPath: string) {
    const sequence = beginOperation()
    try {
      return await client.exportSummary(sessionId, destinationPath)
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  function invalidateRepositoryContext() {
    inspection.value = null
    plan.value = null
  }

  return {
    inspection: readonly(inspection),
    plan: readonly(plan),
    session: readonly(session),
    events: readonly(events),
    logPage: readonly(logPage),
    logViewMode: readonly(logViewMode),
    unreadLogCount: readonly(unreadLogCount),
    logRequestPending: readonly(logRequestPending),
    logError: readonly(logError),
    busy: readonly(busy),
    error: readonly(error),
    inspect,
    pushRepository,
    preparePlan,
    start,
    load,
    resume,
    cancel,
    publish,
    exportSummary,
    loadEarlierLogs,
    refreshLogPage,
    returnToLatestLogs,
    invalidateRepositoryContext,
  }
}
