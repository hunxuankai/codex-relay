function Get-CanonicalPath {
  param([Parameter(Mandatory)][string]$Path)

  return [System.IO.Path]::GetFullPath($Path).TrimEnd(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
  )
}

function Get-CanonicalInstallLocation {
  param([Parameter(Mandatory)][string]$Path)

  $normalizedPath = $Path.Trim()
  if (
    $normalizedPath.Length -ge 2 -and
    $normalizedPath.StartsWith('"') -and
    $normalizedPath.EndsWith('"')
  ) {
    $normalizedPath = $normalizedPath.Substring(1, $normalizedPath.Length - 2).Trim()
  }
  if ([string]::IsNullOrWhiteSpace($normalizedPath) -or $normalizedPath.Contains('"')) {
    throw 'SANDBOX_INSTALL_LOCATION_INVALID'
  }

  return Get-CanonicalPath -Path $normalizedPath
}

function Test-IsWithinPath {
  param(
    [Parameter(Mandatory)][string]$Candidate,
    [Parameter(Mandatory)][string]$Root
  )

  $canonicalCandidate = Get-CanonicalPath -Path $Candidate
  $canonicalRoot = Get-CanonicalPath -Path $Root
  return $canonicalCandidate.StartsWith(
    $canonicalRoot + [System.IO.Path]::DirectorySeparatorChar,
    [System.StringComparison]::OrdinalIgnoreCase
  )
}

function Get-FileSha256 {
  param([Parameter(Mandatory)][string]$Path)

  $stream = [System.IO.File]::OpenRead($Path)
  $hasher = [System.Security.Cryptography.SHA256]::Create()
  try {
    $hashBytes = $hasher.ComputeHash($stream)
    return [System.BitConverter]::ToString($hashBytes).Replace('-', '')
  }
  finally {
    $hasher.Dispose()
    $stream.Dispose()
  }
}
