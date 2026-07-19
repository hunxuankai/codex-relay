import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const tauri = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'))
const prepareDevData = readFileSync('scripts/prepare-dev-data.ps1', 'utf8')
const rustEntryPoint = readFileSync('src-tauri/src/main.rs', 'utf8')

describe('Windows release configuration', () => {
  it('uses stable product, binary, icon, and NSIS current-user metadata', () => {
    expect(tauri.productName).toBe('Codex Relay')
    expect(tauri.mainBinaryName).toBe('CodexRelay')
    expect(tauri.identifier).toBe('com.codexrelay.desktop')
    expect(tauri.bundle.targets).toEqual(['nsis'])
    expect(tauri.bundle.icon).toContain('icons/icon.ico')
    expect(tauri.bundle.windows.nsis).toMatchObject({
      installMode: 'currentUser',
      startMenuFolder: 'Codex Relay',
      installerIcon: 'icons/icon.ico',
    })
    expect(tauri.bundle.windows.nsis.installerHooks).toBeUndefined()
  })

  it('restricts production web content to local application resources', () => {
    expect(tauri.app.security.csp).toContain("default-src 'self'")
    expect(tauri.app.security.csp).not.toBeNull()
  })

  it('provides explicit release and NSIS build scripts', () => {
    expect(packageJson.scripts['build:release']).toBe('tauri build')
    expect(packageJson.scripts['bundle:nsis']).toBe('tauri build --bundles nsis')
  })

  it('marks every generated development API key as explicitly non-real', () => {
    const generatedApiKeys = [...prepareDevData.matchAll(/"(?:OPENAI_API_KEY|apiKey)":\s*"([^"]+)"/g)]
      .map((match) => match[1] ?? '')

    expect(generatedApiKeys).not.toHaveLength(0)
    expect(generatedApiKeys.every((apiKey) => /^test-key-[a-z0-9-]+-not-real$/.test(apiKey))).toBe(true)
  })

  it('uses the Windows GUI subsystem for release builds', () => {
    expect(rustEntryPoint).toContain(
      '#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]',
    )
  })
})
