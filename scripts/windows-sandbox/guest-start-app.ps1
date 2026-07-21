[CmdletBinding()]
param(
  [Parameter(Mandatory)]
  [string]$ResultsPath,

  [switch]$DryRun,

  [string]$InstallLocation
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$sandboxRoot = Get-CanonicalPath -Path (Split-Path -Parent $PSScriptRoot)
$resultsPathValue = Get-CanonicalPath -Path $ResultsPath
if ($resultsPathValue -ne (Get-CanonicalPath -Path (Join-Path $sandboxRoot 'results'))) {
  throw 'SANDBOX_RESULTS_PATH_UNSAFE'
}

if ($DryRun) {
  $tempRoot = Get-CanonicalPath -Path ([System.IO.Path]::GetTempPath())
  if (-not (Test-IsWithinPath -Candidate $sandboxRoot -Root $tempRoot)) {
    throw 'SANDBOX_DRY_RUN_ROOT_MUST_BE_TEMPORARY'
  }
  if ([string]::IsNullOrWhiteSpace($InstallLocation)) {
    throw 'SANDBOX_DRY_RUN_INSTALLATION_REQUIRED'
  }
}
else {
  if (
    $env:USERNAME -ne 'WDAGUtilityAccount' -or
    $sandboxRoot -ne 'C:\CodexRelaySandbox'
  ) {
    throw 'SANDBOX_GUEST_REQUIRED'
  }
  if ($PSBoundParameters.ContainsKey('InstallLocation')) {
    throw 'SANDBOX_REAL_START_CANNOT_OVERRIDE_INSTALLATION'
  }

  $uninstallRoots = @(
    'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
  )
  $installations = @(Get-ItemProperty -Path $uninstallRoots -ErrorAction SilentlyContinue | Where-Object {
      $_.DisplayName -eq 'Codex Relay'
    })
  if ($installations.Count -ne 1) {
    throw 'SANDBOX_CODEX_RELAY_INSTALLATION_NOT_UNIQUE'
  }
  $InstallLocation = [string]$installations[0].InstallLocation
}

$installLocationValue = Get-CanonicalInstallLocation -Path $InstallLocation
if ($DryRun -and -not (Test-IsWithinPath -Candidate $installLocationValue -Root $sandboxRoot)) {
  throw 'SANDBOX_DRY_RUN_INSTALLATION_OUTSIDE_STAGE'
}

$beforeReportPath = Join-Path $resultsPathValue 'before.json'
if (-not (Test-Path -LiteralPath $beforeReportPath -PathType Leaf)) {
  throw 'SANDBOX_BEFORE_REPORT_NOT_FOUND'
}
$before = Get-Content -Raw -LiteralPath $beforeReportPath | ConvertFrom-Json
if ($before.schemaVersion -ne 1 -or $before.phase -ne 'before') {
  throw 'SANDBOX_BEFORE_REPORT_INVALID'
}

$codexHome = Get-CanonicalPath -Path ([string]$before.overrides.codexHome)
$appData = Get-CanonicalPath -Path ([string]$before.overrides.appData)
$devDataRoot = Get-CanonicalPath -Path (Join-Path $sandboxRoot 'dev-data')
if (
  -not (Test-IsWithinPath -Candidate $codexHome -Root $devDataRoot) -or
  -not (Test-IsWithinPath -Candidate $appData -Root $devDataRoot) -or
  $codexHome -eq $appData -or
  -not (Test-Path -LiteralPath $codexHome -PathType Container) -or
  -not (Test-Path -LiteralPath $appData -PathType Container)
) {
  throw 'SANDBOX_BEFORE_REPORT_PATH_UNSAFE'
}

$executablePath = Join-Path $installLocationValue 'CodexRelay.exe'
if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
  throw 'SANDBOX_CODEX_RELAY_EXECUTABLE_NOT_FOUND'
}

$env:CODEX_RELAY_CODEX_HOME = $codexHome
$env:CODEX_RELAY_APP_DATA_DIR = $appData
if (-not $DryRun) {
  [Environment]::SetEnvironmentVariable('CODEX_RELAY_CODEX_HOME', $codexHome, 'User')
  [Environment]::SetEnvironmentVariable('CODEX_RELAY_APP_DATA_DIR', $appData, 'User')
  Start-Process -FilePath $executablePath
}

[ordered]@{
  codexHome = $codexHome
  appData = $appData
  executablePath = $executablePath
  launched = -not $DryRun
} | ConvertTo-Json -Depth 4
