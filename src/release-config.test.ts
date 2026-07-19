import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const tauri = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'))

describe('Windows release configuration', () => {
  it('uses stable product, binary, icon, and NSIS current-user metadata', () => {
    expect(tauri.productName).toBe('Codex Relay')
    expect(tauri.mainBinaryName).toBe('CodexRelay')
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
})
