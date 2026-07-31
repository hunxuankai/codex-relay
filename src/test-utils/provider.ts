import type { ProviderConnectionProjection } from '../types/provider'

export function providerConnection(
  overrides: Partial<ProviderConnectionProjection> = {},
): ProviderConnectionProjection {
  return {
    role: null,
    status: 'none',
    action: null,
    disabledReason: null,
    targetProviderId: null,
    sourceProviderName: null,
    appliedBaseUrlName: null,
    appliedApiKeyName: null,
    restoreBaseUrlName: null,
    restoreApiKeyName: null,
    ...overrides,
  }
}
