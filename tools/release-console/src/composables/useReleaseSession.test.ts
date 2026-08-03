import { describe, expect, it, vi } from 'vitest'
import type {
  DraftIdentity,
  ReleaseEvent,
  ReleaseLogEntry,
  ReleaseLogPage,
  ReleasePlanSummary,
  ReleasePreflightResult,
  ReleaseSession,
} from '../types/release'
import { useReleaseSession } from './useReleaseSession'

const directProxy = { enabled: false, proxyType: 'http' as const, host: '', port: null }

function session(id: string, phase: ReleaseSession['phase']): ReleaseSession {
  return {
    id,
    repositoryPath: 'D:\\safe-temp\\repository',
    targetVersion: '0.5.0',
    phase,
    candidateSha: 'a'.repeat(40),
    remoteMainSha: 'a'.repeat(40),
    workflow: null,
    draft: null,
    published: null,
    cleanup: null,
    cleanupWarning: null,
    failure: null,
  }
}

function logEntry(sequence: number, sessionId = 'session-logs'): ReleaseLogEntry {
  return {
    sessionId,
    sequence,
    timestamp: `2026-08-03T12:00:${String(sequence % 60).padStart(2, '0')}.000Z`,
    stepId: 'full-project-check',
    source: 'stdout',
    level: 'info',
    message: `诊断记录 ${sequence}`,
  }
}

function logPage(entries: readonly ReleaseLogEntry[]): ReleaseLogPage {
  return {
    entries,
    nextBeforeSequence: null,
    hasEarlier: false,
    totalEntries: entries.length,
    totalBytes: entries.length * 128,
    truncated: false,
    warning: null,
  }
}

