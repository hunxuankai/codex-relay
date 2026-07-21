export interface WindowBounds {
  width: number
  height: number
  x: number | null
  y: number | null
}

export interface NetworkProxySettings {
  enabled: boolean
  url: string
}

export interface Settings {
  autostartEnabled: boolean
  trayOnlyOnAutostart: boolean
  closeToTray: boolean
  showWindowOnManualStart: boolean
  window: WindowBounds
  firstRunCompleted: boolean
  networkProxy: NetworkProxySettings
}

export interface AutostartState {
  configuredEnabled: boolean
  actualEnabled: boolean
  isConsistent: boolean
}

export interface SettingsState {
  settings: Settings
  autostart: AutostartState
}
