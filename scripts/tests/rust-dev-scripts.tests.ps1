$ErrorActionPreference = 'Stop'

$workspace = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$guardScript = Join-Path $workspace 'scripts/check-rust-dev-environment.ps1'
$prepareDevDataScript = Join-Path $workspace 'scripts/prepare-dev-data.ps1'

function Invoke-Guard {
  param(
    [string[]]$ObservedCommandLine = @()
  )

  $arguments = @(
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-File',
    $guardScript,
    '-UseObservedCommandLine'
  )

  if ($ObservedCommandLine.Count -gt 0) {
    $arguments += '-ObservedCommandLine'
    $arguments += $ObservedCommandLine
  }

  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    $output = & powershell.exe @arguments 2>&1
    $exitCode = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
  }

  [pscustomobject]@{
    ExitCode = $exitCode
    Output = ($output -join [Environment]::NewLine)
  }
}

function Assert-Equal {
  param(
    $Actual,
    $Expected,
    [string]$Message
  )

  if ($Actual -ne $Expected) {
    throw "$Message`nExpected: $Expected`nActual: $Actual"
  }
}

function Assert-Contains {
  param(
    [string]$Actual,
    [string]$Expected,
    [string]$Message
  )

  if ($Actual.IndexOf($Expected, [StringComparison]::Ordinal) -lt 0) {
    throw "$Message`nExpected output to contain: $Expected`nActual output: $Actual"
  }
}

function Assert-NotContains {
  param(
    [string]$Actual,
    [string]$Unexpected,
    [string]$Message
  )

  if ($Actual.IndexOf($Unexpected, [StringComparison]::Ordinal) -ge 0) {
    throw "$Message`nExpected output not to contain: $Unexpected`nActual output: $Actual"
  }
}

function Invoke-PrepareDevData {
  param(
    [switch]$NoRustWatch
  )

  $arguments = @(
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-File',
    $prepareDevDataScript,
    '-DryRun'
  )

  if ($NoRustWatch) {
    $arguments += '-NoRustWatch'
  }

  $previousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    $output = & powershell.exe @arguments 2>&1
    $exitCode = $LASTEXITCODE
  } finally {
    $ErrorActionPreference = $previousErrorActionPreference
  }

  [pscustomobject]@{
    ExitCode = $exitCode
    Output = ($output -join [Environment]::NewLine)
  }
}

$watching = Invoke-Guard -ObservedCommandLine @(
  'node.exe C:\repo\node_modules\@tauri-apps\cli\tauri.js dev'
)
Assert-Equal $watching.ExitCode 2 'A watching Tauri dev process should block the fast Rust test.'
Assert-Contains $watching.Output 'npm run dev:safe:no-watch' 'The guard should suggest the safe no-watch command.'
Assert-Contains $watching.Output 'npm run dev:frontend' 'The guard should suggest the frontend-only command.'

$noWatch = Invoke-Guard -ObservedCommandLine @(
  'node.exe C:\repo\node_modules\@tauri-apps\cli\tauri.js dev --no-watch'
)
Assert-Equal $noWatch.ExitCode 0 'A Tauri dev process with --no-watch should be allowed.'

$noTauri = Invoke-Guard
Assert-Equal $noTauri.ExitCode 0 'No Tauri dev process should be allowed.'

$defaultDev = Invoke-PrepareDevData
Assert-Equal $defaultDev.ExitCode 0 'The default safe dev dry-run should succeed.'
Assert-Contains $defaultDev.Output 'npm.cmd run dev' 'The default safe dev command should stay unchanged.'
Assert-NotContains $defaultDev.Output '--no-watch' 'The default safe dev command should keep the Rust watcher.'

$noWatchDev = Invoke-PrepareDevData -NoRustWatch
Assert-Equal $noWatchDev.ExitCode 0 'The safe no-watch dry-run should succeed.'
Assert-Contains $noWatchDev.Output 'npm.cmd run dev -- --no-watch' 'The safe no-watch command should forward the Tauri flag.'
Assert-Contains $noWatchDev.Output 'dev-data\codex' 'The safe no-watch command should keep the isolated Codex path.'
Assert-Contains $noWatchDev.Output 'dev-data\app-data' 'The safe no-watch command should keep the isolated app-data path.'

$packageJson = Get-Content -Raw (Join-Path $workspace 'package.json') | ConvertFrom-Json
$rootCargoManifestPath = Join-Path $workspace 'src-tauri/Cargo.toml'
$coreCargoManifestPath = Join-Path $workspace 'src-tauri/crates/codex-relay-core/Cargo.toml'
Assert-Equal $packageJson.scripts.'dev:safe:no-watch' 'powershell -ExecutionPolicy Bypass -File scripts/prepare-dev-data.ps1 -NoRustWatch' 'The package script should reuse the safe data preparation entry point.'
Assert-Contains $packageJson.scripts.'test:rust:lib' '--target-dir src-tauri/target -p codex-relay-core --lib' 'The fast lib test should compile and link only the core package in the stable target directory.'
Assert-Contains $packageJson.scripts.'test:rust:path-safety' '--target-dir src-tauri/target -p codex-relay-core --test path_safety' 'The path-safety test should compile and link only the core package.'
Assert-Contains $packageJson.scripts.'test:rust:provider-workflow' '--target-dir src-tauri/target -p codex-relay-core --test provider_workflow' 'The provider workflow test should compile and link only the core package.'
Assert-Contains $packageJson.scripts.'check:rust' 'cargo fmt --all --check --manifest-path src-tauri/Cargo.toml' 'The full Rust check should format every workspace package.'
Assert-Contains $packageJson.scripts.'check:rust' 'cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets --all-features' 'The full Rust check should lint every workspace package and target.'
Assert-Contains $packageJson.scripts.'check:rust' 'cargo test --manifest-path src-tauri/Cargo.toml --workspace' 'The full Rust check should run tests for every workspace package.'
Assert-Equal (Test-Path -LiteralPath $coreCargoManifestPath -PathType Leaf) $true 'The codex-relay-core manifest should exist.'
$rootCargoManifest = Get-Content -Raw -LiteralPath $rootCargoManifestPath
$coreCargoManifest = Get-Content -Raw -LiteralPath $coreCargoManifestPath
Assert-Contains $rootCargoManifest '[workspace]' 'The Tauri manifest should own the Cargo workspace.'
Assert-Contains $rootCargoManifest '"crates/codex-relay-core"' 'The Cargo workspace should include codex-relay-core.'
Assert-NotContains $coreCargoManifest 'tauri' 'The core manifest should not directly depend on Tauri.'

Write-Host 'rust-dev-scripts: 12 tests passed'
