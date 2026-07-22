import { computed, readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'
import type {
  ProviderAvailabilityResult,
  ProviderTestKind,
} from '../types/providerAvailability'

export interface ProviderAvailabilityClient {
  testProviderApi(providerId: string, requestId: string): Promise<ProviderAvailabilityResult>
  testProviderCodexCompatibility(
    providerId: string,
    requestId: string,
  ): Promise<ProviderAvailabilityResult>
  cancelProviderTest(requestId: string): Promise<boolean>
}

export interface UseProviderAvailabilityOptions {
  client?: ProviderAvailabilityClient
  createRequestId?: () => string
}

interface RunningTest {
  providerId: string
  kind: ProviderTestKind
  requestId: string
  token: number
  generation: number
}

type ProviderResults = Record<
  string,
  Partial<Record<ProviderTestKind, ProviderAvailabilityResult>>
>

const defaultClient: ProviderAvailabilityClient = {
  testProviderApi: relay.testProviderApi,
  testProviderCodexCompatibility: relay.testProviderCodexCompatibility,
  cancelProviderTest: relay.cancelProviderTest,
}

export function useProviderAvailability(options: UseProviderAvailabilityOptions = {}) {
  const client = options.client ?? defaultClient
  const createRequestId = options.createRequestId ?? (() => globalThis.crypto.randomUUID())
  const results = shallowRef<ProviderResults>({})
  const running = shallowRef<RunningTest | null>(null)
  const cancelling = shallowRef(false)
  const error = shallowRef<RelayUiError | null>(null)
  let operationToken = 0
  let generation = 0

  const busy = computed(() => running.value !== null)
  const runningKind = computed(() => running.value?.kind ?? null)
  const runningProviderId = computed(() => running.value?.providerId ?? null)

  function resultFor(
    providerId: string,
    kind: ProviderTestKind,
  ): ProviderAvailabilityResult | null {
    return results.value[providerId]?.[kind] ?? null
  }

  function setError(caught: unknown) {
    if (caught instanceof relay.RelayCommandError) {
      error.value = { code: caught.code, message: caught.message }
      return
    }
    error.value = { code: 'UNEXPECTED_ERROR', message: 'Provider 测试失败，请重试。' }
  }

  function storeResult(result: ProviderAvailabilityResult) {
    results.value = {
      ...results.value,
      [result.providerId]: {
        ...results.value[result.providerId],
        [result.kind]: result,
      },
    }
  }

  async function test(providerId: string, kind: ProviderTestKind): Promise<void> {
    if (running.value) return
    const requestId = createRequestId()
    const current: RunningTest = {
      providerId,
      kind,
      requestId,
      token: ++operationToken,
      generation,
    }
    running.value = current
    cancelling.value = false
    error.value = null
    try {
      const result =
        kind === 'api'
          ? await client.testProviderApi(providerId, requestId)
          : await client.testProviderCodexCompatibility(providerId, requestId)
      if (
        running.value?.token !== current.token ||
        generation !== current.generation ||
        (cancelling.value && result.status !== 'cancelled')
      ) {
        return
      }
      if (result.providerId !== providerId || result.kind !== kind) {
        error.value = {
          code: 'PROVIDER_TEST_RESULT_MISMATCH',
          message: 'Provider 测试结果与当前请求不匹配。',
        }
        return
      }
      storeResult(result)
    } catch (caught) {
      if (running.value?.token === current.token && generation === current.generation) {
        setError(caught)
      }
    } finally {
      if (running.value?.token === current.token) {
        running.value = null
        cancelling.value = false
      }
    }
  }

  function testApi(providerId: string) {
    return test(providerId, 'api')
  }

  function testCodex(providerId: string) {
    return test(providerId, 'codex')
  }

  async function cancel(): Promise<boolean> {
    const current = running.value
    if (!current || cancelling.value) return false
    cancelling.value = true
    error.value = null
    try {
      const cancelled = await client.cancelProviderTest(current.requestId)
      if (!cancelled && running.value?.token === current.token) {
        cancelling.value = false
        error.value = {
          code: 'PROVIDER_TEST_NOT_RUNNING',
          message: 'Provider 测试已经结束。',
        }
      }
      return cancelled
    } catch (caught) {
      if (running.value?.token === current.token) {
        cancelling.value = false
        setError(caught)
      }
      return false
    }
  }

  function invalidateAll() {
    generation += 1
    results.value = {}
    error.value = null
    if (running.value) void cancel()
  }

  return {
    results: readonly(results),
    busy,
    runningKind,
    runningProviderId,
    cancelling: readonly(cancelling),
    error: readonly(error),
    resultFor,
    testApi,
    testCodex,
    cancel,
    invalidateAll,
  }
}
