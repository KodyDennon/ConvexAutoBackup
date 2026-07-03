param(
  [string]$Version = $env:CONVEX_AUTOBACKUP_VERSION,
  [string]$Repo = "KodyDennon/ConvexAutoBackup",
  [string]$InstallRoot = "$env:LOCALAPPDATA\ConvexAutoBackup",
  [string]$DataDir = "$env:LOCALAPPDATA\ConvexAutoBackup\data",
  [string]$Bind = "0.0.0.0:8976",
  [switch]$NoAutostart
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($Version)) {
  $Version = "v0.1.0-beta.1"
}

$asset = "convex-autobackup-windows-x86_64.zip"
$baseUrl = "https://github.com/$Repo/releases/download/$Version"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("convex-autobackup-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $tmp, $InstallRoot, $DataDir | Out-Null

try {
  $assetPath = Join-Path $tmp $asset
  $sumsPath = Join-Path $tmp "SHA256SUMS"
  Invoke-WebRequest -Uri "$baseUrl/$asset" -OutFile $assetPath
  Invoke-WebRequest -Uri "$baseUrl/SHA256SUMS" -OutFile $sumsPath

  $expectedLine = Get-Content $sumsPath | Where-Object { $_ -match [regex]::Escape($asset) } | Select-Object -First 1
  if (-not $expectedLine) {
    throw "No checksum entry found for $asset"
  }
  $expected = ($expectedLine -split "\s+")[0].ToLowerInvariant()
  $actual = (Get-FileHash -Algorithm SHA256 $assetPath).Hash.ToLowerInvariant()
  if ($actual -ne $expected) {
    throw "Checksum mismatch for $asset"
  }

  Expand-Archive -Path $assetPath -DestinationPath $InstallRoot -Force

  $envFile = Join-Path $InstallRoot "convex-autobackup.env.ps1"
  if (-not (Test-Path $envFile)) {
    $bytes = New-Object byte[] 48
    [System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
    $masterKey = [Convert]::ToBase64String($bytes)
    @"
`$env:CONVEX_AUTOBACKUP_DATA_DIR = "$DataDir"
`$env:CONVEX_AUTOBACKUP_MASTER_KEY = "$masterKey"
`$env:CONVEX_AUTOBACKUP_BIND = "$Bind"
`$env:RUST_LOG = "info"
"@ | Set-Content -Encoding UTF8 $envFile
    Write-Host "Generated $envFile. Back up CONVEX_AUTOBACKUP_MASTER_KEY; losing it can make stored secrets unrecoverable."
  }

  . $envFile
  $exe = Join-Path $InstallRoot "convex-autobackup.exe"
  & $exe runner install --json

  $serviceScript = Join-Path $InstallRoot "convex-autobackup-service.ps1"
  @"
. "$envFile"
& "$exe" supervise
"@ | Set-Content -Encoding UTF8 $serviceScript

  if (-not $NoAutostart) {
    $serviceName = "ConvexAutoBackup"
    $binPath = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$serviceScript`""
    $existing = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
    if ($existing) {
      Stop-Service -Name $serviceName -ErrorAction SilentlyContinue
      sc.exe delete $serviceName | Out-Null
      Start-Sleep -Seconds 2
    }
    sc.exe create $serviceName binPath= "$binPath" start= auto DisplayName= "ConvexAutoBackup" | Out-Null
    sc.exe description $serviceName "Self-hosted Convex backup and disaster recovery control plane" | Out-Null
    Start-Service -Name $serviceName
    Start-Sleep -Seconds 5
    & $exe doctor --json
  } else {
    & $exe runner status --json
  }

  Write-Host "ConvexAutoBackup installed."
  Write-Host "URL: http://localhost:8976"
  Write-Host "Install root: $InstallRoot"
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
