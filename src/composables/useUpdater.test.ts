import { flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import type { UpdateClient, UpdateSession } from '../types/update'
import { useUpdater } from './useUpdater'

function createSession(version: string): UpdateSession {
  return {
    info: {
      currentVersion: '0.1.0',
      version,
      date: null,
      notes: null,
    },
    downloadAndInstall: vi.fn(),
    close: vi.fn().mockResolvedValue(undefined),
  }
}

describe('useUpdater', () => {
  it('loads only the local version until the user explicitly checks', async () => {
    const client: UpdateClient = {
      getCurrentVersion: vi.fn().mockResolvedValue('0.1.0'),
      checkForUpdate: vi.fn().mockResolvedValue(null),
    }

    const updater = useUpdater({ client })
    await flushPromises()

    expect(client.getCurrentVersion).toHaveBeenCalledOnce()
    expect(client.checkForUpdate).not.toHaveBeenCalled()
    expect(updater.currentVersion.value).toBe('0.1.0')
    expect(updater.status.value).toBe('idle')

    await updater.check()

    expect(client.getCurrentVersion).toHaveBeenCalledOnce()
    expect(client.checkForUpdate).toHaveBeenCalledOnce()
    expect(updater.currentVersion.value).toBe('0.1.0')
    expect(updater.status.value).toBe('upToDate')
  })

  it('exposes an available release and ignores duplicate checks while busy', async () => {
    let resolveCheck!: (session: UpdateSession | null) => void
    const client: UpdateClient = {
      getCurrentVersion: vi.fn().mockResolvedValue('0.1.0'),
      checkForUpdate: vi.fn().mockReturnValue(new Promise((resolve) => {
        resolveCheck = resolve
      })),
    }
    const updater = useUpdater({ client })

    const first = updater.check()
    const duplicate = updater.check()
    expect(updater.status.value).toBe('checking')
    await flushPromises()
    expect(client.checkForUpdate).toHaveBeenCalledOnce()

    resolveCheck(createSession('0.2.0'))
    await Promise.all([first, duplicate])

    expect(updater.status.value).toBe('available')
    expect(updater.release.value?.version).toBe('0.2.0')
  })

  it('keeps the latest check result when an older request finishes later', async () => {
    let resolveFirst!: (session: UpdateSession | null) => void
    const firstSession = createSession('0.2.0')
    const secondSession = createSession('0.3.0')
    const client: UpdateClient = {
      getCurrentVersion: vi.fn().mockResolvedValue('0.1.0'),
      checkForUpdate: vi.fn()
        .mockReturnValueOnce(new Promise((resolve) => {
          resolveFirst = resolve
        }))
        .mockResolvedValueOnce(secondSession),
    }
    const updater = useUpdater({ client })

    const first = updater.check()
    await flushPromises()
    updater.reset()
    await updater.check()
    resolveFirst(firstSession)
    await first

    expect(updater.release.value?.version).toBe('0.3.0')
    expect(firstSession.close).toHaveBeenCalledOnce()
  })

  it('requires confirmation and exposes progress without reporting installation success', async () => {
    const session = createSession('0.2.0')
    vi.mocked(session.downloadAndInstall).mockImplementation(async (onProgress) => {
      onProgress({ downloadedBytes: 5, totalBytes: 10, percent: 50 })
      onProgress({ downloadedBytes: 10, totalBytes: 10, percent: 100 })
    })
    const client: UpdateClient = {
      getCurrentVersion: vi.fn().mockResolvedValue('0.1.0'),
      checkForUpdate: vi.fn().mockResolvedValue(session),
    }
    const updater = useUpdater({ client })
    await updater.check()

    updater.requestInstall()
    expect(updater.status.value).toBe('confirming')
    updater.cancelInstall()
    expect(updater.status.value).toBe('available')
    updater.requestInstall()
    await updater.confirmInstall()

    expect(session.downloadAndInstall).toHaveBeenCalledOnce()
    expect(updater.progress.value).toEqual({
      downloadedBytes: 10,
      totalBytes: 10,
      percent: 100,
    })
    expect(updater.status.value).toBe('launching')
  })
})
