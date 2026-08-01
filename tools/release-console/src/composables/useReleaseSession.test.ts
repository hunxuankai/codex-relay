import { describe, expect, it, vi } from 'vitest'
import type {
  DraftIdentity,
  ReleaseEvent,
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
  }
}

describe('useReleaseSession', () => {
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