describe('useReleaseSession', () => {
  it('loads the latest log page and keeps realtime step logs bounded and out of events', async () => {
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    const persisted = session('session-logs', 'workflowRunning')
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn().mockResolvedValue({
        session: persisted,
        logs: logPage([logEntry(1)]),
      }),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return persisted
        },
      ),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(persisted.repositoryPath)
    await release.resume(persisted.id, directProxy)
    resumeEvent?.({ kind: 'stepLog', entry: logEntry(1) })
    for (let sequence = 2; sequence <= 100_000; sequence += 1) {
      resumeEvent?.({ kind: 'stepLog', entry: logEntry(sequence) })
    }

    expect(release.session.value).toEqual(persisted)
    expect(release.logPage.value.entries).toHaveLength(2_000)
    expect(release.logPage.value.entries[0]?.sequence).toBe(98_001)
    expect(
      release.logPage.value.entries[release.logPage.value.entries.length - 1]?.sequence,
    ).toBe(100_000)
    expect(release.logPage.value.totalEntries).toBe(100_000)
    expect(release.events.value).toEqual([])
  })

  it('applies an authoritative compacted page from the realtime log event', async () => {
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    const persisted = session('session-logs', 'workflowRunning')
    const marker: ReleaseLogEntry = {
      ...logEntry(2),
      source: 'lifecycle',
      level: 'warning',
      message: '早期普通输出已截断',
    }
    const failure: ReleaseLogEntry = {
      ...logEntry(4),
      source: 'stderr',
      level: 'error',
      message: 'latest failure',
    }
    const compacted: ReleaseLogPage = {
      ...logPage([marker, failure]),
      totalBytes: 512,
      truncated: true,
      warning: marker.message,
    }
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn().mockResolvedValue({
        session: persisted,
        logs: logPage([logEntry(1), logEntry(2), logEntry(3)]),
      }),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return persisted
        },
      ),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(persisted.repositoryPath)
    await release.resume(persisted.id, directProxy)
    resumeEvent?.({ kind: 'stepLog', entry: failure, page: compacted })

    expect(release.logPage.value).toEqual(compacted)
    expect(release.events.value).toEqual([])
  })

  it('keeps history stable while realtime logs arrive and returns to the latest page', async () => {
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    const persisted = session('session-logs', 'workflowRunning')
    const latest: ReleaseLogPage = {
      ...logPage([logEntry(2_001), logEntry(2_002)]),
      nextBeforeSequence: 2_001,
      hasEarlier: true,
      totalEntries: 2_002,
    }
    const history: ReleaseLogPage = {
      ...logPage([logEntry(1), logEntry(2)]),
      totalEntries: 2_002,
      truncated: true,
      warning: '早期普通输出已截断。',
    }
    const refreshedLatest: ReleaseLogPage = {
      ...logPage([logEntry(2_001), logEntry(2_002), logEntry(2_003)]),
      nextBeforeSequence: 2_001,
      hasEarlier: true,
      totalEntries: 1_600,
      totalBytes: 256_000,
      truncated: true,
      warning: '早期普通输出已截断。',
    }
    const getReleaseLogs = vi
      .fn()
      .mockResolvedValueOnce(history)
      .mockResolvedValueOnce(refreshedLatest)
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn().mockResolvedValue({ session: persisted, logs: latest }),
      getReleaseLogs,
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return persisted
        },
      ),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(persisted.repositoryPath)
    await release.resume(persisted.id, directProxy)
    await release.loadEarlierLogs()
    expect(release.logViewMode.value).toBe('history')
    expect(release.logPage.value).toEqual(history)

    resumeEvent?.({
      kind: 'stepLog',
      entry: logEntry(2_003),
      page: refreshedLatest,
    })
    expect(release.logPage.value.entries).toEqual(history.entries)
    expect(release.logPage.value.totalEntries).toBe(1_600)
    expect(release.logPage.value.totalBytes).toBe(256_000)
    expect(release.logPage.value.truncated).toBe(true)
    expect(release.logPage.value.warning).toBe('早期普通输出已截断。')
    expect(release.unreadLogCount.value).toBe(1)

    await release.returnToLatestLogs()
    expect(release.logViewMode.value).toBe('latest')
    expect(release.logPage.value).toEqual(refreshedLatest)
    expect(release.unreadLogCount.value).toBe(0)
    expect(release.logRequestPending.value).toBe(false)
    expect(release.logError.value).toBeNull()
    expect(getReleaseLogs.mock.calls).toEqual([
      [persisted.id, 2_001],
      [persisted.id, null],
    ])
  })

  it('drops stale pagination responses and channel logs after loading another repository', async () => {
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    let resolveOldPage: ((page: ReleaseLogPage) => void) | undefined
    const first = {
      ...session('session-first', 'workflowRunning'),
      repositoryPath: 'D:\\safe-temp\\repository-first',
    }
    const second = {
      ...session('session-second', 'failed'),
      repositoryPath: 'D:\\safe-temp\\repository-second',
    }
    const firstPage: ReleaseLogPage = {
      ...logPage([logEntry(100, first.id)]),
      hasEarlier: true,
      nextBeforeSequence: 100,
      totalEntries: 100,
    }
    const secondPage = logPage([logEntry(1, second.id)])
    const oldPagePromise = new Promise<ReleaseLogPage>((resolve) => {
      resolveOldPage = resolve
    })
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi
        .fn()
        .mockResolvedValueOnce({ session: first, logs: firstPage })
        .mockResolvedValueOnce({ session: second, logs: secondPage }),
      getReleaseLogs: vi.fn().mockReturnValue(oldPagePromise),
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return first
        },
      ),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(first.repositoryPath)
    await release.resume(first.id, directProxy)
    const oldRequest = release.loadEarlierLogs()
    await release.load(second.repositoryPath)
    resumeEvent?.({ kind: 'stepLog', entry: logEntry(101, first.id) })
    resolveOldPage?.(logPage([logEntry(1, first.id)]))
    await oldRequest

    expect(release.session.value).toEqual(second)
    expect(release.logPage.value).toEqual(secondPage)
    expect(release.logViewMode.value).toBe('latest')
    expect(release.unreadLogCount.value).toBe(0)
    expect(release.logRequestPending.value).toBe(false)
  })

  it('refreshes the current history cursor and keeps the page when log loading fails', async () => {
    const persisted = session('session-history-error', 'failed')
    const latest: ReleaseLogPage = {
      ...logPage([logEntry(100, persisted.id)]),
      hasEarlier: true,
      nextBeforeSequence: 100,
      totalEntries: 100,
    }
    const history = logPage([logEntry(1, persisted.id)])
    const getReleaseLogs = vi
      .fn()
      .mockResolvedValueOnce(history)
      .mockRejectedValueOnce({
        code: 'RELEASE_LOG_READ_FAILED',
        message: '发布日志暂时无法读取。',
      })
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn().mockResolvedValue({ session: persisted, logs: latest }),
      getReleaseLogs,
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(persisted.repositoryPath)
    await release.loadEarlierLogs()
    await release.refreshLogPage()

    expect(getReleaseLogs.mock.calls).toEqual([
      [persisted.id, 100],
      [persisted.id, 100],
    ])
    expect(release.logPage.value).toEqual(history)
    expect(release.logViewMode.value).toBe('history')
    expect(release.logRequestPending.value).toBe(false)
    expect(release.logError.value).toEqual({
      code: 'RELEASE_LOG_READ_FAILED',
      message: '发布日志暂时无法读取。',
    })
    expect(release.error.value).toBeNull()
  })

  it('does not let a latest-page response overwrite realtime entries received in flight', async () => {
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    let resolvePage: ((page: ReleaseLogPage) => void) | undefined
    const persisted = session('session-inflight-log', 'workflowRunning')
    const initial = logPage([logEntry(100, persisted.id)])
    const response = new Promise<ReleaseLogPage>((resolve) => {
      resolvePage = resolve
    })
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn().mockResolvedValue({ session: persisted, logs: initial }),
      getReleaseLogs: vi.fn().mockReturnValue(response),
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return persisted
        },
      ),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(persisted.repositoryPath)
    await release.resume(persisted.id, directProxy)
    const refresh = release.refreshLogPage()
    resumeEvent?.({ kind: 'stepLog', entry: logEntry(101, persisted.id) })
    resolvePage?.(initial)
    await refresh

    expect(release.logPage.value.entries.map((item) => item.sequence)).toEqual([100, 101])
    expect(release.logPage.value.totalEntries).toBe(2)
  })

  it('discards an in-flight page after realtime compaction replaces the sequence space', async () => {
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    let resolvePage: ((page: ReleaseLogPage) => void) | undefined
    const persisted = session('session-inflight-compaction', 'workflowRunning')
    const initial: ReleaseLogPage = {
      ...logPage([logEntry(100, persisted.id)]),
      totalEntries: 100,
      totalBytes: 10_000,
    }
    const marker: ReleaseLogEntry = {
      ...logEntry(90, persisted.id),
      source: 'lifecycle',
      level: 'warning',
      message: '早期普通输出已截断',
    }
    const failure: ReleaseLogEntry = {
      ...logEntry(101, persisted.id),
      source: 'stderr',
      level: 'error',
      message: 'latest failure',
    }
    const compacted: ReleaseLogPage = {
      ...logPage([marker, failure]),
      totalEntries: 80,
      totalBytes: 8_000,
      truncated: true,
      warning: marker.message,
    }
    const response = new Promise<ReleaseLogPage>((resolve) => {
      resolvePage = resolve
    })
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn().mockResolvedValue({ session: persisted, logs: initial }),
      getReleaseLogs: vi.fn().mockReturnValue(response),
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return persisted
        },
      ),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.load(persisted.repositoryPath)
    await release.resume(persisted.id, directProxy)
    const refresh = release.refreshLogPage()
    resumeEvent?.({ kind: 'stepLog', entry: failure, page: compacted })
    resolvePage?.(initial)
    await refresh

    expect(release.logPage.value).toEqual(compacted)
    expect(release.logRequestPending.value).toBe(false)
  })

  it('owns inspection, planning and explicit release actions as readonly state', async () => {
    const inspection: ReleasePreflightResult = {
      repositoryPath: 'D:\\safe-temp\\repository',
      repository: {
        localBranch: 'master',
        defaultBranch: 'main',
        headSha: 'a'.repeat(40),
        remoteMainSha: 'a'.repeat(40),
        remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
        clean: true,
        sync: {
          status: 'synced',
          aheadCount: 0,
          behindCount: 0,
          aheadCommits: [],
        },
      },
      external: {
        tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
        activeReleaseRuns: 0,
        conflictingDrafts: 0,
        latestReleaseTag: 'v0.4.0',
      },
      releaseReady: true,
      blockingReasons: [],
      safePush: null,
    }
    const plan: ReleasePlanSummary = {
      id: 'plan-1',
      repositoryPath: 'D:\\safe-temp\\repository',
      previousVersion: '0.4.0',
      targetVersion: '0.5.0',
      notes: '最终说明',
      files: [],
    }
    const client = {
      inspectRepository: vi.fn().mockResolvedValue(inspection),
      pushRepository: vi.fn(),
      preparePlan: vi.fn().mockResolvedValue(plan),
      startRelease: vi.fn().mockResolvedValue(session('session-1', 'workflowRunning')),
      getReleaseSession: vi.fn(),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.inspect('D:\\safe-temp\\repository', directProxy)
    await release.preparePlan('D:\\safe-temp\\repository', '0.5.0', directProxy)
    await release.start('plan-1', directProxy)

    expect(release.inspection.value).toEqual(inspection)
    expect(release.plan.value).toEqual(plan)
    expect(release.session.value?.id).toBe('session-1')
    expect(release.busy.value).toBe(false)
    expect(release.error.value).toBeNull()
    expect(client.inspectRepository).toHaveBeenCalledWith(
      'D:\\safe-temp\\repository',
      directProxy,
    )
    expect(client.preparePlan).toHaveBeenCalledWith(
      'D:\\safe-temp\\repository',
      '0.5.0',
      directProxy,
      undefined,
    )
    expect(client.startRelease).toHaveBeenCalledWith(
      'plan-1',
      directProxy,
      expect.any(Function),
    )
  })

  it('drops stale channel events after resume and keeps stable errors', async () => {
    let startEvent: ((event: ReleaseEvent) => void) | undefined
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(
        async (
          _planId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          startEvent = onEvent
          return session('session-1', 'workflowQueued')
        },
      ),
      getReleaseSession: vi.fn(),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(
        async (
          _sessionId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          resumeEvent = onEvent
          return session('session-1', 'workflowRunning')
        },
      ),
      cancelRelease: vi.fn().mockRejectedValue({
        code: 'RELEASE_CANCEL_AFTER_PUSH_FORBIDDEN',
        message: '候选已推送，不能回滚取消。',
      }),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.start('plan-1', directProxy)
    await release.resume('session-1', directProxy)
    startEvent?.({ kind: 'sessionUpdated', session: session('stale', 'failed') })
    resumeEvent?.({ kind: 'sessionUpdated', session: session('session-1', 'auditingDraft') })

    expect(release.session.value?.id).toBe('session-1')
    expect(release.session.value?.phase).toBe('auditingDraft')
    expect(release.events.value).toHaveLength(1)

    await release.cancel('session-1')
    expect(release.error.value).toEqual({
      code: 'RELEASE_CANCEL_AFTER_PUSH_FORBIDDEN',
      message: '候选已推送，不能回滚取消。',
    })
  })

  it('clears the previous event channel before loading a persisted session', async () => {
    let startEvent: ((event: ReleaseEvent) => void) | undefined
    const persisted = {
      ...session('session-2', 'failed'),
      repositoryPath: 'D:\\safe-temp\\repository-2',
      failure: {
        phase: 'localChecks' as const,
        stepId: 'full-project-check',
        code: 'RELEASE_LOCAL_VERIFICATION_FAILED',
      },
    }
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(
        async (
          _planId: string,
          _proxy: typeof directProxy,
          onEvent: (event: ReleaseEvent) => void,
        ) => {
          startEvent = onEvent
          return session('session-1', 'localChecks')
        },
      ),
      getReleaseSession: vi.fn().mockResolvedValue({
        session: persisted,
        logs: logPage([]),
      }),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.start('plan-1', directProxy)
    startEvent?.({
      kind: 'stepFailed',
      stepId: 'release-structure-tests',
      code: 'RELEASE_LOCAL_VERIFICATION_FAILED',
      message: '旧会话失败。',
    })
    await release.load(persisted.repositoryPath)
    startEvent?.({ kind: 'sessionUpdated', session: session('stale', 'failed') })

    expect(release.session.value).toEqual(persisted)
    expect(release.events.value).toEqual([])
  })

  it('passes the explicitly displayed Draft identity to publish', async () => {
    const identity: DraftIdentity = {
      releaseId: 42,
      tagName: 'v0.5.0',
      targetCommitSha: 'a'.repeat(40),
    }
    const publishRelease = vi.fn().mockResolvedValue(session('session-1', 'completed'))
    const client = {
      inspectRepository: vi.fn(),
      pushRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn(),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease,
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.publish('session-1', identity, directProxy)

    expect(publishRelease).toHaveBeenCalledWith(
      'session-1',
      identity,
      directProxy,
      expect.any(Function),
    )
    expect(release.session.value?.phase).toBe('completed')
  })

  it('pushes the backend-provided preview with the current proxy and refreshes inspection', async () => {
    const ahead: ReleasePreflightResult = {
      repositoryPath: 'D:\\safe-temp\\repository',
      repository: {
        localBranch: 'master',
        defaultBranch: 'main',
        headSha: 'b'.repeat(40),
        remoteMainSha: 'a'.repeat(40),
        remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
        clean: true,
        sync: {
          status: 'ahead',
          aheadCount: 1,
          behindCount: 0,
          aheadCommits: [{ sha: 'b'.repeat(40), subject: 'feat: reviewed commit' }],
        },
      },
      external: {
        tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
        activeReleaseRuns: 0,
        conflictingDrafts: 0,
        latestReleaseTag: 'v0.4.0',
      },
      releaseReady: false,
      blockingReasons: ['本地领先远端 main 1 个提交；请先推送当前提交。'],
      safePush: {
        expectedHeadSha: 'b'.repeat(40),
        expectedRemoteMainSha: 'a'.repeat(40),
        commitCount: 1,
        commits: [{ sha: 'b'.repeat(40), subject: 'feat: reviewed commit' }],
      },
    }
    const synced: ReleasePreflightResult = {
      ...ahead,
      repository: {
        ...ahead.repository,
        remoteMainSha: 'b'.repeat(40),
        sync: { status: 'synced', aheadCount: 0, behindCount: 0, aheadCommits: [] },
      },
      releaseReady: true,
      blockingReasons: [],
      safePush: null,
    }
    const pushRepository = vi.fn().mockResolvedValue(synced)
    const client = {
      inspectRepository: vi.fn().mockResolvedValue(ahead),
      pushRepository,
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn(),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })
    const proxy = {
      enabled: true,
      proxyType: 'socks5' as const,
      host: '127.0.0.1',
      port: 1080,
    }

    await release.inspect(ahead.repositoryPath, directProxy)
    await release.pushRepository(proxy)

    expect(pushRepository).toHaveBeenCalledWith({
      repositoryPath: ahead.repositoryPath,
      expectedHeadSha: 'b'.repeat(40),
      expectedRemoteMainSha: 'a'.repeat(40),
      proxy,
    })
    expect(release.inspection.value).toEqual(synced)
  })

  it('keeps the original push preview when safe push fails', async () => {
    const ahead = {
      repositoryPath: 'D:\\safe-temp\\repository',
      repository: {
        localBranch: 'master',
        defaultBranch: 'main',
        headSha: 'b'.repeat(40),
        remoteMainSha: 'a'.repeat(40),
        remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
        clean: true,
        sync: {
          status: 'ahead' as const,
          aheadCount: 1,
          behindCount: 0,
          aheadCommits: [{ sha: 'b'.repeat(40), subject: 'feat: reviewed commit' }],
        },
      },
      external: {
        tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
        activeReleaseRuns: 0,
        conflictingDrafts: 0,
        latestReleaseTag: 'v0.4.0',
      },
      releaseReady: false,
      blockingReasons: ['本地领先远端 main 1 个提交；请先推送当前提交。'],
      safePush: {
        expectedHeadSha: 'b'.repeat(40),
        expectedRemoteMainSha: 'a'.repeat(40),
        commitCount: 1,
        commits: [{ sha: 'b'.repeat(40), subject: 'feat: reviewed commit' }],
      },
    }
    const client = {
      inspectRepository: vi.fn().mockResolvedValue(ahead),
      pushRepository: vi.fn().mockRejectedValue({
        code: 'GIT_REMOTE_MOVED',
        message: '远端 main 在确认后发生变化。',
      }),
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn(),
      getReleaseLogs: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.inspect(ahead.repositoryPath, directProxy)
    await release.pushRepository({ enabled: false, proxyType: 'http', host: '', port: null })

    expect(release.inspection.value).toEqual(ahead)
    expect(release.error.value?.code).toBe('GIT_REMOTE_MOVED')
  })
})
