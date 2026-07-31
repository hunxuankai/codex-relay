import { Channel, invoke } from '@tauri-apps/api/core'
import type {
  CommandResult,
  DraftIdentity,
  ReleaseEvent,
  ReleasePlanSummary,
  ReleasePreflightResult,
  ReleaseSession,
} from '../types/release'

export class ReleaseConsoleError extends Error {
  readonly code: string

  constructor(code: string, message: string) {
    super(message)
    this.name = 'ReleaseConsoleError'
    this.code = code
  }
}

export interface ReleaseConsoleClient {
  inspectRepository(repositoryPath: string): Promise<ReleasePreflightResult>
  preparePlan(
    repositoryPath: string,
    targetVersion: string,
    notes?: string,
  ): Promise<ReleasePlanSummary>
  startRelease(planId: string, onEvent: (event: ReleaseEvent) => void): Promise<ReleaseSession>
  getReleaseSession(repositoryPath: string): Promise<ReleaseSession | null>
  resumeRelease(sessionId: string, onEvent: (event: ReleaseEvent) => void): Promise<ReleaseSession>
  cancelRelease(sessionId: string): Promise<ReleaseSession>
  publishRelease(
    sessionId: string,
    expectedDraftIdentity: DraftIdentity,
    onEvent: (event: ReleaseEvent) => void,
  ): Promise<ReleaseSession>
  exportSummary(sessionId: string, destinationPath: string): Promise<string>
}

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const result = await invoke<CommandResult<T>>(command, args)
  if (result.success && result.data !== undefined) {
    return result.data
  }
  throw new ReleaseConsoleError(
    result.error?.code ?? 'RELEASE_COMMAND_FAILED',
    result.error?.message ?? '发布控制台操作失败。',
  )
}

function eventChannel(onEvent: (event: ReleaseEvent) => void): Channel<ReleaseEvent> {
  const channel = new Channel<ReleaseEvent>()
  channel.onmessage = onEvent
  return channel
}

export const releaseConsoleTauri: ReleaseConsoleClient = {
  inspectRepository(repositoryPath: string) {
    return invokeCommand<ReleasePreflightResult>('inspect_release_repository', { repositoryPath })
  },

  preparePlan(repositoryPath: string, targetVersion: string, notes?: string) {
    return invokeCommand<ReleasePlanSummary>('prepare_release_plan', {
      repositoryPath,
      targetVersion,
      notes: notes ?? null,
    })
  },

  startRelease(planId: string, onEvent: (event: ReleaseEvent) => void) {
    return invokeCommand<ReleaseSession>('start_release', {
      planId,
      onEvent: eventChannel(onEvent),
    })
  },

  getReleaseSession(repositoryPath: string) {
    return invokeCommand<ReleaseSession | null>('get_release_session', { repositoryPath })
  },

  resumeRelease(sessionId: string, onEvent: (event: ReleaseEvent) => void) {
    return invokeCommand<ReleaseSession>('resume_release', {
      sessionId,
      onEvent: eventChannel(onEvent),
    })
  },

  cancelRelease(sessionId: string) {
    return invokeCommand<ReleaseSession>('cancel_release', { sessionId })
  },

  publishRelease(
    sessionId: string,
    expectedDraftIdentity: DraftIdentity,
    onEvent: (event: ReleaseEvent) => void,
  ) {
    return invokeCommand<ReleaseSession>('publish_release', {
      sessionId,
      expectedDraftIdentity,
      onEvent: eventChannel(onEvent),
    })
  },

  exportSummary(sessionId: string, destinationPath: string) {
    return invokeCommand<string>('export_release_summary', { sessionId, destinationPath })
  },
}
