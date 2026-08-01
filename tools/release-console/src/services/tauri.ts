import { Channel, invoke } from '@tauri-apps/api/core'
import type {
  ReleaseConnectionTestResult,
  ReleaseProxySettings,
} from '../types/network'
import type {
  CommandResult,
  DraftIdentity,
  ReleaseEvent,
  ReleasePlanSummary,
  ReleasePreflightResult,
  ReleaseSession,
  SafeRepositoryPushRequest,
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
  testConnection(proxy: ReleaseProxySettings): Promise<ReleaseConnectionTestResult>
  inspectRepository(
    repositoryPath: string,
    proxy: ReleaseProxySettings,
  ): Promise<ReleasePreflightResult>
  pushRepository(request: SafeRepositoryPushRequest): Promise<ReleasePreflightResult>
  preparePlan(
    repositoryPath: string,
    targetVersion: string,
    proxy: ReleaseProxySettings,
    notes?: string,
  ): Promise<ReleasePlanSummary>
  startRelease(
    planId: string,
    proxy: ReleaseProxySettings,
    onEvent: (event: ReleaseEvent) => void,
  ): Promise<ReleaseSession>
  getReleaseSession(repositoryPath: string): Promise<ReleaseSession | null>
  resumeRelease(
    sessionId: string,
    proxy: ReleaseProxySettings,
    onEvent: (event: ReleaseEvent) => void,
  ): Promise<ReleaseSession>
  cancelRelease(sessionId: string): Promise<ReleaseSession>
  publishRelease(
    sessionId: string,
    expectedDraftIdentity: DraftIdentity,
    proxy: ReleaseProxySettings,
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
  testConnection(proxy: ReleaseProxySettings) {
    return invokeCommand<ReleaseConnectionTestResult>('test_release_connection', { proxy })
  },

  inspectRepository(repositoryPath: string, proxy: ReleaseProxySettings) {
    return invokeCommand<ReleasePreflightResult>('inspect_release_repository', {
      repositoryPath,
      proxy,
    })
  },

  pushRepository(request: SafeRepositoryPushRequest) {
    return invokeCommand<ReleasePreflightResult>('push_release_repository', { request })
  },

  preparePlan(
    repositoryPath: string,
    targetVersion: string,
    proxy: ReleaseProxySettings,
    notes?: string,
  ) {
    return invokeCommand<ReleasePlanSummary>('prepare_release_plan', {
      repositoryPath,
      targetVersion,
      proxy,
      notes: notes ?? null,
    })
  },

  startRelease(
    planId: string,
    proxy: ReleaseProxySettings,
    onEvent: (event: ReleaseEvent) => void,
  ) {
    return invokeCommand<ReleaseSession>('start_release', {
      planId,
      proxy,
      onEvent: eventChannel(onEvent),
    })
  },

  getReleaseSession(repositoryPath: string) {
    return invokeCommand<ReleaseSession | null>('get_release_session', { repositoryPath })
  },

  resumeRelease(
    sessionId: string,
    proxy: ReleaseProxySettings,
    onEvent: (event: ReleaseEvent) => void,
  ) {
    return invokeCommand<ReleaseSession>('resume_release', {
      sessionId,
      proxy,
      onEvent: eventChannel(onEvent),
    })
  },

  cancelRelease(sessionId: string) {
    return invokeCommand<ReleaseSession>('cancel_release', { sessionId })
  },

  publishRelease(
    sessionId: string,
    expectedDraftIdentity: DraftIdentity,
    proxy: ReleaseProxySettings,
    onEvent: (event: ReleaseEvent) => void,
  ) {
    return invokeCommand<ReleaseSession>('publish_release', {
      sessionId,
      expectedDraftIdentity,
      proxy,
      onEvent: eventChannel(onEvent),
    })
  },

  exportSummary(sessionId: string, destinationPath: string) {
    return invokeCommand<string>('export_release_summary', { sessionId, destinationPath })
  },
}
