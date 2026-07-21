import { describe, expect, it, vi } from 'vitest'
import { LOCAL_PROXY_CANDIDATES, useProxyDiscovery } from './useProxyDiscovery'

describe('useProxyDiscovery', () => {
  it('does not probe before confirmation or after cancellation', async () => {
    const testProxy = vi.fn().mockResolvedValue(undefined)
    const discovery = useProxyDiscovery({ testProxy })

    discovery.requestDiscovery()
    discovery.cancelDiscovery()
    await Promise.resolve()

    expect(testProxy).not.toHaveBeenCalled()
    expect(discovery.confirmationOpen.value).toBe(false)
  })

  it('probes the fixed candidates in parallel and keeps every successful result', async () => {
    const pending = new Map<string, { resolve: () => void; reject: () => void }>()
    const testProxy = vi.fn((proxy: string) => new Promise<void>((resolve, reject) => {
      pending.set(proxy, { resolve, reject: () => reject(new Error('unavailable')) })
    }))
    const discovery = useProxyDiscovery({ testProxy })

    discovery.requestDiscovery()
    const result = discovery.confirmDiscovery()

    expect(testProxy.mock.calls.map(([proxy]) => proxy)).toEqual(LOCAL_PROXY_CANDIDATES)
    pending.get('http://127.0.0.1:7890')?.resolve()
    pending.get('http://127.0.0.1:7897')?.resolve()
    for (const proxy of LOCAL_PROXY_CANDIDATES.slice(2)) pending.get(proxy)?.reject()
    await result

    expect(discovery.availableProxies.value).toEqual([
      'http://127.0.0.1:7890',
      'http://127.0.0.1:7897',
    ])
    expect(discovery.selectedProxy.value).toBe('http://127.0.0.1:7890')
    expect(discovery.resultsOpen.value).toBe(true)
  })

  it('reports manual proxy test success and failure without saving settings', async () => {
    const testProxy = vi.fn()
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error('secret response'))
    const discovery = useProxyDiscovery({ testProxy })

    await discovery.testCurrentProxy('http://127.0.0.1:7890')
    expect(discovery.message.value).toBe('代理可用于访问更新源。')

    await discovery.testCurrentProxy('http://127.0.0.1:7897')
    expect(discovery.error.value).toEqual({
      code: 'PROXY_TEST_FAILED',
      message: '代理无法访问更新源，请检查代理是否正在运行。',
    })
  })
})
