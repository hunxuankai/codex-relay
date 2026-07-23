[CmdletBinding()]
param(
  [switch]$UseObservedCommandLine,
  [string[]]$ObservedCommandLine = @()
)

$ErrorActionPreference = 'Stop'

function Test-IsWatchingTauriDev {
  param(
    [AllowEmptyString()]
    [string]$CommandLine
  )

  if ([string]::IsNullOrWhiteSpace($CommandLine)) {
    return $false
  }

  $isTauriCli = $CommandLine.IndexOf('tauri.js', [StringComparison]::OrdinalIgnoreCase) -ge 0
  $isDev = [regex]::IsMatch($CommandLine, '(?i)(?:^|\s)dev(?:\s|$)')
  $isNoWatch = [regex]::IsMatch($CommandLine, '(?i)(?:^|\s)--no-watch(?:\s|$)')

  return $isTauriCli -and $isDev -and -not $isNoWatch
}

if ($UseObservedCommandLine) {
  $commandLines = $ObservedCommandLine
} else {
  $commandLines = @(
    Get-CimInstance Win32_Process |
      Where-Object { $_.Name -eq 'node.exe' -and $_.CommandLine } |
      Select-Object -ExpandProperty CommandLine
  )
}

$watchingProcesses = @($commandLines | Where-Object { Test-IsWatchingTauriDev $_ })
if ($watchingProcesses.Count -gt 0) {
  Write-Host '检测到启用了 Rust watcher 的 Tauri dev。为避免同一源码变化触发两套 Cargo 编译，快速 Rust 测试已停止。' -ForegroundColor Yellow
  Write-Host '请改用 npm run dev:safe:no-watch，或仅运行 npm run dev:frontend。' -ForegroundColor Yellow
  exit 2
}

exit 0
