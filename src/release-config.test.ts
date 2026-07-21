import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const tauri = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const packageJson = JSON.parse(readFileSync('package.json', 'utf8'))
const cargoToml = readFileSync('src-tauri/Cargo.toml', 'utf8')
const rustLibrary = readFileSync('src-tauri/src/lib.rs', 'utf8')
const defaultCapability = JSON.parse(
  readFileSync('src-tauri/capabilities/default.json', 'utf8'),
)
const updaterReleaseConfigPath = 'src-tauri/tauri.updater.conf.json'
const updaterReleaseConfig = existsSync(updaterReleaseConfigPath)
  ? JSON.parse(readFileSync(updaterReleaseConfigPath, 'utf8'))
  : null
const releaseWorkflowPath = '.github/workflows/release.yml'
const releaseWorkflow = existsSync(releaseWorkflowPath)
  ? readFileSync(releaseWorkflowPath, 'utf8')
  : ''
const updaterEndpoint =
  'https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json'
const updaterPublicKey =
  'dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEFEMEEwQ0U1QUFFQzI3RApSV1I5d3E1YXpxRFFDbGlIR2UxY3ZZR05BUDhqZnRPYk9ycFd6OVB4MVdMenBFR1RuTjhaVnc4UQo='
const prepareDevData = readFileSync('scripts/prepare-dev-data.ps1', 'utf8')
const rustEntryPoint = readFileSync('src-tauri/src/main.rs', 'utf8')
const nsisTemplatePath = 'src-tauri/installer/custom-installer.nsi'
const nsisTemplate = existsSync(nsisTemplatePath) ? readFileSync(nsisTemplatePath, 'utf8') : ''

describe('Windows release configuration', () => {
  it('uses stable product, binary, icon, and NSIS per-machine metadata', () => {
    expect(tauri.productName).toBe('Codex Relay')
    expect(tauri.mainBinaryName).toBe('CodexRelay')
    expect(tauri.identifier).toBe('com.codexrelay.desktop')
    expect(tauri.bundle.targets).toEqual(['nsis'])
    expect(tauri.bundle.icon).toContain('icons/icon.ico')
    expect(tauri.bundle.windows.nsis).toMatchObject({
      installMode: 'perMachine',
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
    expect(packageJson.scripts['build:release']).toBe(
      'tauri build --config src-tauri/tauri.updater.conf.json',
    )
    expect(packageJson.scripts['bundle:nsis']).toBe('tauri build --bundles nsis')
  })

  it('keeps updater signing out of ordinary builds and enables it only for publishing', () => {
    expect(tauri.version).toBe('../package.json')
    expect(packageJson.dependencies['@tauri-apps/plugin-updater']).toMatch(/^\^2/)
    expect(cargoToml).toMatch(/^tauri-plugin-updater\s*=\s*"2\.[^"]+"/m)
    expect(rustLibrary).toContain('.plugin(tauri_plugin_updater::Builder::new().build())')
    expect(defaultCapability.permissions).toContain('updater:default')

    expect(updaterReleaseConfig).toMatchObject({
      bundle: {
        createUpdaterArtifacts: true,
      },
    })
    expect(packageJson.scripts.build).toBe('tauri build')
    expect(packageJson.scripts['build:release']).toContain(
      '--config src-tauri/tauri.updater.conf.json',
    )
  })

  it('trusts only the fixed public GitHub release endpoint and updater public key', () => {
    expect(tauri.plugins?.updater).toEqual({
      endpoints: [updaterEndpoint],
      pubkey: updaterPublicKey,
    })
    expect(updaterEndpoint).toMatch(/^https:\/\//)
  })

  it('publishes updater assets only through a manually triggered draft workflow', () => {
    expect(releaseWorkflow).toContain('workflow_dispatch:')
    expect(releaseWorkflow).toContain('contents: write')
    expect(releaseWorkflow).toContain('npm run check')
    expect(releaseWorkflow).toContain('windows-latest')
    expect(releaseWorkflow).toContain('releaseDraft: true')
    expect(releaseWorkflow).toContain('updaterJsonPreferNsis: true')
    expect(releaseWorkflow).toContain('uploadUpdaterJson: true')
    expect(releaseWorkflow).toContain('TAURI_SIGNING_PRIVATE_KEY:')
    expect(releaseWorkflow).toContain('TAURI_SIGNING_PRIVATE_KEY_PASSWORD:')
    expect(releaseWorkflow.match(/TAURI_SIGNING_PRIVATE_KEY:/g)).toHaveLength(1)
    expect(releaseWorkflow).toContain('releaseBody: |')
    expect(releaseWorkflow).toContain('首次带 updater 的版本需要手动安装')
    expect(releaseWorkflow).not.toContain('请在发布前补充本版本的变更说明')
    expect(releaseWorkflow).toContain(
      'tauri-apps/tauri-action@1deb371b0cd8bd54025b384f1cd735e725c4060f',
    )
  })

  it('marks every generated development API key as explicitly non-real', () => {
    const generatedApiKeys = [...prepareDevData.matchAll(/"(?:OPENAI_API_KEY|apiKey)":\s*"([^"]+)"/g)]
      .map((match) => match[1] ?? '')

    expect(generatedApiKeys).not.toHaveLength(0)
    expect(generatedApiKeys.every((apiKey) => /^test-key-[a-z0-9-]+-not-real$/.test(apiKey))).toBe(true)
  })

  it('uses the Windows npm command shim when safe development launches Tauri', () => {
    expect(prepareDevData).toContain('& npm.cmd run dev')
    expect(prepareDevData).not.toMatch(/&\s+npm\s+run\s+dev/)
  })

  it('uses the Windows GUI subsystem for release builds', () => {
    expect(rustEntryPoint).toContain(
      '#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]',
    )
  })

  it('prefers a fixed D drive for fresh per-machine installs and preserves upgrade paths', () => {
    expect(tauri.bundle.windows.nsis.installMode).toBe('perMachine')
    expect(tauri.bundle.windows.nsis.template).toBe('installer/custom-installer.nsi')
    expect(existsSync(nsisTemplatePath)).toBe(true)
    expect(nsisTemplate).toContain('GetDriveTypeW(w "D:\\")')
    expect(nsisTemplate).toContain('D:\\Program Files\\${PRODUCTNAME}')
    expect(nsisTemplate).toContain('$PROGRAMFILES64\\${PRODUCTNAME}')
    expect(nsisTemplate.indexOf('GetDriveTypeW')).toBeLessThan(
      nsisTemplate.indexOf('Call RestorePreviousInstallLocation'),
    )
    expect(nsisTemplate).toContain('!insertmacro MUI_PAGE_DIRECTORY')
  })
})
