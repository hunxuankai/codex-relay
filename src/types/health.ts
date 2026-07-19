export type HealthLevel = 'normal' | 'warning' | 'error'

export interface HealthCheck {
  id: string
  label: string
  level: HealthLevel
  message: string
}

export interface HealthReport {
  level: HealthLevel
  checks: HealthCheck[]
  configDirectory: string
  currentProvider: string | null
  generatedAt: string
}
