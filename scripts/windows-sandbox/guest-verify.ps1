[CmdletBinding()]
param(
  [Parameter(Mandatory)]
  [string]$ResultsPath,

  [Parameter(Mandatory)]
  [ValidatePattern('^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')]
  [string]$ExpectedVersion,

  [switch]$DryRun,

  [string]$InstalledVersion,

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
  if ([string]::IsNullOrWhiteSpace($InstalledVersion) -or [string]::IsNullOrWhiteSpace($InstallLocation)) {
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
  if ($PSBoundParameters.ContainsKey('InstalledVersion') -or $PSBoundParameters.ContainsKey('InstallLocation')) {
    throw 'SANDBOX_REAL_VERIFICATION_CANNOT_OVERRIDE_INSTALLATION'
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

  $InstalledVersion = [string]$installations[0].DisplayVersion
  $InstallLocation = [string]$installations[0].InstallLocation
}

$installLocationValue = Get-CanonicalInstallLocation -Path $InstallLocation
if ($DryRun -and -not (Test-IsWithinPath -Candidate $installLocationValue -Root $sandboxRoot)) {
  throw 'SANDBOX_DRY_RUN_INSTALLATION_OUTSIDE_STAGE'
}

$executablePath = Join-Path $installLocationValue 'CodexRelay.exe'
$executablePresent = Test-Path -LiteralPath $executablePath -PathType Leaf

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
  $codexHome -eq $appData
) {
  throw 'SANDBOX_BEFORE_REPORT_PATH_UNSAFE'
}

$allowedRelativePaths = @(
  'codex/auth.json',
  'codex/config.toml',
  'app-data/providers.json'
)
$beforeFiles = @($before.protectedFiles)
$reportedRelativePaths = @($beforeFiles | ForEach-Object { [string]$_.relativePath })
$uniqueRelativePaths = @($reportedRelativePaths | Sort-Object -Unique)
if (
  $beforeFiles.Count -ne $allowedRelativePaths.Count -or
  $uniqueRelativePaths.Count -ne $allowedRelativePaths.Count -or
  @($reportedRelativePaths | Where-Object { $_ -notin $allowedRelativePaths }).Count -gt 0 -or
  @($allowedRelativePaths | Where-Object { $_ -notin $reportedRelativePaths }).Count -gt 0
) {
  throw 'SANDBOX_BEFORE_REPORT_FILES_INVALID'
}

$protectedFiles = @(foreach ($beforeFile in $beforeFiles) {
    $relativePath = [string]$beforeFile.relativePath
    $currentPath = Get-CanonicalPath -Path (Join-Path $devDataRoot $relativePath.Replace('/', '\'))
    if (-not (Test-IsWithinPath -Candidate $currentPath -Root $devDataRoot)) {
      throw 'SANDBOX_PROTECTED_FILE_PATH_UNSAFE'
    }

    $exists = Test-Path -LiteralPath $currentPath -PathType Leaf
    $currentLength = $null
    $currentSha256 = $null
    if ($exists) {
      $item = Get-Item -LiteralPath $currentPath
      $currentLength = $item.Length
      $currentSha256 = Get-FileSha256 -Path $item.FullName
    }

    $matchesBefore = (
      $exists -and
      $currentLength -eq [long]$beforeFile.length -and
      $currentSha256 -eq [string]$beforeFile.sha256
    )
    [ordered]@{
      relativePath = $relativePath
      exists = $exists
      beforeLength = [long]$beforeFile.length
      afterLength = $currentLength
      beforeSha256 = [string]$beforeFile.sha256
      afterSha256 = $currentSha256
      matchesBefore = $matchesBefore
    }
  })

$dataPreserved = @($protectedFiles | Where-Object { -not $_.matchesBefore }).Count -eq 0
$versionMatched = $InstalledVersion -eq $ExpectedVersion
$success = $dataPreserved -and $versionMatched -and $executablePresent

$report = [ordered]@{
  schemaVersion = 1
  phase = 'after'
  createdAt = [DateTimeOffset]::UtcNow.ToString('o')
  expectedVersion = $ExpectedVersion
  installedVersion = $InstalledVersion
  versionMatched = $versionMatched
  installLocation = $installLocationValue
  executablePath = $executablePath
  executablePresent = $executablePresent
  dataPreserved = $dataPreserved
  protectedFiles = $protectedFiles
  success = $success
}
$reportPath = Join-Path $resultsPathValue 'after.json'
$reportJson = $report | ConvertTo-Json -Depth 7
[System.IO.File]::WriteAllText(
  $reportPath,
  $reportJson + [Environment]::NewLine,
  [System.Text.UTF8Encoding]::new($false)
)

$output = [ordered]@{
  reportPath = $reportPath
  dataPreserved = $dataPreserved
  versionMatched = $versionMatched
  executablePresent = $executablePresent
  success = $success
}
$output | ConvertTo-Json -Depth 4

if (-not $success) {
  throw 'SANDBOX_UPDATE_VERIFICATION_FAILED'
}
