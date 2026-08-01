import { readonly, shallowRef } from 'vue'

export const REPOSITORY_PREFERENCE_KEY =
  'codex-relay-release-console.repository-preference.v1'

export type RepositoryPreferenceStorage = Pick<Storage, 'getItem' | 'setItem'>

interface UseRepositoryPreferenceOptions {
  storage?: RepositoryPreferenceStorage | null
}

interface StoredRepositoryPreference {
  version: 1
  repositoryPath: string
}

function normalizeRepositoryPathForDisplay(value: string): string {
  const trimmed = value.trim()

  if (/^\\\\\?\\UNC\\/i.test(trimmed)) {
    return `\\\\${trimmed.slice(8)}`
  }

  const extendedDrivePath = trimmed.match(/^\\\\\?\\([A-Za-z]:\\.*)$/)
  return extendedDrivePath?.[1] ?? trimmed
}

function browserStorage(): RepositoryPreferenceStorage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage
  } catch {
    return null
  }
}

function loadRepositoryPath(storage: RepositoryPreferenceStorage | null): string {
  if (!storage) return ''
  try {
    const raw = storage.getItem(REPOSITORY_PREFERENCE_KEY)
    if (raw === null) return ''
    const value = JSON.parse(raw) as Partial<StoredRepositoryPreference> | null
    if (value?.version !== 1 || typeof value.repositoryPath !== 'string') return ''
    return normalizeRepositoryPathForDisplay(value.repositoryPath)
  } catch {
    return ''
  }
}

export function useRepositoryPreference(options: UseRepositoryPreferenceOptions = {}) {
  const storage = options.storage === undefined ? browserStorage() : options.storage
  const repositoryPath = shallowRef(loadRepositoryPath(storage))

  function update(value: string) {
    repositoryPath.value = value
  }

  function remember(value: string) {
    const normalized = normalizeRepositoryPathForDisplay(value)
    if (normalized.length === 0) return
    repositoryPath.value = normalized
    try {
      storage?.setItem(
        REPOSITORY_PREFERENCE_KEY,
        JSON.stringify({ version: 1, repositoryPath: normalized }),
      )
    } catch {
      // 仓库偏好只是便利状态；浏览器存储不可用时继续使用当前会话内存值。
    }
  }

  return {
    repositoryPath: readonly(repositoryPath),
    update,
    remember,
  }
}
