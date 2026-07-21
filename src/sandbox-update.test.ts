import { createHash } from 'node:crypto'
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, isAbsolute, join, resolve } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'
import { spawnSync } from 'node:child_process'

const prepareScript = resolve(
  'scripts/windows-sandbox/prepare-update-test.ps1',
)
const temporaryRoots: string[] = []

function expectSameExistingPath(actual: string, expected: string) {
  expect(isAbsolute(actual)).toBe(true)
  expect(realpathSync.native(actual).toLowerCase()).toBe(
    realpathSync.native(expected).toLowerCase(),
  )
}

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true })
  }
})

describe('Windows Sandbox update test preparation', () => {
  it('creates an isolated Sandbox configuration from a verified installer', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const installerPath = join(testRoot, 'Codex Relay_0.1.0_x64-setup.exe')
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)

    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()
    const stageRoot = join(testRoot, 'stage')
    const result = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      { encoding: 'utf8' },
    )

    expect(result.status, result.stderr).toBe(0)
    if (result.status !== 0) return

    const output = JSON.parse(result.stdout) as {
      stageRoot: string
      configPath: string
      inputPath: string
      resultsPath: string
      installer: { fileName: string; sha256: string }
      expectedTargetVersion: string
    }
    expectSameExistingPath(output.stageRoot, stageRoot)
    expect(output.installer).toEqual({
      fileName: basename(installerPath),
      sha256: expectedSha256,
    })
    expect(output.expectedTargetVersion).toBe('0.1.1')
    expect(readFileSync(join(output.inputPath, basename(installerPath)))).toEqual(
      installerBytes,
    )
    expect(existsSync(join(output.inputPath, 'guest-bootstrap.ps1'))).toBe(true)

    const sandboxConfig = readFileSync(output.configPath, 'utf8')
    const xml = new DOMParser().parseFromString(sandboxConfig, 'application/xml')
    expect(xml.querySelector('parsererror')).toBeNull()
    expect(xml.querySelector('Networking')?.textContent).toBe('Default')
    expect(xml.querySelector('ClipboardRedirection')?.textContent).toBe('Disable')

    const mappings = [...xml.querySelectorAll('MappedFolder')].map((mapping) => ({
      hostFolder: mapping.querySelector('HostFolder')?.textContent,
      sandboxFolder: mapping.querySelector('SandboxFolder')?.textContent,
      readOnly: mapping.querySelector('ReadOnly')?.textContent,
    }))
    expect(mappings).toEqual([
      {
        hostFolder: output.inputPath,
        sandboxFolder: 'C:\\CodexRelaySandbox\\Input',
        readOnly: 'true',
      },
      {
        hostFolder: output.resultsPath,
        sandboxFolder: 'C:\\CodexRelaySandbox\\Results',
        readOnly: 'false',
      },
    ])
    const logonCommand = xml.querySelector('LogonCommand Command')?.textContent ?? ''
    expect(logonCommand).toContain('guest-bootstrap.ps1')
    expect(logonCommand).toContain('-ExpectedTargetVersion "0.1.1"')
  }, 20_000)

  it('dry-runs guest fixture preparation without exposing test keys in the report', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const installerPath = join(testRoot, 'Codex Relay_0.1.0_x64-setup.exe')
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)
    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()
    const stageRoot = join(testRoot, 'stage')

    const preparation = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      { encoding: 'utf8' },
    )
    expect(preparation.status, preparation.stderr).toBe(0)
    if (preparation.status !== 0) return

    const prepared = JSON.parse(preparation.stdout) as {
      inputPath: string
      resultsPath: string
      installer: { fileName: string }
    }
    const bootstrap = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-bootstrap.ps1'),
        '-InstallerPath',
        join(prepared.inputPath, prepared.installer.fileName),
        '-ResultsPath',
        prepared.resultsPath,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-DryRun',
      ],
      { encoding: 'utf8' },
    )
    expect(bootstrap.status, bootstrap.stderr).toBe(0)
    if (bootstrap.status !== 0) return

    const output = JSON.parse(bootstrap.stdout) as {
      codexHome: string
      appData: string
      reportPath: string
      verifyLauncherPath: string
    }
    expectSameExistingPath(output.codexHome, join(stageRoot, 'dev-data', 'codex'))
    expectSameExistingPath(output.appData, join(stageRoot, 'dev-data', 'app-data'))
    const verifyLauncher = readFileSync(output.verifyLauncherPath, 'utf8')
    expect(verifyLauncher).toContain('guest-verify.ps1')
    expect(verifyLauncher).toContain('-ExpectedVersion "0.1.1"')

    const auth = JSON.parse(
      readFileSync(join(output.codexHome, 'auth.json'), 'utf8'),
    ) as { OPENAI_API_KEY: string }
    const providers = JSON.parse(
      readFileSync(join(output.appData, 'providers.json'), 'utf8'),
    ) as { providers: Record<string, { apiKey: string }> }
    const fixtureKeys = [
      auth.OPENAI_API_KEY,
      ...Object.values(providers.providers).map((provider) => provider.apiKey),
    ]
    expect(
      fixtureKeys.every((key) => /^test-key-[a-z0-9-]+-not-real$/.test(key)),
    ).toBe(true)

    const reportText = readFileSync(output.reportPath, 'utf8')
    const report = JSON.parse(reportText) as {
      phase: string
      protectedFiles: Array<{ relativePath: string; sha256: string }>
    }
    expect(report.phase).toBe('before')
    expect(report.protectedFiles.map((file) => file.relativePath)).toEqual([
      'codex/auth.json',
      'codex/config.toml',
      'app-data/providers.json',
    ])
    expect(report.protectedFiles.every((file) => /^[A-F0-9]{64}$/.test(file.sha256))).toBe(
      true,
    )
    expect(reportText).not.toContain('test-key-')
  }, 20_000)

  it('verifies the installed version and protected fixture hashes after an update', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const installerPath = join(testRoot, 'Codex Relay_0.1.0_x64-setup.exe')
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)
    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()
    const stageRoot = join(testRoot, 'stage')

    const preparation = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      { encoding: 'utf8' },
    )
    expect(preparation.status, preparation.stderr).toBe(0)
    if (preparation.status !== 0) return

    const prepared = JSON.parse(preparation.stdout) as {
      inputPath: string
      resultsPath: string
      installer: { fileName: string }
    }
    const bootstrap = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-bootstrap.ps1'),
        '-InstallerPath',
        join(prepared.inputPath, prepared.installer.fileName),
        '-ResultsPath',
        prepared.resultsPath,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-DryRun',
      ],
      { encoding: 'utf8' },
    )
    expect(bootstrap.status, bootstrap.stderr).toBe(0)
    if (bootstrap.status !== 0) return

    const installLocation = join(stageRoot, 'installed')
    mkdirSync(installLocation)
    writeFileSync(join(installLocation, 'CodexRelay.exe'), 'executable-fixture')

    const verification = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-verify.ps1'),
        '-ResultsPath',
        prepared.resultsPath,
        '-ExpectedVersion',
        '0.1.1',
        '-DryRun',
        '-InstalledVersion',
        '0.1.1',
        '-InstallLocation',
        `"${installLocation}"`,
      ],
      { encoding: 'utf8' },
    )
    expect(verification.status, verification.stderr).toBe(0)
    if (verification.status !== 0) return

    const output = JSON.parse(verification.stdout) as {
      reportPath: string
      dataPreserved: boolean
      versionMatched: boolean
    }
    expect(output).toMatchObject({
      dataPreserved: true,
      versionMatched: true,
    })

    const reportText = readFileSync(output.reportPath, 'utf8')
    const report = JSON.parse(reportText) as {
      phase: string
      expectedVersion: string
      installedVersion: string
      installLocation: string
      protectedFiles: Array<{ relativePath: string; matchesBefore: boolean }>
    }
    expect(report).toMatchObject({
      phase: 'after',
      expectedVersion: '0.1.1',
      installedVersion: '0.1.1',
    })
    expectSameExistingPath(report.installLocation, installLocation)
    expect(report.protectedFiles.every((file) => file.matchesBefore)).toBe(true)
    expect(reportText).not.toContain('test-key-')
  }, 20_000)

  it('restores paired Relay overrides before starting an installed app', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const installerPath = join(testRoot, 'Codex Relay_0.1.0_x64-setup.exe')
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)
    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()
    const stageRoot = join(testRoot, 'stage')

    const preparation = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      { encoding: 'utf8' },
    )
    expect(preparation.status, preparation.stderr).toBe(0)
    if (preparation.status !== 0) return

    const prepared = JSON.parse(preparation.stdout) as {
      inputPath: string
      resultsPath: string
      installer: { fileName: string }
    }
    const bootstrap = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-bootstrap.ps1'),
        '-InstallerPath',
        join(prepared.inputPath, prepared.installer.fileName),
        '-ResultsPath',
        prepared.resultsPath,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-DryRun',
      ],
      { encoding: 'utf8' },
    )
    expect(bootstrap.status, bootstrap.stderr).toBe(0)
    if (bootstrap.status !== 0) return

    const bootstrapOutput = JSON.parse(bootstrap.stdout) as {
      startLauncherPath: string
    }
    expect(existsSync(join(prepared.inputPath, 'guest-start-app.ps1'))).toBe(true)
    if (!existsSync(join(prepared.inputPath, 'guest-start-app.ps1'))) return
    expect(readFileSync(bootstrapOutput.startLauncherPath, 'utf8')).toContain(
      'guest-start-app.ps1',
    )

    const installLocation = join(stageRoot, 'installed')
    mkdirSync(installLocation)
    const executablePath = join(installLocation, 'CodexRelay.exe')
    writeFileSync(executablePath, 'executable-fixture')
    const start = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-start-app.ps1'),
        '-ResultsPath',
        prepared.resultsPath,
        '-DryRun',
        '-InstallLocation',
        `"${installLocation}"`,
      ],
      { encoding: 'utf8' },
    )
    expect(start.status, start.stderr).toBe(0)
    if (start.status !== 0) return

    const output = JSON.parse(start.stdout) as {
      codexHome: string
      appData: string
      executablePath: string
      launched: boolean
    }
    expect(output.launched).toBe(false)
    expectSameExistingPath(output.codexHome, join(stageRoot, 'dev-data', 'codex'))
    expectSameExistingPath(output.appData, join(stageRoot, 'dev-data', 'app-data'))
    expectSameExistingPath(output.executablePath, executablePath)
  }, 20_000)

  it('rejects a before report that duplicates a protected file entry', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const installerPath = join(testRoot, 'Codex Relay_0.1.0_x64-setup.exe')
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)
    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()
    const stageRoot = join(testRoot, 'stage')

    const preparation = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      { encoding: 'utf8' },
    )
    expect(preparation.status, preparation.stderr).toBe(0)
    if (preparation.status !== 0) return

    const prepared = JSON.parse(preparation.stdout) as {
      inputPath: string
      resultsPath: string
      installer: { fileName: string }
    }
    const bootstrap = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-bootstrap.ps1'),
        '-InstallerPath',
        join(prepared.inputPath, prepared.installer.fileName),
        '-ResultsPath',
        prepared.resultsPath,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-DryRun',
      ],
      { encoding: 'utf8' },
    )
    expect(bootstrap.status, bootstrap.stderr).toBe(0)
    if (bootstrap.status !== 0) return

    const beforePath = join(prepared.resultsPath, 'before.json')
    const before = JSON.parse(readFileSync(beforePath, 'utf8')) as {
      protectedFiles: Array<{
        relativePath: string
        length: number
        sha256: string
      }>
    }
    const firstProtectedFile = before.protectedFiles[0]
    expect(firstProtectedFile).toBeDefined()
    if (!firstProtectedFile) return
    before.protectedFiles[1] = { ...firstProtectedFile }
    writeFileSync(beforePath, JSON.stringify(before, null, 2))

    const installLocation = join(stageRoot, 'installed')
    mkdirSync(installLocation)
    writeFileSync(join(installLocation, 'CodexRelay.exe'), 'executable-fixture')
    const verification = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        join(prepared.inputPath, 'guest-verify.ps1'),
        '-ResultsPath',
        prepared.resultsPath,
        '-ExpectedVersion',
        '0.1.1',
        '-DryRun',
        '-InstalledVersion',
        '0.1.1',
        '-InstallLocation',
        installLocation,
      ],
      { encoding: 'utf8' },
    )

    expect(verification.status).not.toBe(0)
    expect(verification.stderr).toContain('SANDBOX_BEFORE_REPORT_FILES_INVALID')
  }, 20_000)

  it('rejects a staging root that traverses a directory junction', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const installerPath = join(testRoot, 'Codex Relay_0.1.0_x64-setup.exe')
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)
    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()

    const junctionTarget = join(testRoot, 'junction-target')
    const stageRoot = join(testRoot, 'stage-junction')
    mkdirSync(junctionTarget)
    symlinkSync(junctionTarget, stageRoot, 'junction')

    const result = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      { encoding: 'utf8' },
    )

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain('SANDBOX_STAGE_ROOT_REPARSE_POINT')
  }, 20_000)

  it('rejects an installer path under protected Relay directories', () => {
    const testRoot = mkdtempSync(join(tmpdir(), 'codex-relay-sandbox-test-'))
    temporaryRoots.push(testRoot)

    const fakeUserProfile = join(testRoot, 'user-profile')
    const fakeLocalAppData = join(testRoot, 'local-app-data')
    const installerDirectory = join(fakeUserProfile, '.codex')
    const installerPath = join(installerDirectory, 'installer.exe')
    mkdirSync(installerDirectory, { recursive: true })
    mkdirSync(fakeLocalAppData)
    const installerBytes = Buffer.from('sandbox-installer-fixture')
    writeFileSync(installerPath, installerBytes)
    const expectedSha256 = createHash('sha256')
      .update(installerBytes)
      .digest('hex')
      .toUpperCase()
    const stageRoot = join(testRoot, 'stage')

    const result = spawnSync(
      'powershell.exe',
      [
        '-NoLogo',
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        prepareScript,
        '-InstallerPath',
        installerPath,
        '-ExpectedSha256',
        expectedSha256,
        '-ExpectedTargetVersion',
        '0.1.1',
        '-StageRoot',
        stageRoot,
        '-PrepareOnly',
      ],
      {
        encoding: 'utf8',
        env: {
          ...process.env,
          USERPROFILE: fakeUserProfile,
          LOCALAPPDATA: fakeLocalAppData,
        },
      },
    )

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain('SANDBOX_INSTALLER_PATH_UNSAFE')
    expect(existsSync(stageRoot)).toBe(false)
  }, 20_000)
})
