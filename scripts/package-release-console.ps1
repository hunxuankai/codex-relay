[CmdletBinding()]
param(
  [string]$SourcePath,
  [string]$DestinationDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ([string]::IsNullOrWhiteSpace($SourcePath)) {
  $SourcePath = Join-Path $repositoryRoot 'src-tauri\target\release\CodexRelayReleaseConsole.exe'
}
if ([string]::IsNullOrWhiteSpace($DestinationDirectory)) {
  $DestinationDirectory = Join-Path $repositoryRoot 'dist\release-console'
}

$source = [System.IO.Path]::GetFullPath($SourcePath)
if (-not [System.IO.File]::Exists($source)) {
  throw "发布控制台 EXE 不存在：$source"
}
if (-not [System.IO.Path]::GetFileName($source).Equals(
    'CodexRelayReleaseConsole.exe',
    [System.StringComparison]::OrdinalIgnoreCase
  )) {
  throw '发布控制台源文件名必须为 CodexRelayReleaseConsole.exe。'
}

$destinationRoot = [System.IO.Path]::GetFullPath($DestinationDirectory)
[System.IO.Directory]::CreateDirectory($destinationRoot) | Out-Null
$destination = Join-Path $destinationRoot 'CodexRelayReleaseConsole.exe'
[System.IO.File]::Copy($source, $destination, $true)

$artifact = Get-Item -LiteralPath $destination
$hash = Get-FileHash -LiteralPath $destination -Algorithm SHA256
[ordered]@{
  path = $artifact.FullName
  size = $artifact.Length
  lastWriteTime = $artifact.LastWriteTime.ToString('o')
  sha256 = $hash.Hash
} | ConvertTo-Json -Compress
