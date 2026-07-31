import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
import { afterEach, describe, expect, it } from 'vitest'

const workspaces: string[] = []
const validator = resolve('scripts/validate-release-request.ps1')
const version = '0.4.0'
const candidateSha = 'a'.repeat(40)

function createWorkspace(releaseNotes: string): { root: string; output: string } {
  const root = mkdtempSync(join(tmpdir(), 'codex-relay-release-request-'))
  workspaces.push(root)
  mkdirSync(join(root, '.github'), { recursive: true })
  mkdirSync(join(root, 'src-tauri', 'crates', 'codex-relay-core'), { recursive: true })

  writeFileSync(
    join(root, 'package.json'),
    `${JSON.stringify({ name: 'codex-relay', version }, null, 2)}\n`,
  )
  writeFileSync(
    join(root, 'package-lock.json'),
    `${JSON.stringify({
      name: 'codex-relay',
      version,
      lockfileVersion: 3,
      packages: { '': { name: 'codex-relay', version } },
    }, null, 2)}\n`,
  )
  writeFileSync(
    join(root, 'src-tauri', 'Cargo.toml'),
    `[workspace]\nmembers = ["crates/codex-relay-core"]\n\n[package]\nname = "codex-relay"\nversion = "${version}"\n`,
  )
  writeFileSync(
    join(root, 'src-tauri', 'crates', 'codex-relay-core', 'Cargo.toml'),
    `[package]\nname = "codex-relay-core"\nversion = "${version}"\n`,
  )
  writeFileSync(
    join(root, 'src-tauri', 'Cargo.lock'),
    `version = 4\n\n[[package]]\nname = "codex-relay"\nversion = "${version}"\n\n[[package]]\nname = "codex-relay-core"\nversion = "${version}"\n`,
  )
  writeFileSync(join(root, '.github', 'release-notes.md'), releaseNotes)

  return { root, output: join(root, 'github-output.txt') }
}

function runValidator(root: string, output: string) {
  return spawnSync(
    'pwsh.exe',
    [
      '-NoLogo',
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      validator,
      '-ExpectedVersion',
      version,
      '-ExpectedSha',
      candidateSha,
      '-ActualSha',
      candidateSha,
      '-Workspace',
      root,
      '-GitHubOutput',
      output,
    ],
    { encoding: 'utf8', timeout: 20_000 },
  )
}

afterEach(() => {
  for (const workspace of workspaces.splice(0)) {
    rmSync(workspace, { recursive: true, force: true })
  }
})

describe('release request validation', () => {
  it('rejects release notes containing credential-shaped text before writing workflow output', () => {
    const { root, output } = createWorkspace(`## 更新内容

- 修复发布流程。
- Authorization: Bearer test-key-release-not-real

## 更新方式

从 \`v0.3.0\` 更新到 \`v${version}\`。

## 注意事项

Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。
`)

    const result = runValidator(root, output)
    const diagnostic = `${result.stdout}\n${result.stderr}`

    expect(result.status).not.toBe(0)
    expect(diagnostic).toContain('发布说明包含疑似秘密')
  })
})
