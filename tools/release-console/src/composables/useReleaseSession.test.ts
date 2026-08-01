import { describe, expect, it, vi } from 'vitest'
import type {
  DraftIdentity,
  ReleaseEvent,
  ReleasePlanSummary,
  ReleasePreflightResult,
  ReleaseSession,
} from '../types/release'
import { useReleaseSession } from './useReleaseSession'

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
      },
      external: {
        tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
        activeReleaseRuns: 0,
        conflictingDrafts: 0,
        latestReleaseTag: 'v0.4.0',
      },
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
      preparePlan: vi.fn().mockResolvedValue(plan),
      startRelease: vi.fn().mockResolvedValue(session('session-1', 'workflowRunning')),
      getReleaseSession: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.inspect('D:\\safe-temp\\repository')
    await release.preparePlan('D:\\safe-temp\\repository', '0.5.0')
    await release.start('plan-1')

    expect(release.inspection.value).toEqual(inspection)
    expect(release.plan.value).toEqual(plan)
    expect(release.session.value?.id).toBe('session-1')
    expect(release.busy.value).toBe(false)
    expect(release.error.value).toBeNull()
  })

  it('drops stale channel events after resume and keeps stable errors', async () => {
    let startEvent: ((event: ReleaseEvent) => void) | undefined
    let resumeEvent: ((event: ReleaseEvent) => void) | undefined
    const client = {
      inspectRepository: vi.fn(),
      preparePlan: vi.fn(),
      startRelease: vi.fn(async (_planId: string, onEvent: (event: ReleaseEvent) => void) => {
        startEvent = onEvent
        return session('session-1', 'workflowQueued')
      }),
      getReleaseSession: vi.fn(),
      resumeRelease: vi.fn(async (_sessionId: string, onEvent: (event: ReleaseEvent) => void) => {
        resumeEvent = onEvent
        return session('session-1', 'workflowRunning')
      }),
      cancelRelease: vi.fn().mockRejectedValue({
        code: 'RELEASE_CANCEL_AFTER_PUSH_FORBIDDEN',
        message: '候选已推送，不能回滚取消。',
      }),
      publishRelease: vi.fn(),
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.start('plan-1')
    await release.resume('session-1')
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
      preparePlan: vi.fn(),
      startRelease: vi.fn(),
      getReleaseSession: vi.fn(),
      resumeRelease: vi.fn(),
      cancelRelease: vi.fn(),
      publishRelease,
      exportSummary: vi.fn(),
    }
    const release = useReleaseSession({ client })

    await release.publish('session-1', identity)

    expect(publishRelease).toHaveBeenCalledWith('session-1', identity, expect.any(Function))
    expect(release.session.value?.phase).toBe('completed')
  })
})
