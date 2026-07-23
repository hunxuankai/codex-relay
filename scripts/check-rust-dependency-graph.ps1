[CmdletBinding()]
param(
  [string]$EncodedTreeOutput
)

$ErrorActionPreference = 'Stop'

if ($PSBoundParameters.ContainsKey('EncodedTreeOutput')) {
  try {
    $treeOutput = [Text.Encoding]::UTF8.GetString(
      [Convert]::FromBase64String($EncodedTreeOutput)
    )
  } catch {
    Write-Error '无法解析依赖图测试输入。'
    exit 1
  }
} else {
  $workspace = Split-Path -Parent $PSScriptRoot
  $manifest = Join-Path $workspace 'src-tauri/Cargo.toml'
  $treeLines = & cargo tree `
    --manifest-path $manifest `
    -e features `
    --prefix none `
    --format '{p} [{f}]' 2>&1

  if ($LASTEXITCODE -ne 0) {
    $treeLines | ForEach-Object { Write-Host $_ }
    Write-Error '无法生成 Rust 依赖图。'
    exit 1
  }

  $treeOutput = $treeLines -join [Environment]::NewLine
}

$lines = @($treeOutput -split '\r?\n')
$hasAwsLc = @($lines | Where-Object { $_ -match '^aws-lc-sys(?:\s|$)' }).Count -gt 0
if ($hasAwsLc) {
  Write-Host 'Rust 依赖图仍包含 aws-lc-sys；reqwest 与 updater 的 TLS provider 尚未收敛。' -ForegroundColor Yellow
  exit 2
}

$rustlsLines = @($lines | Where-Object { $_ -match '^rustls v' })
$hasRing = @($rustlsLines | Where-Object { $_ -match '\[[^\]]*\bring\b[^\]]*\]' }).Count -gt 0
if (-not $hasRing) {
  Write-Host 'Rust 依赖图没有显式启用 rustls ring provider。' -ForegroundColor Yellow
  exit 3
}

Write-Host 'Rust 依赖图已收敛：ring provider 已启用，aws-lc-sys 不存在。'
exit 0
