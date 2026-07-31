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
        },
      },
    })

    const result = await releaseConsoleTauri.inspectRepository('D:\\safe-temp\\repository')

    expect(result.repository.defaultBranch).toBe('main')
    expect(invokeMock).toHaveBeenCalledWith('inspect_release_repository', {
      repositoryPath: 'D:\\safe-temp\\repository',
    })
  })

  it('forwards typed channel events and preserves stable backend errors', async () => {
    const { releaseConsoleTauri } = await import('./tauri')
    const events: unknown[] = []
    invokeMock.mockImplementationOnce(async (_command: string, args: Record<string, unknown>) => {
      const channel = args.onEvent as MockChannel<unknown>
      channel.emit({ kind: 'stepStarted', stepId: 'remoteRun', startedAt: '2026-07-31T10:00:00Z' })
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
        },
      }
    })

    await releaseConsoleTauri.startRelease('plan-1', (event) => events.push(event))

    expect(events).toEqual([
      { kind: 'stepStarted', stepId: 'remoteRun', startedAt: '2026-07-31T10:00:00Z' },
    ])
    expect(channels).toHaveLength(1)
    expect(invokeMock).toHaveBeenCalledWith('start_release', {
      planId: 'plan-1',
      onEvent: expect.any(MockChannel),
    })

    invokeMock.mockResolvedValueOnce({
      success: false,
      error: { code: 'GITHUB_RUN_FAILED', message: 'GitHub Actions Run 执行失败。' },
    })
    await expect(releaseConsoleTauri.resumeRelease('session-1', () => undefined)).rejects.toMatchObject({
      code: 'GITHUB_RUN_FAILED',
      message: 'GitHub Actions Run 执行失败。',
    })
  })
})
