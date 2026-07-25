export type ProviderTestKind = 'api' | 'codex'
export type ProviderTestStatus = 'passed' | 'failed' | 'unsupported' | 'cancelled'

export interface ProviderAvailabilityRequestTrace {
  method: string
  url: string
  body: string
}

export interface ProviderAvailabilityResponseTrace {
  status: number
  body: string
  bodyTruncated: boolean
}

export interface ProviderAvailabilityTrace {
  request: ProviderAvailabilityRequestTrace
  response: ProviderAvailabilityResponseTrace | null
}

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
  trace: ProviderAvailabilityTrace | null
}
