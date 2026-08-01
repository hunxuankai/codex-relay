export type ReleaseProxyType = 'http' | 'socks5'

export interface ReleaseProxySettings {
  enabled: boolean
  proxyType: ReleaseProxyType
  host: string
  port: number | null
}

export const DEFAULT_RELEASE_PROXY_SETTINGS: ReleaseProxySettings = {
  enabled: false,
  proxyType: 'http',
  host: '',
  port: null,
}

function isValidIpv6Host(host: string): boolean {
  try {
    const parsed = new URL(`http://[${host}]/`)
    return parsed.hostname.length > 2
  } catch {
    return false
  }
}

export function releaseProxyValidationReason(settings: ReleaseProxySettings): string | null {
  if (!settings.enabled) return null
  const host = settings.host.trim()
  if (host.length === 0) return '填写代理地址。'
  const ipv4Parts = host.split('.')
  const looksLikeIpv4 =
    ipv4Parts.length === 4 && ipv4Parts.every((part) => /^\d+$/.test(part))
  const isIpv4 =
    looksLikeIpv4 &&
    ipv4Parts.every((part) => part.length <= 3 && Number(part) <= 255)
  const isIpv6 = host.includes(':') && isValidIpv6Host(host)
  const isHostname =
    !looksLikeIpv4 &&
    !host.includes(':') &&
    host.length <= 253 &&
    host
      .split('.')
      .every((label) =>
        /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/i.test(label),
      )
  if (!isIpv4 && !isIpv6 && !isHostname) {
    return '代理地址只填写主机名、IPv4 或 IPv6，不包含协议、路径或账号。'
  }
  if (
    settings.port === null ||
    !Number.isInteger(settings.port) ||
    settings.port < 1 ||
    settings.port > 65_535
  ) {
    return '填写 1–65535 的代理端口。'
  }
  return null
}

export interface ConnectionProbeResult {
  success: boolean
  code: string | null
  message: string
  durationMillis: number
}

export interface ReleaseConnectionTestResult {
  git: ConnectionProbeResult
  github: ConnectionProbeResult
}
