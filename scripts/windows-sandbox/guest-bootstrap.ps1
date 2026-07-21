param(
  [Parameter(Mandatory)]
  [string]$InstallerPath,

  [Parameter(Mandatory)]
  [string]$ResultsPath,

  [Parameter(Mandatory)]
  [ValidatePattern('^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')]
  [string]$ExpectedTargetVersion,

  [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$sandboxRoot = Get-CanonicalPath -Path (Split-Path -Parent $PSScriptRoot)
$installerPathValue = Get-CanonicalPath -Path $InstallerPath
$resultsPathValue = Get-CanonicalPath -Path $ResultsPath

if ($DryRun) {
  $tempRoot = Get-CanonicalPath -Path ([System.IO.Path]::GetTempPath())
  if (-not (Test-IsWithinPath -Candidate $sandboxRoot -Root $tempRoot)) {
    throw 'SANDBOX_DRY_RUN_ROOT_MUST_BE_TEMPORARY'
  }
}
elseif (
  $env:USERNAME -ne 'WDAGUtilityAccount' -or
  $sandboxRoot -ne 'C:\CodexRelaySandbox'
) {
  throw 'SANDBOX_GUEST_REQUIRED'
}

if (-not (Test-IsWithinPath -Candidate $installerPathValue -Root (Join-Path $sandboxRoot 'input'))) {
  throw 'SANDBOX_INSTALLER_OUTSIDE_INPUT'
}

if ($resultsPathValue -ne (Get-CanonicalPath -Path (Join-Path $sandboxRoot 'results'))) {
  throw 'SANDBOX_RESULTS_PATH_UNSAFE'
}

if (-not (Test-Path -LiteralPath $installerPathValue -PathType Leaf)) {
  throw 'SANDBOX_INSTALLER_NOT_FOUND'
}

if (-not (Test-Path -LiteralPath $resultsPathValue -PathType Container)) {
  throw 'SANDBOX_RESULTS_PATH_NOT_FOUND'
}

$prepareDevDataPath = Join-Path $PSScriptRoot 'prepare-dev-data.ps1'
if (-not (Test-Path -LiteralPath $prepareDevDataPath -PathType Leaf)) {
  throw 'SANDBOX_FIXTURE_SCRIPT_NOT_FOUND'
}

& $prepareDevDataPath -PrepareOnly *> $null

$codexHome = Get-CanonicalPath -Path (Join-Path $sandboxRoot 'dev-data\codex')
$appData = Get-CanonicalPath -Path (Join-Path $sandboxRoot 'dev-data\app-data')
$env:CODEX_RELAY_CODEX_HOME = $codexHome
$env:CODEX_RELAY_APP_DATA_DIR = $appData

if (-not $DryRun) {
  [Environment]::SetEnvironmentVariable('CODEX_RELAY_CODEX_HOME', $codexHome, 'User')
  [Environment]::SetEnvironmentVariable('CODEX_RELAY_APP_DATA_DIR', $appData, 'User')
}

$protectedFileDefinitions = @(
  [ordered]@{ relativePath = 'codex/auth.json'; path = (Join-Path $codexHome 'auth.json') },
  [ordered]@{ relativePath = 'codex/config.toml'; path = (Join-Path $codexHome 'config.toml') },
  [ordered]@{ relativePath = 'app-data/providers.json'; path = (Join-Path $appData 'providers.json') }
)
$protectedFiles = @($protectedFileDefinitions | ForEach-Object {
    $item = Get-Item -LiteralPath $_.path
    [ordered]@{
      relativePath = $_.relativePath
      length = $item.Length
      sha256 = Get-FileSha256 -Path $item.FullName
    }
  })

$report = [ordered]@{
  schemaVersion = 1
  phase = 'before'
  createdAt = [DateTimeOffset]::UtcNow.ToString('o')
  overrides = [ordered]@{
    codexHome = $codexHome
    appData = $appData
  }
  protectedFiles = $protectedFiles
}
$reportPath = Join-Path $resultsPathValue 'before.json'
$reportJson = $report | ConvertTo-Json -Depth 6
[System.IO.File]::WriteAllText(
  $reportPath,
  $reportJson + [Environment]::NewLine,
  [System.Text.UTF8Encoding]::new($false)
)

$desktopPath = if ($DryRun) {
  Join-Path $sandboxRoot 'desktop'
}
else {
  [Environment]::GetFolderPath('Desktop')
}
New-Item -ItemType Directory -Force -Path $desktopPath | Out-Null
$verifyLauncherPath = Join-Path $desktopPath 'Verify Codex Relay Update.cmd'
$verifyCommand = 'powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "{0}" -ResultsPath "{1}" -ExpectedVersion "{2}"' -f (
  (Join-Path $PSScriptRoot 'guest-verify.ps1'),
  $resultsPathValue,
  $ExpectedTargetVersion
)
$verifyLauncher = @(
  '@echo off',
  $verifyCommand,
  'pause'
) -join "`r`n"
[System.IO.File]::WriteAllText(
  $verifyLauncherPath,
  $verifyLauncher + "`r`n",
  [System.Text.UTF8Encoding]::new($false)
)

$startLauncherPath = Join-Path $desktopPath 'Start Codex Relay Safely.cmd'
$startCommand = 'powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "{0}" -ResultsPath "{1}"' -f (
  (Join-Path $PSScriptRoot 'guest-start-app.ps1'),
  $resultsPathValue
)
$startLauncher = @(
  '@echo off',
  $startCommand,
  'pause'
) -join "`r`n"
[System.IO.File]::WriteAllText(
  $startLauncherPath,
  $startLauncher + "`r`n",
  [System.Text.UTF8Encoding]::new($false)
)

if (-not $DryRun) {
  Start-Process -FilePath $installerPathValue
}

[ordered]@{
  codexHome = $codexHome
  appData = $appData
  reportPath = $reportPath
  verifyLauncherPath = $verifyLauncherPath
  startLauncherPath = $startLauncherPath
  installerLaunched = -not $DryRun
} | ConvertTo-Json -Depth 4
