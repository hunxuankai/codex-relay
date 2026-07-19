param(
  [switch]$PrepareOnly
)

$ErrorActionPreference = 'Stop'

$workspace = Split-Path -Parent $PSScriptRoot
$devRoot = Join-Path $workspace 'dev-data'
$codexHome = Join-Path $devRoot 'codex'
$appData = Join-Path $devRoot 'app-data'

New-Item -ItemType Directory -Force -Path $codexHome, $appData | Out-Null

$config = @'
model_provider = "provider-a"

[model_providers.provider-a]
name = "Provider A"
base_url = "https://provider-a.example.test/v1"
wire_api = "responses"

[model_providers.provider-b]
name = "Provider B"
base_url = "https://provider-b.example.test/v1"
wire_api = "responses"
'@

$auth = @'
{
  "OPENAI_API_KEY": "test-key-not-real"
}
'@

$providers = @'
{
  "version": 1,
  "providers": {
    "provider-a": {
      "apiKey": "test-key-not-real"
    },
    "provider-b": {
      "apiKey": "test-key-b-not-real"
    }
  }
}
'@

$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText(
  (Join-Path $codexHome 'config.toml'),
  $config + [Environment]::NewLine,
  $utf8NoBom
)
[System.IO.File]::WriteAllText(
  (Join-Path $codexHome 'auth.json'),
  $auth + [Environment]::NewLine,
  $utf8NoBom
)
[System.IO.File]::WriteAllText(
  (Join-Path $appData 'providers.json'),
  $providers + [Environment]::NewLine,
  $utf8NoBom
)

$env:CODEX_RELAY_CODEX_HOME = $codexHome
$env:CODEX_RELAY_APP_DATA_DIR = $appData

Write-Host "Safe Codex directory: $codexHome"
Write-Host "Safe application data: $appData"

if (-not $PrepareOnly) {
  & npm run dev
  exit $LASTEXITCODE
}
