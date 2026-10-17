<#
.SYNOPSIS
  Start the real tsumiba on Windows and check that the window loop, the tray and
  the local API come up and answer. Informational in CI: a runner has no GPU,
  so this also tells us whether the window can be created without one.
#>
param([string]$Exe = "target\release\tsumiba.exe")

$ErrorActionPreference = "Stop"
$Exe = (Resolve-Path $Exe).Path
$home_ = Join-Path ([System.IO.Path]::GetTempPath()) ("tsumiba-smoke-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $home_ | Out-Null
$env:TSUMIBA_HOME = $home_
$env:TSUMIBA_LOG = "debug"

function Invoke-Tsumiba([string[]]$Arguments) {
  # The release build is a GUI-subsystem program, so capture its output through files.
  $out = Join-Path $home_ "cli.out"; $err = Join-Path $home_ "cli.err"
  $p = Start-Process -FilePath $Exe -ArgumentList $Arguments -Wait -PassThru -NoNewWindow `
    -RedirectStandardOutput $out -RedirectStandardError $err
  [pscustomobject]@{ Code = $p.ExitCode; Out = (Get-Content $out -Raw); Err = (Get-Content $err -Raw) }
}

function Show-Log {
  $log = Join-Path $home_ "data\tsumiba.log"
  if (Test-Path $log) { Write-Host "--- tsumiba.log"; Get-Content $log -Tail 40 }
}

$failures = 0
function Check([string]$What, [bool]$Ok) {
  if ($Ok) { Write-Host "ok    $What" } else { Write-Host "FAIL  $What"; $script:failures++ }
}

$app = Start-Process -FilePath $Exe -ArgumentList "--background" -PassThru
Start-Sleep -Seconds 6
Check "the app is still running after start-up" (-not $app.HasExited)
if ($app.HasExited) { Write-Host "exit code: $($app.ExitCode)"; Show-Log; exit 1 }

$note = Invoke-Tsumiba @("note", "written while the app runs")
Check "a note goes through the running app" ($note.Code -eq 0 -and $note.Out.Trim().Length -eq 20)
$list = Invoke-Tsumiba @("list")
Check "the note is listed" ($list.Out -match "written while the app runs")

$show = Invoke-Tsumiba @("show")
Check "show is accepted" ($show.Code -eq 0)
Start-Sleep -Seconds 2
$app.Refresh()
Check "the app survived showing its window" (-not $app.HasExited)
Check "the window exists" ($app.MainWindowHandle -ne 0)
Write-Host "      window title: '$($app.MainWindowTitle)'"

$hide = Invoke-Tsumiba @("hide")
Check "hide is accepted" ($hide.Code -eq 0)

$quit = Invoke-Tsumiba @("quit")
Check "quit is accepted" ($quit.Code -eq 0)
$exited = $app.WaitForExit(10000)
Check "the process exits" $exited

Show-Log
Remove-Item -Recurse -Force $home_ -ErrorAction SilentlyContinue
if ($failures -gt 0) { Write-Host "$failures check(s) failed"; exit 1 }
Write-Host "all Windows GUI smoke checks passed"
