#requires -Version 7.0

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$')]
    [string]$ExpectedVersion,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$ExpectedSha,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$ActualSha,

    [string]$Workspace = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,

    [string]$GitHubOutput = $env:GITHUB_OUTPUT
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-CargoManifestVersion {
    param([Parameter(Mandatory = $true)][string]$Path)

    $content = Get-Content -LiteralPath $Path -Raw
    $match = [regex]::Match(
        $content,
        '(?ms)^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"'
    )
    if (-not $match.Success) {
        throw "无法从 Cargo manifest 读取 package.version: $Path"
    }
    return $match.Groups[1].Value
}

function Get-CargoLockVersion {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$PackageName
    )

    $content = Get-Content -LiteralPath $Path -Raw
    $escapedName = [regex]::Escape($PackageName)
    $pattern = "(?ms)^\[\[package\]\]\s*\r?\nname\s*=\s*`"$escapedName`"\s*\r?\nversion\s*=\s*`"([^`"]+)`""
    $match = [regex]::Match($content, $pattern)
    if (-not $match.Success) {
        throw "无法从 Cargo.lock 读取 package.version: $PackageName"
    }
    return $match.Groups[1].Value
}

if ($ExpectedSha.ToLowerInvariant() -ne $ActualSha.ToLowerInvariant()) {
    throw "发布候选 SHA 与工作流实际 SHA 不一致。expected=$ExpectedSha actual=$ActualSha"
}

$packageJsonPath = Join-Path $Workspace 'package.json'
$packageLockPath = Join-Path $Workspace 'package-lock.json'
$cargoManifestPath = Join-Path $Workspace 'src-tauri/Cargo.toml'
$coreCargoManifestPath = Join-Path $Workspace 'src-tauri/crates/codex-relay-core/Cargo.toml'
$cargoLockPath = Join-Path $Workspace 'src-tauri/Cargo.lock'
$releaseNotesPath = Join-Path $Workspace '.github/release-notes.md'

$packageJson = Get-Content -LiteralPath $packageJsonPath -Raw | ConvertFrom-Json -AsHashtable
$packageLock = Get-Content -LiteralPath $packageLockPath -Raw | ConvertFrom-Json -AsHashtable
$versions = [ordered]@{
    packageJson = [string]$packageJson['version']
    packageLock = [string]$packageLock['version']
    packageLockRoot = [string]$packageLock['packages']['']['version']
    cargo = Get-CargoManifestVersion -Path $cargoManifestPath
    cargoCore = Get-CargoManifestVersion -Path $coreCargoManifestPath
    cargoLock = Get-CargoLockVersion -Path $cargoLockPath -PackageName 'codex-relay'
    cargoLockCore = Get-CargoLockVersion -Path $cargoLockPath -PackageName 'codex-relay-core'
}

foreach ($entry in $versions.GetEnumerator()) {
    if ($entry.Value -ne $ExpectedVersion) {
        throw "发布版本不一致：$($entry.Key)=$($entry.Value)，expected=$ExpectedVersion"
    }
}

if (-not (Test-Path -LiteralPath $releaseNotesPath -PathType Leaf)) {
    throw '缺少 .github/release-notes.md。'
}
$releaseBody = Get-Content -LiteralPath $releaseNotesPath -Raw
if ([string]::IsNullOrWhiteSpace($releaseBody)) {
    throw '发布说明不能为空。'
}

$requiredText = @(
    '## 更新内容',
    '## 更新方式',
    '## 注意事项',
    "v$ExpectedVersion",
    'Windows 可能显示“未知发布者”',
    '安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份'
)
foreach ($text in $requiredText) {
    if (-not $releaseBody.Contains($text)) {
        throw "发布说明缺少必需内容：$text"
    }
}
if ($releaseBody.Contains('请在发布前补充本版本的变更说明')) {
    throw '发布说明仍包含占位文案。'
}

$secretPatterns = @(
    '(?i)\bAuthorization\s*[:=]\s*Bearer\s+\S+',
    '(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{8,}',
    '(?i)\b(?:OPENAI_API_KEY|GH_TOKEN|GITHUB_TOKEN|TAURI_SIGNING_PRIVATE_KEY|TAURI_SIGNING_PRIVATE_KEY_PASSWORD)\b\s*[:=]\s*\S+',
    '(?i)\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})\b'
)
foreach ($pattern in $secretPatterns) {
    if ([regex]::IsMatch($releaseBody, $pattern)) {
        throw 'RELEASE_NOTES_SECRET_DETECTED: 发布说明包含疑似秘密，已停止发布。'
    }
}

if ([string]::IsNullOrWhiteSpace($GitHubOutput)) {
    throw 'GITHUB_OUTPUT 未设置，无法传递发布说明。'
}
$delimiter = "release-body-$([guid]::NewGuid().ToString('N'))"
$output = "release_body<<$delimiter`n$($releaseBody.TrimEnd())`n$delimiter`n"
[System.IO.File]::AppendAllText(
    $GitHubOutput,
    $output,
    [System.Text.UTF8Encoding]::new($false)
)

Write-Output "发布请求验证通过：version=$ExpectedVersion sha=$ActualSha"
