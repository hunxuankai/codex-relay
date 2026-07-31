import { readonly, shallowRef } from 'vue'
import {
  releaseConsoleTauri,
  type ReleaseConsoleClient,
} from '../services/tauri'
import type {
  CommandError,
  DraftIdentity,
  ReleaseEvent,
  ReleasePlanSummary,
  ReleasePreflightResult,
  ReleaseSession,
} from '../types/release'

interface UseReleaseSessionOptions {
  client?: ReleaseConsoleClient
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
  const busy = shallowRef(false)
  const error = shallowRef<CommandError | null>(null)
  let operationSequence = 0
  let channelGeneration = 0

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
    return (event: ReleaseEvent) => {
      if (generation !== channelGeneration) return
      events.value = [...events.value, event]
      if (event.kind === 'sessionUpdated') {
        session.value = event.session
      }
    }
  }

  async function inspect(repositoryPath: string) {
    const sequence = beginOperation()
    try {
      const value = await client.inspectRepository(repositoryPath)
      if (sequence === operationSequence) inspection.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function preparePlan(repositoryPath: string, targetVersion: string, notes?: string) {
    const sequence = beginOperation()
    try {
      const value = await client.preparePlan(repositoryPath, targetVersion, notes)
      if (sequence === operationSequence) plan.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function start(planId: string) {
    const sequence = beginOperation()
    const onEvent = openEventStream()
    events.value = []
    try {
      const value = await client.startRelease(planId, onEvent)
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
    try {
      const value = await client.getReleaseSession(repositoryPath)
      if (sequence === operationSequence) session.value = value
      return value
    } catch (cause) {
      recordError(sequence, cause)
      return null
    } finally {
      finishOperation(sequence)
    }
  }

  async function resume(sessionId: string) {
    const sequence = beginOperation()
    const onEvent = openEventStream()
    try {
      const value = await client.resumeRelease(sessionId, onEvent)
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

  async function publish(sessionId: string, expectedDraftIdentity: DraftIdentity) {
    const sequence = beginOperation()
    const onEvent = openEventStream()
    try {
      const value = await client.publishRelease(sessionId, expectedDraftIdentity, onEvent)
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

  return {
    inspection: readonly(inspection),
    plan: readonly(plan),
    session: readonly(session),
    events: readonly(events),
    busy: readonly(busy),
    error: readonly(error),
    inspect,
    preparePlan,
    start,
    load,
    resume,
    cancel,
    publish,
    exportSummary,
  }
}
