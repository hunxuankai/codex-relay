import { readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'

export const LOCAL_PROXY_CANDIDATES = [
  'http://127.0.0.1:7890',
  'http://127.0.0.1:7897',
  'http://127.0.0.1:10809',
  'http://127.0.0.1:1080',
  'http://127.0.0.1:8080',
  'http://127.0.0.1:3128',
] as const

export interface ProxyDiscoveryClient {
  testProxy(proxy: string): Promise<void>
}

export interface UseProxyDiscoveryOptions {
  testProxy?: ProxyDiscoveryClient['testProxy']
}

export function useProxyDiscovery(options: UseProxyDiscoveryOptions = {}) {
  const testProxy = options.testProxy ?? relay.testUpdateProxy
  const confirmationOpen = shallowRef(false)
  const resultsOpen = shallowRef(false)
  const testing = shallowRef(false)
  const discovering = shallowRef(false)
  const availableProxies = shallowRef<string[]>([])
  const selectedProxy = shallowRef<string | null>(null)
  const message = shallowRef<string | null>(null)
  const error = shallowRef<RelayUiError | null>(null)
  let requestSequence = 0

  function clearFeedback() {
    message.value = null
    error.value = null
  }

  function requestDiscovery() {
    clearFeedback()
    confirmationOpen.value = true
  }

  function cancelDiscovery() {
    confirmationOpen.value = false
  }

  async function confirmDiscovery() {
    const sequence = ++requestSequence
    confirmationOpen.value = false
    discovering.value = true
    clearFeedback()
    const results = await Promise.all(
      LOCAL_PROXY_CANDIDATES.map(async (proxy) => {
        try {
          await testProxy(proxy)
          return proxy
        } catch {
          return null
        }
      }),
    )
    if (sequence !== requestSequence) return
    availableProxies.value = results.filter((proxy) => proxy !== null)
    selectedProxy.value = availableProxies.value[0] ?? null
    resultsOpen.value = true
    discovering.value = false
  }

  async function testCurrentProxy(proxy: string) {
    const sequence = ++requestSequence
    testing.value = true
    clearFeedback()
    try {
      await testProxy(proxy)
      if (sequence === requestSequence) message.value = '代理可用于访问更新源。'
    } catch (caught) {
      if (sequence === requestSequence) {
        error.value = caught instanceof relay.RelayCommandError
          ? { code: caught.code, message: caught.message }
          : { code: 'PROXY_TEST_FAILED', message: '代理无法访问更新源，请检查代理是否正在运行。' }
      }
    } finally {
      if (sequence === requestSequence) testing.value = false
    }
  }

  function selectProxy(proxy: string) {
    if (availableProxies.value.includes(proxy)) selectedProxy.value = proxy
  }

  function closeResults() {
    resultsOpen.value = false
  }

  return {
    confirmationOpen: readonly(confirmationOpen),
    resultsOpen: readonly(resultsOpen),
    testing: readonly(testing),
    discovering: readonly(discovering),
    availableProxies: readonly(availableProxies),
    selectedProxy: readonly(selectedProxy),
    message: readonly(message),
    error: readonly(error),
    requestDiscovery,
    cancelDiscovery,
    confirmDiscovery,
    testCurrentProxy,
    selectProxy,
    closeResults,
  }
}
