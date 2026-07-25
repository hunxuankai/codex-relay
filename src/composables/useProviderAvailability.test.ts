import { flushPromises } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { RelayCommandError } from '../services/tauri'
import type {
  ProviderAvailabilityResult,
  ProviderTestKind,
  ProviderTestStatus,
} from '../types/providerAvailability'
import {
  useProviderAvailability,
  type ProviderAvailabilityClient,
} from './useProviderAvailability'

function result(
  kind: ProviderTestKind,
  status: ProviderTestStatus = 'passed',
): ProviderAvailabilityResult {
  return {
    providerId: 'provider-a',
    kind,
    status,
    code: status === 'passed' ? `${kind.toUpperCase()}_PASSED` : 'PROVIDER_TEST_CANCELLED',
    message: status === 'passed' ? '测试通过。' : '测试已取消。',
    model: 'gpt-5.6-sol',
    durationMs: 12,
    testedAt: '2026-07-23T00:00:00Z',
    httpStatus: kind === 'api' ? 200 : null,
    codexVersion: kind === 'codex' ? '0.144.4' : null,
  }
}

function client(overrides: Partial<ProviderAvailabilityClient> = {}): ProviderAvailabilityClient {
  return {
    testProviderApi: vi.fn().mockResolvedValue(result('api')),
    testProviderCodexCompatibility: vi.fn().mockResolvedValue(result('codex')),
    cancelProviderTest: vi.fn().mockResolvedValue(true),
    ...overrides,
  }
}

describe('useProviderAvailability', () => {
  it('keeps API and Codex results independent', async () => {
    const api = client()
    const availability = useProviderAvailability({
      client: api,
      createRequestId: vi.fn()
        .mockReturnValueOnce('request-api')
        .mockReturnValueOnce('request-codex'),
    })

    await availability.testApi('provider-a', false)
    await availability.testCodex('provider-a', true)

    expect(availability.resultFor('provider-a', 'api')).toEqual(result('api'))
    expect(availability.resultFor('provider-a', 'codex')).toEqual(result('codex'))
    expect(api.testProviderApi).toHaveBeenCalledWith('provider-a', 'request-api', false)
    expect(api.testProviderCodexCompatibility).toHaveBeenCalledWith(
      'provider-a',
      'request-codex',
      true,
    )
  })

  it('allows only one active test and exposes cancellable state', async () => {
    let finish!: (value: ProviderAvailabilityResult) => void
    const pending = new Promise<ProviderAvailabilityResult>((resolve) => {
      finish = resolve
    })
    const testProviderApi = vi.fn().mockReturnValue(pending)
    const api = client({ testProviderApi })
    const availability = useProviderAvailability({
      client: api,
      createRequestId: () => 'request-api',
    })

    const first = availability.testApi('provider-a', false)
    const duplicate = availability.testCodex('provider-a', false)

    expect(availability.busy.value).toBe(true)
    expect(availability.runningKind.value).toBe('api')
    expect(testProviderApi).toHaveBeenCalledOnce()
    expect(api.testProviderCodexCompatibility).not.toHaveBeenCalled()
    await duplicate
    finish(result('api'))
    await first
    expect(availability.busy.value).toBe(false)
  })

  it('requests cancellation and keeps only a cancelled late result', async () => {
    let finish!: (value: ProviderAvailabilityResult) => void
    const pending = new Promise<ProviderAvailabilityResult>((resolve) => {
      finish = resolve
    })
    const cancelProviderTest = vi.fn().mockResolvedValue(true)
    const availability = useProviderAvailability({
      client: client({
        testProviderApi: vi.fn().mockReturnValue(pending),
        cancelProviderTest,
      }),
      createRequestId: () => 'request-api',
    })

    const running = availability.testApi('provider-a', false)
    await availability.cancel()
    expect(availability.cancelling.value).toBe(true)
    expect(cancelProviderTest).toHaveBeenCalledWith('request-api')
    finish(result('api', 'cancelled'))
    await running

    expect(availability.resultFor('provider-a', 'api')?.status).toBe('cancelled')
    expect(availability.busy.value).toBe(false)
    expect(availability.cancelling.value).toBe(false)
  })

  it('clears results and discards responses from an invalidated Provider fingerprint', async () => {
    let finish!: (value: ProviderAvailabilityResult) => void
    const pending = new Promise<ProviderAvailabilityResult>((resolve) => {
      finish = resolve
    })
    const availability = useProviderAvailability({
      client: client({ testProviderApi: vi.fn().mockReturnValue(pending) }),
      createRequestId: () => 'request-api',
    })

    const running = availability.testApi('provider-a', false)
    availability.invalidateAll()
    finish(result('api'))
    await running

    expect(availability.resultFor('provider-a', 'api')).toBeNull()
  })

  it('stores only stable safe command errors', async () => {
    const availability = useProviderAvailability({
      client: client({
        testProviderApi: vi.fn().mockRejectedValue(
          new RelayCommandError('API_AUTH_FAILED', 'Provider 拒绝了 API Key。'),
        ),
      }),
      createRequestId: () => 'request-api',
    })

    await availability.testApi('provider-a', false)
    await flushPromises()

    expect(availability.error.value).toEqual({
      code: 'API_AUTH_FAILED',
      message: 'Provider 拒绝了 API Key。',
    })
    expect(JSON.stringify(availability.error.value)).not.toContain('stack')
  })
})
