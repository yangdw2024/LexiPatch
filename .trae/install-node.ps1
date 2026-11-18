$ProgressPreference = 'SilentlyContinue'
$ErrorActionPreference = 'Stop'

$ver = 'v22.11.0'
$arch = 'win-x64'
$url = "https://nodejs.org/dist/$ver/node-$ver-$arch.zip"
$dest = 'D:\nodejs'
$zip = "$env:TEMP\node-$ver-$arch.zip"

Write-Output "Downloading $url"
try {
  Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing -TimeoutSec 120
} catch {
  Write-Output ("DOWNLOAD_ERR: " + $_.Exception.Message)
  exit 2
}

if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
New-Item -ItemType Directory -Force -Path $dest | Out-Null

Write-Output "Extracting to $dest"
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zipObj = [System.IO.Compression.ZipFile]::OpenRead($zip)
try {
  foreach ($entry in $zipObj.Entries) {
    # strip top folder "node-v22.11.0-win-x64/"
    $rel = $entry.FullName -replace '^[^/]+/', ''
    if ([string]::IsNullOrEmpty($rel)) { continue }
    $target = Join-Path $dest $rel
    $dir = Split-Path $target -Parent
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    if ($entry.FullName.EndsWith('/')) { continue }
    [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $target, $true)
  }
} finally {
  $zipObj.Dispose()
}

Remove-Item $zip -Force -ErrorAction SilentlyContinue

Write-Output "Installed. node path: $dest\node.exe"
& "$dest\node.exe" --version
& "$dest\npm.cmd" --version
