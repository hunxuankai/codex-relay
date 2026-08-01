import { describe, expect, it, vi } from 'vitest'
import type { ReleaseConnectionTestResult, ReleaseProxySettings } from '../types/network'
import { useReleaseNetwork } from './useReleaseNetwork'

const DIRECT: ReleaseProxySettings = {
  enabled: false,
  proxyType: 'http',
  host: '',
  port: null,
}

function connectionResult(message: string): ReleaseConnectionTestResult {
  return {
    git: { success: true, code: null, message, durationMillis: 12 },
    github: { success: true, code: null, message, durationMillis: 18 },
  }
}

describe('useReleaseNetwork', () => {
  it('owns the read-only connection test result and stable busy state', async () => {
    const expected = connectionResult('连接正常')
    const client = {
      testConnection: vi.fn().mockResolvedValue(expected),
    }
    const network = useReleaseNetwork({ client })

    const returned = await network.test(DIRECT)

    expect(returned).toEqual(expected)
    expect(network.result.value).toEqual(expected)
    expect(network.busy.value).toBe(false)
    expect(network.error.value).toBeNull()
    expect(client.testConnection).toHaveBeenCalledWith(DIRECT)
  })

  it('drops late connection results after a newer test starts', async () => {
    let resolveFirst: ((value: ReleaseConnectionTestResult) => void) | undefined
    let resolveSecond: ((value: ReleaseConnectionTestResult) => void) | undefined
    const client = {
      testConnection: vi
        .fn()
        .mockImplementationOnce(
          () =>
            new Promise<ReleaseConnectionTestResult>((resolve) => {
              resolveFirst = resolve
            }),
        )
        .mockImplementationOnce(
          () =>
            new Promise<ReleaseConnectionTestResult>((resolve) => {
              resolveSecond = resolve
            }),
        ),
    }
    const network = useReleaseNetwork({ client })
    const first = network.test(DIRECT)
    const secondSettings: ReleaseProxySettings = {
      enabled: true,
      proxyType: 'http',
      host: '127.0.0.1',
      port: 7890,
    }
    const second = network.test(secondSettings)
    const newest = connectionResult('新结果')
    resolveSecond?.(newest)
    await second
    resolveFirst?.(connectionResult('过期结果'))
    await first

    expect(network.result.value).toEqual(newest)
    expect(network.busy.value).toBe(false)
  })

  it('invalidates an in-flight result when proxy settings change', async () => {
    let resolveProbe: ((value: ReleaseConnectionTestResult) => void) | undefined
    const client = {
      testConnection: vi.fn(
        () =>
          new Promise<ReleaseConnectionTestResult>((resolve) => {
            resolveProbe = resolve
          }),
      ),
    }
    const network = useReleaseNetwork({ client })
    const pending = network.test(DIRECT)

    network.invalidate()
    resolveProbe?.(connectionResult('不应显示'))
    await pending

    expect(network.result.value).toBeNull()
    expect(network.error.value).toBeNull()
    expect(network.busy.value).toBe(false)
  })
})
