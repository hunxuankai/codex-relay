$ErrorActionPreference = 'Stop'

$workspace = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$checkScript = Join-Path $workspace 'scripts/check-rust-dependency-graph.ps1'

function Invoke-DependencyCheck {
  param(
    [string]$TreeOutput,
    [string]$CoreTreeOutput
  )

  $encodedTree = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($TreeOutput))
  $arguments = @(
    '-NoProfile',
    '-ExecutionPolicy',
    'Bypass',
    '-File',
    $checkScript,
    '-EncodedTreeOutput',
    $encodedTree
  )
  if ($PSBoundParameters.ContainsKey('CoreTreeOutput')) {
    $encodedCoreTree = [Convert]::ToBase64String(
      [Text.Encoding]::UTF8.GetBytes($CoreTreeOutput)
    )
    $arguments += '-EncodedCoreTreeOutput'
    $arguments += $encodedCoreTree
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

$duplicatedProvider = Invoke-DependencyCheck @'
rustls v0.23.42 [aws-lc-rs,ring,std,tls12]
aws-lc-sys v0.43.0 [prebuilt-nasm]
'@
Assert-Equal $duplicatedProvider.ExitCode 2 'aws-lc-sys should fail the dependency graph check.'
Assert-Contains $duplicatedProvider.Output 'aws-lc-sys' 'The failure should identify the duplicate provider.'

$missingRing = Invoke-DependencyCheck @'
rustls v0.23.42 [std,tls12]
'@
Assert-Equal $missingRing.ExitCode 3 'A missing ring provider should fail the dependency graph check.'
Assert-Contains $missingRing.Output 'ring' 'The failure should identify the missing ring provider.'

$ringOnly = Invoke-DependencyCheck @'
rustls v0.23.42 [ring,std,tls12]
'@
Assert-Equal $ringOnly.ExitCode 0 'A ring-only Rustls dependency graph should pass.'

$coreWithTauri = Invoke-DependencyCheck -TreeOutput @'
rustls v0.23.42 [ring,std,tls12]
'@ -CoreTreeOutput @'
codex-relay-core v0.1.2
tauri v2.11.5
'@
Assert-Equal $coreWithTauri.ExitCode 4 'A core dependency graph containing Tauri should fail.'
Assert-Contains $coreWithTauri.Output 'codex-relay-core' 'The failure should identify the core boundary.'

$coreWithoutTauri = Invoke-DependencyCheck -TreeOutput @'
rustls v0.23.42 [ring,std,tls12]
'@ -CoreTreeOutput @'
codex-relay-core v0.1.2
reqwest v0.13.4
rustls v0.23.42
'@
Assert-Equal $coreWithoutTauri.ExitCode 0 'A Tauri-free core dependency graph should pass.'

Write-Host 'rust-dependency-graph: 5 tests passed'
