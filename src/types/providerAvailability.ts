export type ProviderTestKind = 'api' | 'codex'
export type ProviderTestStatus = 'passed' | 'failed' | 'unsupported' | 'cancelled'

export interface ProviderAvailabilityResult {
  providerId: string
  kind: ProviderTestKind
  status: ProviderTestStatus
  code: string
  message: string
  model: string
  durationMs: number
  testedAt: string
  httpStatus: number | null
  codexVersion: string | null
}
