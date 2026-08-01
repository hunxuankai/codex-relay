import { readonly, shallowRef } from 'vue'
import {
  releaseConsoleTauri,
  type ReleaseConsoleClient,
} from '../services/tauri'
import type { ReleaseConnectionTestResult, ReleaseProxySettings } from '../types/network'
import type { CommandError } from '../types/release'

interface UseReleaseNetworkOptions {
  client?: Pick<ReleaseConsoleClient, 'testConnection'>
}

function safeError(error: unknown): CommandError {
  if (typeof error === 'object' && error !== null) {
    const code = 'code' in error ? error.code : undefined
    const message = 'message' in error ? error.message : undefined
    if (typeof code === 'string' && typeof message === 'string') return { code, message }
  }
  return {
    code: 'RELEASE_CONNECTION_TEST_FAILED',
    message: '连接测试失败。',
  }
}

export function useReleaseNetwork(options: UseReleaseNetworkOptions = {}) {
  const client = options.client ?? releaseConsoleTauri
  const result = shallowRef<ReleaseConnectionTestResult | null>(null)
  const busy = shallowRef(false)
  const error = shallowRef<CommandError | null>(null)
  let operationSequence = 0

  async function test(settings: ReleaseProxySettings) {
    const sequence = ++operationSequence
    busy.value = true
    error.value = null
    result.value = null
    try {
      const value = await client.testConnection(settings)
      if (sequence === operationSequence) result.value = value
      return value
    } catch (cause) {
      if (sequence === operationSequence) error.value = safeError(cause)
      return null
    } finally {
      if (sequence === operationSequence) busy.value = false
    }
  }

  function invalidate() {
    operationSequence += 1
    result.value = null
    error.value = null
    busy.value = false
  }

  return {
    result: readonly(result),
    busy: readonly(busy),
    error: readonly(error),
    test,
    invalidate,
  }
}
