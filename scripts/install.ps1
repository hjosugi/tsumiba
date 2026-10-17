<#
.SYNOPSIS
  Install tsumiba from a GitHub release (Windows).

.DESCRIPTION
  Downloads the Windows archive, checks it against the release's SHA256SUMS.txt,
  unpacks it under %LOCALAPPDATA%\Programs\tsumiba, adds that folder to your user
  PATH and creates a Start menu shortcut. No administrator rights needed.

  irm https://raw.githubusercontent.com/hjosugi/tsumiba/main/scripts/install.ps1 | iex

.PARAMETER Version
  A tag such as v0.1.0. Defaults to the latest release.

.PARAMETER InstallDir
  Where to unpack the program.
#>
[CmdletBinding()]
param(
  [string]$Version = "latest",
  [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\tsumiba")
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$base = if ($env:TSUMIBA_BASE_URL) { $env:TSUMIBA_BASE_URL } else { "https://github.com/hjosugi/tsumiba/releases" }
$releaseUrl = if ($Version -eq "latest") { "$base/latest/download" } else { "$base/download/$Version" }

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("tsumiba-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null
try {
  Write-Host "Looking up the release ($Version)..."
  $sums = Join-Path $work "SHA256SUMS.txt"
  Invoke-WebRequest -UseBasicParsing -Uri "$releaseUrl/SHA256SUMS.txt" -OutFile $sums

  $line = Get-Content $sums | Where-Object { $_ -match "tsumiba-[0-9][^ ]*-windows-x86_64\.zip\s*$" } | Select-Object -First 1
  if (-not $line) { throw "this release has no Windows archive" }
  $parts = $line -split "\s+", 2
  $expected = $parts[0].ToLowerInvariant()
  $asset = $parts[1].Trim().TrimStart("*")

  Write-Host "Downloading $asset..."
  $zip = Join-Path $work $asset
  Invoke-WebRequest -UseBasicParsing -Uri "$releaseUrl/$asset" -OutFile $zip

  $actual = (Get-FileHash -Algorithm SHA256 -Path $zip).Hash.ToLowerInvariant()
  if ($actual -ne $expected) { throw "checksum mismatch for $asset (expected $expected, got $actual)" }
  Write-Host "Checksum OK."

  # Stop a running copy so its files can be replaced.
  Get-Process -Name tsumiba -ErrorAction SilentlyContinue | ForEach-Object {
    & $_.Path quit 2>$null
    Start-Sleep -Milliseconds 800
    if (-not $_.HasExited) { $_ | Stop-Process -Force }
  }

  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  Expand-Archive -Path $zip -DestinationPath $InstallDir -Force

  # PATH (user scope), so `tsumiba note ...` works in a new terminal.
  $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if (($userPath -split ";") -notcontains $InstallDir) {
    [Environment]::SetEnvironmentVariable("Path", (($userPath.TrimEnd(";") + ";" + $InstallDir).TrimStart(";")), "User")
    Write-Host "Added $InstallDir to your PATH (open a new terminal to use it)."
  }

  # Start menu shortcut.
  $startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut((Join-Path $startMenu "tsumiba.lnk"))
  $shortcut.TargetPath = Join-Path $InstallDir "tsumiba.exe"
  $shortcut.WorkingDirectory = $InstallDir
  $shortcut.Description = "Notes and files, one gesture away"
  $shortcut.Save()

  Write-Host "Installed to $InstallDir."
  Write-Host "Start it from the Start menu, or run: tsumiba"
  Write-Host "Windows may show a SmartScreen prompt the first time: the program is not code-signed yet."
}
finally {
  Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
