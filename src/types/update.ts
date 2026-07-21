export interface UpdateReleaseInfo {
  currentVersion: string
  version: string
  date: string | null
  notes: string | null
}

export interface UpdateProgress {
  downloadedBytes: number
  totalBytes: number | null
  percent: number | null
}

export interface UpdateSession {
  info: UpdateReleaseInfo
  downloadAndInstall(onProgress: (progress: UpdateProgress) => void): Promise<void>
  close(): Promise<void>
}

export interface UpdateClient {
  getCurrentVersion(): Promise<string>
  checkForUpdate(): Promise<UpdateSession | null>
}
