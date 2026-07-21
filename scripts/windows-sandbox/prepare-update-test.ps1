[CmdletBinding()]
param(
  [Parameter(Mandatory)]
  [string]$InstallerPath,

  [Parameter(Mandatory)]
  [ValidatePattern('^[A-Fa-f0-9]{64}$')]
  [string]$ExpectedSha256,

  [Parameter(Mandatory)]
  [ValidatePattern('^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$')]
  [string]$ExpectedTargetVersion,

  [string]$StageRoot = (Join-Path ([System.IO.Path]::GetTempPath()) ("codex-relay-sandbox-{0}" -f [guid]::NewGuid().ToString('N'))),

  [switch]$PrepareOnly
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

function Assert-NoReparsePoint {
  param(
    [Parameter(Mandatory)][string]$Candidate,
    [Parameter(Mandatory)][string]$Root
  )

  $currentPath = Get-CanonicalPath -Path $Candidate
  $rootPath = Get-CanonicalPath -Path $Root
  while ($currentPath -ne $rootPath) {
    if (Test-Path -LiteralPath $currentPath) {
      $item = Get-Item -LiteralPath $currentPath -Force
      if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw 'SANDBOX_STAGE_ROOT_REPARSE_POINT'
      }
    }

    $parent = [System.IO.Directory]::GetParent($currentPath)
    if ($null -eq $parent) {
      throw 'SANDBOX_STAGE_ROOT_MUST_BE_TEMPORARY'
    }
    $currentPath = Get-CanonicalPath -Path $parent.FullName
  }
}

$stageRootPath = Get-CanonicalPath -Path $StageRoot
$tempRootPath = Get-CanonicalPath -Path ([System.IO.Path]::GetTempPath())
if (-not (Test-IsWithinPath -Candidate $stageRootPath -Root $tempRootPath)) {
  throw 'SANDBOX_STAGE_ROOT_MUST_BE_TEMPORARY'
}
Assert-NoReparsePoint -Candidate $stageRootPath -Root $tempRootPath

if (Test-Path -LiteralPath $stageRootPath) {
  $existingEntries = @(Get-ChildItem -LiteralPath $stageRootPath -Force)
  if ($existingEntries.Count -gt 0) {
    throw 'SANDBOX_STAGE_ROOT_NOT_EMPTY'
  }
}

$installerPathValue = Get-CanonicalPath -Path $InstallerPath
$protectedRoots = @(
  (Join-Path $env:USERPROFILE '.codex'),
  (Join-Path $env:LOCALAPPDATA 'CodexRelay')
)
foreach ($protectedRoot in $protectedRoots) {
  $protectedRootValue = Get-CanonicalPath -Path $protectedRoot
  if (
    $installerPathValue -eq $protectedRootValue -or
    (Test-IsWithinPath -Candidate $installerPathValue -Root $protectedRootValue)
  ) {
    throw 'SANDBOX_INSTALLER_PATH_UNSAFE'
  }
}

$resolvedInstaller = (Resolve-Path -LiteralPath $InstallerPath -ErrorAction Stop).Path
if (-not (Test-Path -LiteralPath $resolvedInstaller -PathType Leaf)) {
  throw 'SANDBOX_INSTALLER_NOT_FOUND'
}

$actualSha256 = (Get-FileSha256 -Path $resolvedInstaller).ToUpperInvariant()
$normalizedExpectedSha256 = $ExpectedSha256.ToUpperInvariant()
if ($actualSha256 -ne $normalizedExpectedSha256) {
  throw 'SANDBOX_INSTALLER_HASH_MISMATCH'
}

$inputPath = Join-Path $stageRootPath 'input'
$resultsPath = Join-Path $stageRootPath 'results'
New-Item -ItemType Directory -Force -Path $inputPath, $resultsPath | Out-Null

$installerFileName = [System.IO.Path]::GetFileName($resolvedInstaller)
Copy-Item -LiteralPath $resolvedInstaller -Destination (Join-Path $inputPath $installerFileName)
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'common.ps1') -Destination $inputPath
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'guest-bootstrap.ps1') -Destination $inputPath
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'guest-start-app.ps1') -Destination $inputPath
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'guest-verify.ps1') -Destination $inputPath
Copy-Item -LiteralPath (Join-Path (Split-Path -Parent $PSScriptRoot) 'prepare-dev-data.ps1') -Destination $inputPath

$configPath = Join-Path $stageRootPath 'codex-relay-update-test.wsb'
$settings = [System.Xml.XmlWriterSettings]::new()
$settings.Encoding = [System.Text.UTF8Encoding]::new($false)
$settings.Indent = $true
$settings.NewLineChars = [Environment]::NewLine

$writer = [System.Xml.XmlWriter]::Create($configPath, $settings)
try {
  $writer.WriteStartDocument()
  $writer.WriteStartElement('Configuration')

  $writer.WriteElementString('VGpu', 'Disable')
  $writer.WriteElementString('Networking', 'Default')
  $writer.WriteElementString('ClipboardRedirection', 'Disable')
  $writer.WriteElementString('PrinterRedirection', 'Disable')

  $writer.WriteStartElement('MappedFolders')
  foreach ($mapping in @(
      [ordered]@{
        HostFolder = $inputPath
        SandboxFolder = 'C:\CodexRelaySandbox\Input'
        ReadOnly = 'true'
      },
      [ordered]@{
        HostFolder = $resultsPath
        SandboxFolder = 'C:\CodexRelaySandbox\Results'
        ReadOnly = 'false'
      }
    )) {
    $writer.WriteStartElement('MappedFolder')
    foreach ($name in $mapping.Keys) {
      $writer.WriteElementString($name, $mapping[$name])
    }
    $writer.WriteEndElement()
  }
  $writer.WriteEndElement()

  $guestInstallerPath = "C:\CodexRelaySandbox\Input\$installerFileName"
  $command = 'powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "C:\CodexRelaySandbox\Input\guest-bootstrap.ps1" -InstallerPath "{0}" -ResultsPath "C:\CodexRelaySandbox\Results" -ExpectedTargetVersion "{1}"' -f $guestInstallerPath, $ExpectedTargetVersion
  $writer.WriteStartElement('LogonCommand')
  $writer.WriteElementString('Command', $command)
  $writer.WriteEndElement()

  $writer.WriteEndElement()
  $writer.WriteEndDocument()
}
finally {
  $writer.Dispose()
}

$output = [ordered]@{
  stageRoot = $stageRootPath
  configPath = $configPath
  inputPath = $inputPath
  resultsPath = $resultsPath
  installer = [ordered]@{
    fileName = $installerFileName
    sha256 = $actualSha256
  }
  expectedTargetVersion = $ExpectedTargetVersion
}

if (-not $PrepareOnly) {
  Start-Process -FilePath $configPath
}

$output | ConvertTo-Json -Depth 4
