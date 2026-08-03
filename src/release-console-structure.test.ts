import { createHash } from 'node:crypto'
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { isAbsolute, join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { describe, expect, it } from 'vitest'

const rootPackage = JSON.parse(readFileSync('package.json', 'utf8'))
const rootCargo = readFileSync('src-tauri/Cargo.toml', 'utf8')
const mainTauri = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const consoleRoot = 'tools/release-console'
const consolePackagePath = `${consoleRoot}/package.json`
const consoleCargoPath = `${consoleRoot}/src-tauri/Cargo.toml`
const consoleTauriPath = `${consoleRoot}/src-tauri/tauri.conf.json`
const consoleCapabilityPath = `${consoleRoot}/src-tauri/capabilities/default.json`
const packageScriptPath = 'scripts/package-release-console.ps1'
const packageScript = readFileSync(packageScriptPath, 'utf8')
const gitignore = readFileSync('.gitignore', 'utf8')
const readme = readFileSync('README.md', 'utf8')
const publishingGuide = readFileSync('.trellis/spec/release/publishing.md', 'utf8')
const updaterGuide = readFileSync('.trellis/spec/release/updater.md', 'utf8')
const portableDeliveryPath = 'artifacts/release-console/CodexRelayReleaseConsole.exe'

function getWindowsShortPath(directory: string) {
  const result = spawnSync(
    'cmd.exe',
    ['/d', '/c', 'for %I in (.) do @echo %~sI'],
    { cwd: directory, encoding: 'utf8' },
  )
  expect(result.status, result.stderr).toBe(0)
  const shortPath = result.stdout.trim()
  expect(isAbsolute(shortPath)).toBe(true)
  return shortPath
}

describe('release console project isolation', () => {
  it('registers an independent npm and Cargo workspace with explicit root scripts', () => {
    expect(rootPackage.workspaces ?? []).toContain(consoleRoot)
    expect(rootPackage.scripts['dev:release-console']).toBe(
      'npm run dev --workspace @codex-relay/release-console',
    )
    expect(rootPackage.scripts['test:release-console']).toBe(
      'npm run test --workspace @codex-relay/release-console',
    )
    expect(rootPackage.scripts['typecheck:release-console']).toBe(
      'npm run typecheck --workspace @codex-relay/release-console',
    )
    expect(rootPackage.scripts['build:release-console']).toBe(
      'npm run build:app --workspace @codex-relay/release-console',
    )
    expect(rootPackage.scripts['postbuild:release-console']).toBe(
      'pwsh -NoProfile -File scripts/package-release-console.ps1',
    )
    expect(rootCargo).toContain('"../tools/release-console/src-tauri"')
    expect(gitignore).toContain('tools/release-console/src-tauri/gen/')
  })

  it('uses a separate portable Tauri identity and only core permissions', () => {
    expect(existsSync(consolePackagePath)).toBe(true)
    expect(existsSync(consoleCargoPath)).toBe(true)
    expect(existsSync(consoleTauriPath)).toBe(true)
    expect(existsSync(consoleCapabilityPath)).toBe(true)

    const consolePackage = JSON.parse(readFileSync(consolePackagePath, 'utf8'))
    const consoleCargo = readFileSync(consoleCargoPath, 'utf8')
    const consoleTauri = JSON.parse(readFileSync(consoleTauriPath, 'utf8'))
    const consoleCapability = JSON.parse(readFileSync(consoleCapabilityPath, 'utf8'))

    expect(consolePackage).toMatchObject({
      name: '@codex-relay/release-console',
      private: true,
    })
    expect(consoleCargo).toContain('name = "codex-relay-release-console"')
    expect(consoleCargo).toContain('name = "CodexRelayReleaseConsole"')
    expect(consoleTauri).toMatchObject({
      productName: 'Codex Relay 发布控制台',
      mainBinaryName: 'CodexRelayReleaseConsole',
      identifier: 'com.codexrelay.release-console',
      bundle: { active: false },
    })
    expect(consoleTauri.app.windows).toEqual([
      expect.objectContaining({
        label: 'main',
        title: 'Codex Relay 发布控制台',
        visible: true,
      }),
    ])
    expect(consoleCapability.permissions).toEqual(['core:default'])
  })

  it('keeps the public Codex Relay bundle independent from the console binary', () => {
    expect(mainTauri.productName).toBe('Codex Relay')
    expect(mainTauri.mainBinaryName).toBe('CodexRelay')
    expect(mainTauri.identifier).toBe('com.codexrelay.desktop')
    expect(mainTauri.bundle.active).toBe(true)
    expect(JSON.stringify(mainTauri)).not.toContain('CodexRelayReleaseConsole')
    expect(JSON.stringify(mainTauri)).not.toContain('release-console')
  })

  it('keeps the default portable delivery outside the main frontend output tree', () => {
    const defaultDirectory = packageScript.match(
      /\$DestinationDirectory = Join-Path \$repositoryRoot '([^']+)'/,
    )?.[1]

    const normalizedDirectory = defaultDirectory?.replace(/\\/g, '/')

    expect(normalizedDirectory).toBe('artifacts/release-console')
    expect(normalizedDirectory).not.toMatch(/^dist(?:\/|$)/)
    expect(gitignore).toContain('artifacts/')
  })

  it('copies the portable EXE to the ignored delivery directory and reports its hash', () => {
    expect(existsSync(packageScriptPath)).toBe(true)
    const temporaryRoot = mkdtempSync(join(tmpdir(), 'codex-relay-release-console-package-'))
    try {
      const temporaryRootAlias = getWindowsShortPath(temporaryRoot)
      const source = join(temporaryRoot, 'CodexRelayReleaseConsole.exe')
      const destination = join(temporaryRootAlias, 'dist')
      const bytes = Buffer.from('portable-release-console-fixture')
      writeFileSync(source, bytes)

      const result = spawnSync(
        'pwsh.exe',
        [
          '-NoProfile',
          '-File',
          packageScriptPath,
          '-SourcePath',
          source,
          '-DestinationDirectory',
          destination,
        ],
        { encoding: 'utf8' },
      )

      expect(result.status, result.stderr).toBe(0)
      const evidence = JSON.parse(result.stdout.trim())
      const packaged = join(destination, 'CodexRelayReleaseConsole.exe')
      expect(readFileSync(packaged)).toEqual(bytes)
      expect(isAbsolute(evidence.path)).toBe(true)
      expect(realpathSync.native(evidence.path).toLowerCase()).toBe(
        realpathSync.native(packaged).toLowerCase(),
      )
      expect(evidence).toMatchObject({
        size: bytes.length,
        sha256: createHash('sha256').update(bytes).digest('hex').toUpperCase(),
      })
    } finally {
      rmSync(temporaryRoot, { recursive: true, force: true })
    }
  })

  it('documents the console-first release flow and its explicit verification limits', () => {
    for (const document of [readme, publishingGuide]) {
      expect(document).toContain('npm run build:release-console')
      expect(document).toContain(portableDeliveryPath)
      expect(document).toContain('Sandbox')
      expect(document).toContain('UAC')
    }
    expect(publishingGuide).toContain('expected_version')
    expect(publishingGuide).toContain('expected_sha')
    expect(publishingGuide).toContain('.github/release-notes.md')
    expect(updaterGuide).toContain('Codex Relay 发布控制台')
    expect(updaterGuide).toContain('不进入正式 Codex Relay 安装包')
  })
})
