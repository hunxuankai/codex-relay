import { getCurrentScope, onScopeDispose, readonly, shallowRef } from 'vue'
import * as relay from '../services/tauri'
import type { RelayUiError } from '../types/command'
import type { HealthReport } from '../types/health'

export interface HealthClient {
  runCriticalSelfCheck(): Promise<HealthReport>
  runExtendedSelfCheck(): Promise<HealthReport>
  onSelfCheckCompleted(handler: (report: HealthReport) => void): Promise<() => void>
}

export interface UseHealthOptions {
  client?: HealthClient
  subscribe?: boolean
}

const defaultClient: HealthClient = {
  runCriticalSelfCheck: relay.runCriticalSelfCheck,
  runExtendedSelfCheck: relay.runExtendedSelfCheck,
  onSelfCheckCompleted: relay.onSelfCheckCompleted,
}

export function useHealth(options: UseHealthOptions = {}) {
  const client = options.client ?? defaultClient
  const report = shallowRef<HealthReport | null>(null)
  const loading = shallowRef(true)
  const busy = shallowRef(false)
  const error = shallowRef<RelayUiError | null>(null)
  let stateSequence = 0

  function setError(caught: unknown) {
    error.value = caught instanceof relay.RelayCommandError
      ? { code: caught.code, message: caught.message }
      : { code: 'UNEXPECTED_ERROR', message: '自检失败，请重试。' }
  }

  async function refreshCritical() {
    const requestSequence = ++stateSequence
    loading.value = true
    error.value = null
    try {
      const next = await client.runCriticalSelfCheck()
      if (requestSequence === stateSequence) {
        report.value = next
      }
    } catch (caught) {
      if (requestSequence === stateSequence) {
        setError(caught)
      }
    } finally {
      if (requestSequence === stateSequence) {
        loading.value = false
      }
    }
  }

  async function runExtended() {
    if (busy.value) return
    const requestSequence = ++stateSequence
    busy.value = true
    error.value = null
    try {
      const next = await client.runExtendedSelfCheck()
      if (requestSequence === stateSequence) {
        report.value = next
      }
    } catch (caught) {
      if (requestSequence === stateSequence) {
        setError(caught)
      }
    } finally {
      busy.value = false
    }
  }

  let disposed = false
  let stopSubscription: (() => void) | undefined
  if (options.subscribe ?? true) {
    void client
      .onSelfCheckCompleted((next) => {
        stateSequence += 1
        report.value = next
        loading.value = false
      })
      .then((stop) => {
        if (disposed) stop()
        else stopSubscription = stop
      })
      .catch((caught: unknown) => {
        if (!disposed) setError(caught)
      })
  }
  if (getCurrentScope()) {
    onScopeDispose(() => {
      disposed = true
      stopSubscription?.()
    })
  }

  void refreshCritical()

  return {
    report: readonly(report),
    loading: readonly(loading),
    busy: readonly(busy),
    error: readonly(error),
    refreshCritical,
    runExtended,
  }
}
