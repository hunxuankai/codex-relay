import { describe, expect, it, vi } from 'vitest'
import {
  REPOSITORY_PREFERENCE_KEY,
  useRepositoryPreference,
  type RepositoryPreferenceStorage,
} from './useRepositoryPreference'

function storageWith(value: string | null): RepositoryPreferenceStorage {
  return {
    getItem: vi.fn().mockReturnValue(value),
    setItem: vi.fn(),
  }
}

describe('useRepositoryPreference', () => {
  it('restores a versioned path and only persists through the explicit remember action', () => {
    const storage = storageWith(JSON.stringify({
      version: 1,
      repositoryPath: '\\\\?\\D:\\safe-temp\\repository',
    }))
    const preference = useRepositoryPreference({ storage })

    expect(preference.repositoryPath.value).toBe('D:\\safe-temp\\repository')

    preference.update('D:\\unverified')
    expect(preference.repositoryPath.value).toBe('D:\\unverified')
    expect(storage.setItem).not.toHaveBeenCalled()

    preference.remember('  \\\\?\\D:\\canonical\\repository  ')
    expect(preference.repositoryPath.value).toBe('D:\\canonical\\repository')
    expect(storage.setItem).toHaveBeenCalledWith(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath: 'D:\\canonical\\repository' }),
    )
  })

  it.each([
    null,
    '{broken',
    JSON.stringify({ version: 2, repositoryPath: 'D:\\old' }),
    JSON.stringify({ version: 1, repositoryPath: '   ' }),
    JSON.stringify([]),
  ])('falls back to an empty path for missing, damaged or unsupported data', (stored) => {
    const preference = useRepositoryPreference({ storage: storageWith(stored) })

    expect(preference.repositoryPath.value).toBe('')
  })

  it('keeps the in-memory path usable when browser storage throws', () => {
    const storage: RepositoryPreferenceStorage = {
      getItem: vi.fn(() => {
        throw new Error('storage unavailable')
      }),
      setItem: vi.fn(() => {
        throw new Error('storage unavailable')
      }),
    }
    const preference = useRepositoryPreference({ storage })

    expect(preference.repositoryPath.value).toBe('')
    expect(() => preference.remember('D:\\safe-temp\\repository')).not.toThrow()
    expect(preference.repositoryPath.value).toBe('D:\\safe-temp\\repository')
  })

  it('normalizes extended UNC paths without truncating unsupported device paths', () => {
    const storage = storageWith(null)
    const preference = useRepositoryPreference({ storage })

    preference.remember('\\\\?\\UNC\\server\\share\\repository')
    expect(preference.repositoryPath.value).toBe('\\\\server\\share\\repository')
    expect(storage.setItem).toHaveBeenLastCalledWith(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath: '\\\\server\\share\\repository' }),
    )

    preference.remember('\\\\?\\Volume{test}\\repository')
    expect(preference.repositoryPath.value).toBe('\\\\?\\Volume{test}\\repository')
  })
})
