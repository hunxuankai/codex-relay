import { beforeEach, describe, expect, it, vi } from 'vitest'

const invokeMock = vi.fn()
const channels: MockChannel<unknown>[] = []

class MockChannel<T> {
  onmessage: (message: T) => void = () => undefined

  constructor() {
    channels.push(this as MockChannel<unknown>)
  }

  emit(message: T) {
    this.onmessage(message)
  }
}

vi.mock('@tauri-apps/api/core', () => ({
  Channel: MockChannel,
  invoke: invokeMock,
}))

describe('release console typed Tauri service', () => {
  beforeEach(() => {
    invokeMock.mockReset()
    channels.length = 0
  })

  it('uses fixed command names, camelCase arguments and unwraps safe results', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: {
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
      },
    })
    const proxy = { enabled: false, proxyType: 'http' as const, host: '', port: null }

    const result = await releaseConsoleTauri.inspectRepository(
      'D:\\safe-temp\\repository',
      proxy,
    )

    expect(result.repository.defaultBranch).toBe('main')
    expect(invokeMock).toHaveBeenCalledWith('inspect_release_repository', {
      repositoryPath: 'D:\\safe-temp\\repository',
      proxy,
    })
  })

  it('passes one proxy snapshot to planning, start, resume and publish commands', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    const proxy = {
      enabled: true,
      proxyType: 'socks5' as const,
      host: '127.0.0.1',
      port: 1080,
    }
    const session = {
      id: 'session-1',
      repositoryPath: 'D:\\safe-temp\\repository',
      targetVersion: '0.5.0',
      phase: 'workflowRunning',
      candidateSha: 'a'.repeat(40),
      remoteMainSha: 'a'.repeat(40),
      workflow: null,
      draft: null,
      published: null,
      cleanup: null,
      cleanupWarning: null,
      failure: null,
    }
    invokeMock
      .mockResolvedValueOnce({
        success: true,
        data: {
          id: 'plan-1',
          repositoryPath: session.repositoryPath,
          previousVersion: '0.4.0',
          targetVersion: '0.5.0',
          notes: '最终说明',
          files: [],
        },
      })
      .mockResolvedValueOnce({ success: true, data: session })
      .mockResolvedValueOnce({ success: true, data: session })
      .mockResolvedValueOnce({ success: true, data: session })

    await releaseConsoleTauri.preparePlan(session.repositoryPath, '0.5.0', proxy, '最终说明')
    await releaseConsoleTauri.startRelease('plan-1', proxy, () => undefined)
    await releaseConsoleTauri.resumeRelease('session-1', proxy, () => undefined)
    await releaseConsoleTauri.publishRelease(
      'session-1',
      { releaseId: 42, tagName: 'v0.5.0', targetCommitSha: 'a'.repeat(40) },
      proxy,
      () => undefined,
    )

    expect(invokeMock.mock.calls[0]).toEqual([
      'prepare_release_plan',
      {
        repositoryPath: session.repositoryPath,
        targetVersion: '0.5.0',
        proxy,
        notes: '最终说明',
      },
    ])
    expect(invokeMock.mock.calls[1]).toEqual([
      'start_release',
      { planId: 'plan-1', proxy, onEvent: expect.any(MockChannel) },
    ])
    expect(invokeMock.mock.calls[2]).toEqual([
      'resume_release',
      { sessionId: 'session-1', proxy, onEvent: expect.any(MockChannel) },
    ])
    expect(invokeMock.mock.calls[3]).toEqual([
      'publish_release',
      {
        sessionId: 'session-1',
        expectedDraftIdentity: {
          releaseId: 42,
          tagName: 'v0.5.0',
          targetCommitSha: 'a'.repeat(40),
        },
        proxy,
        onEvent: expect.any(MockChannel),
      },
    ])
  })

  it('unwraps the session snapshot and requests bounded log pages with camelCase cursors', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    const session = {
      id: 'session-logs',
      repositoryPath: 'D:\\safe-temp\\repository',
      targetVersion: '0.5.0',
      phase: 'failed' as const,
      candidateSha: null,
      remoteMainSha: null,
      workflow: null,
      draft: null,
      published: null,
      cleanup: null,
      cleanupWarning: null,
      failure: {
        phase: 'localChecks' as const,
        stepId: 'full-project-check',
        code: 'RELEASE_LOCAL_VERIFICATION_FAILED',
      },
    }
    const entry = {
      sessionId: session.id,
      sequence: 42,
      timestamp: '2026-08-03T12:00:00.000Z',
      stepId: 'full-project-check',
      source: 'stderr' as const,
      level: 'error' as const,
      message: '测试失败上下文',
    }
    const page = {
      entries: [entry],
      nextBeforeSequence: 42,
      hasEarlier: true,
      totalEntries: 42,
      totalBytes: 4_096,
      truncated: false,
      warning: null,
    }
    invokeMock
      .mockResolvedValueOnce({ success: true, data: { session, logs: page } })
      .mockResolvedValueOnce({ success: true, data: page })

    const snapshot = await releaseConsoleTauri.getReleaseSession(session.repositoryPath)
    const earlier = await releaseConsoleTauri.getReleaseLogs(session.id, 42)

    expect(snapshot).toEqual({ session, logs: page })
    expect(earlier.entries[0]).toEqual(entry)
    expect(invokeMock.mock.calls).toEqual([
      ['get_release_session', { repositoryPath: session.repositoryPath }],
      ['get_release_logs', { sessionId: session.id, beforeSequence: 42 }],
    ])
  })

  it('forwards typed channel events and preserves stable backend errors', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    const events: unknown[] = []
    const proxy = { enabled: false, proxyType: 'http' as const, host: '', port: null }
    invokeMock.mockImplementationOnce(async (_command: string, args: Record<string, unknown>) => {
      const channel = args.onEvent as MockChannel<unknown>
      channel.emit({
        kind: 'stepLog',
        entry: {
          sessionId: 'session-1',
          sequence: 7,
          timestamp: '2026-07-31T10:00:00.000Z',
          stepId: 'remoteRun',
          source: 'lifecycle',
          level: 'info',
          message: 'Run 42 正在执行。',
        },
      })
      return {
        success: true,
        data: {
          id: 'session-1',
          repositoryPath: 'D:\\safe-temp\\repository',
          targetVersion: '0.5.0',
          phase: 'workflowRunning',
          candidateSha: 'a'.repeat(40),
          remoteMainSha: 'a'.repeat(40),
          workflow: null,
          draft: null,
          published: null,
          cleanup: null,
          cleanupWarning: null,
          failure: null,
        },
      }
    })

    await releaseConsoleTauri.startRelease('plan-1', proxy, (event) => events.push(event))

    expect(events).toEqual([
      {
        kind: 'stepLog',
        entry: {
          sessionId: 'session-1',
          sequence: 7,
          timestamp: '2026-07-31T10:00:00.000Z',
          stepId: 'remoteRun',
          source: 'lifecycle',
          level: 'info',
          message: 'Run 42 正在执行。',
        },
      },
    ])
    expect(channels).toHaveLength(1)
    expect(invokeMock).toHaveBeenCalledWith('start_release', {
      planId: 'plan-1',
      proxy,
      onEvent: expect.any(MockChannel),
    })

    invokeMock.mockResolvedValueOnce({
      success: false,
      error: { code: 'GITHUB_RUN_FAILED', message: 'GitHub Actions Run 执行失败。' },
    })
    await expect(
      releaseConsoleTauri.resumeRelease('session-1', proxy, () => undefined),
    ).rejects.toMatchObject({
      code: 'GITHUB_RUN_FAILED',
      message: 'GitHub Actions Run 执行失败。',
    })
  })

  it('passes the current proxy snapshot to the read-only connection test', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: {
        git: {
          success: true,
          code: null,
          message: 'Git 远端连接正常。',
          durationMillis: 12,
        },
        github: {
          success: false,
          code: 'GITHUB_PROCESS_TIMEOUT',
          message: 'GitHub API 连接超时。',
          durationMillis: 30_000,
        },
      },
    })
    const proxy = {
      enabled: true,
      proxyType: 'socks5' as const,
      host: '127.0.0.1',
      port: 1080,
    }

    const result = await releaseConsoleTauri.testConnection(proxy)

    expect(result.git.success).toBe(true)
    expect(result.github.code).toBe('GITHUB_PROCESS_TIMEOUT')
    expect(invokeMock).toHaveBeenCalledWith('test_release_connection', { proxy })
  })

  it('passes the confirmed repository SHAs and proxy to the safe push command', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: {
        repositoryPath: 'D:\\safe-temp\\repository',
        repository: {
          localBranch: 'master',
          defaultBranch: 'main',
          headSha: 'b'.repeat(40),
          remoteMainSha: 'b'.repeat(40),
          remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
          clean: true,
          sync: { status: 'synced', aheadCount: 0, behindCount: 0, aheadCommits: [] },
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
      },
    })
    const request = {
      repositoryPath: 'D:\\safe-temp\\repository',
      expectedHeadSha: 'b'.repeat(40),
      expectedRemoteMainSha: 'a'.repeat(40),
      proxy: {
        enabled: true,
        proxyType: 'socks5' as const,
        host: '127.0.0.1',
        port: 1080,
      },
    }

    const result = await releaseConsoleTauri.pushRepository(request)

    expect(result.releaseReady).toBe(true)
    expect(invokeMock).toHaveBeenCalledWith('push_release_repository', { request })
  })
})
