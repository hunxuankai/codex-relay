export interface CommandError {
  code: string
  message: string
}

export interface CommandResult<T> {
  success: boolean
  data?: T
  error?: CommandError
}

export interface RelayUiError {
  code: string
  message: string
}
