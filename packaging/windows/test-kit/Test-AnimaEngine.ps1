<#
.SYNOPSIS
    animaEngine on a real Windows machine: install, run, a few checks by
    hand, uninstall, and a report to send back.

.DESCRIPTION
    Installs the per-user installer silently, checks what it wrote, runs
    the app with its log captured, measures it at rest, asks about what
    only a person can see, checks Start at login and the diagnostic
    report, then uninstalls and checks nothing is left. Everything it
    finds goes to a folder on the Desktop: report.txt, the app's log and
    the diagnostic report.

    Needs no administrator rights. Windows PowerShell 5.1 (built in) or
    PowerShell 7.

.PARAMETER Setup
    The installer. Default: the newest animaEngine-*-setup.exe beside
    this script, else in Downloads.

.PARAMETER KeepInstalled
    Leave the app installed at the end.

.PARAMETER RestSeconds
    How long to measure CPU and memory with the app running (default 30).

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\Test-AnimaEngine.ps1
#>
param(
    [string]$Setup,
    [switch]$KeepInstalled,
    [int]$RestSeconds = 30
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

$App = 'animaEngine'
$Exe = 'anima-engine.exe'
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\$App"
$UninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$App"
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$Shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$App.lnk"
$DataDir = Join-Path $env:APPDATA $App

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$OutDir = Join-Path ([Environment]::GetFolderPath('Desktop')) "anima-test-$stamp"
New-Item -ItemType Directory -Path $OutDir | Out-Null
$Report = New-Object System.Collections.Generic.List[string]

function Note([string]$Text) {
    $Report.Add($Text)
    Write-Host $Text
}

function Check([string]$Label, [bool]$Ok, [string]$Detail = '') {
    $mark = if ($Ok) { 'ok ' } else { 'BAD' }
    $line = "$mark $Label"
    if ($Detail) { $line += " -- $Detail" }
    Note $line
}

function Ask([string]$Question) {
    do {
        $answer = (Read-Host "$Question  [y = yes, n = no, s = skip]").Trim().ToLower()
    } until ($answer -in @('y', 'n', 's'))
    $mark = @{ y = 'ok '; n = 'BAD'; s = '-- ' }[$answer]
    $line = "$mark (by hand) $Question"
    if ($answer -eq 'n') {
        $seen = Read-Host '    What did you see instead?'
        if ($seen) { $line += " -- $seen" }
    }
    Note $line
    return ($answer -eq 'y')
}

function Wait-Key([string]$Text) {
    Read-Host "$Text  (press Enter when done)" | Out-Null
}

function Get-AppProcess {
    Get-Process -Name 'anima-engine' -ErrorAction SilentlyContinue
}

Note "animaEngine test on Windows -- $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
Note ''

# -- The machine -------------------------------------------------------
Note '== The machine'
$os = Get-CimInstance Win32_OperatingSystem
Note "Windows: $($os.Caption) $($os.Version) (build $($os.BuildNumber)), $($os.OSArchitecture)"
foreach ($gpu in Get-CimInstance Win32_VideoController) {
    Note "GPU: $($gpu.Name), driver $($gpu.DriverVersion)"
}
Add-Type -AssemblyName System.Windows.Forms
foreach ($screen in [System.Windows.Forms.Screen]::AllScreens) {
    $b = $screen.Bounds
    $primary = if ($screen.Primary) { ' (primary)' } else { '' }
    Note "Screen: $($b.Width)x$($b.Height) at $($b.X),$($b.Y)$primary"
}
$dpi = (Get-ItemProperty 'HKCU:\Control Panel\Desktop\WindowMetrics' -Name AppliedDPI -ErrorAction SilentlyContinue)
if ($dpi) { Note "Scaling of the primary screen: $([math]::Round($dpi.AppliedDPI / 96 * 100))%" }
$langs = (Get-WinUserLanguageList | ForEach-Object { $_.LanguageTag }) -join ', '
Note "Languages / keyboards: $langs"
Note ''

# -- Install -----------------------------------------------------------
Note '== Install'
if (-not $Setup) {
    $candidates = @(Get-ChildItem -Path $PSScriptRoot, (Join-Path $env:USERPROFILE 'Downloads') `
            -Filter 'animaEngine-*-setup.exe' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending)
    if ($candidates.Count -eq 0) { throw 'No animaEngine-*-setup.exe here or in Downloads: pass -Setup <path>.' }
    $Setup = $candidates[0].FullName
}
Note "Installer: $Setup"
$sig = Get-AuthenticodeSignature -FilePath $Setup
Note "Installer signature: $($sig.Status)  (NotSigned means SmartScreen warns on download)"
if (Get-Item -Path $Setup -Stream Zone.Identifier -ErrorAction SilentlyContinue) {
    Note 'The installer came from the internet (it carries the Mark of the Web).'
}
if (Get-AppProcess) {
    Note 'animaEngine was running: closing it first.'
    Get-AppProcess | Stop-Process -Force
    Start-Sleep -Seconds 2
}
$started = Get-Date
Start-Process -FilePath $Setup -ArgumentList '/S' -Wait
Note "Installed in $([math]::Round(((Get-Date) - $started).TotalSeconds, 1)) s"
$exePath = Join-Path $InstallDir $Exe
Check 'the app is in %LOCALAPPDATA%\Programs\animaEngine' (Test-Path $exePath)
$uninst = Get-ItemProperty -Path $UninstallKey -ErrorAction SilentlyContinue
Check 'it is listed in Apps & features' ($null -ne $uninst) $(if ($uninst) { "version $($uninst.DisplayVersion)" } else { '' })
Check 'it is in the Start menu' (Test-Path $Shortcut)
if (Test-Path $exePath) {
    Note "App signature: $((Get-AuthenticodeSignature -FilePath $exePath).Status)"
}
Note ''

# -- Run ----------------------------------------------------------------
Note '== Run'
$log = Join-Path $OutDir 'app-log.txt'
$env:RUST_LOG = 'anima_engine=info,warn'
$proc = Start-Process -FilePath $exePath -WorkingDirectory $env:USERPROFILE `
    -RedirectStandardError $log -PassThru
Start-Sleep -Seconds 15
$alive = -not $proc.HasExited
Check 'it starts and keeps running' $alive
if ($alive) {
    $p = Get-Process -Id $proc.Id
    $cpu0 = $p.TotalProcessorTime
    Start-Sleep -Seconds $RestSeconds
    $p.Refresh()
    $cpuShare = ($p.TotalProcessorTime - $cpu0).TotalSeconds / $RestSeconds / [Environment]::ProcessorCount * 100
    Note ("At rest over {0} s: CPU {1:N1}% of the machine, memory {2:N0} MB" -f `
            $RestSeconds, $cpuShare, ($p.WorkingSet64 / 1MB))
}
if (Test-Path $log) {
    $text = Get-Content -Path $log -Raw -ErrorAction SilentlyContinue
    if ($text) {
        foreach ($pattern in @('GPU adapter', 'Hotkey strategy', 'System tray', 'Placing primary overlay')) {
            $line = ($text -split "`n" | Where-Object { $_ -match $pattern } | Select-Object -First 1)
            if ($line) { Note ("log: " + ($line -replace '\x1b\[[0-9;]*m', '').Trim()) }
        }
        $bad = @($text -split "`n" | Where-Object { $_ -match 'ERROR|panicked' })
        Check 'no errors in the log' ($bad.Count -eq 0) $(if ($bad.Count) { "$($bad.Count) lines, see app-log.txt" } else { '' })
    }
}
Note ''

# -- By hand ------------------------------------------------------------
Note '== By hand'
Write-Host ''
Write-Host 'A few things only a person can see. Answer each one.' -ForegroundColor Cyan
Ask 'Five characters on the desktop, with nothing around them (no black or white box)?' | Out-Null
Ask 'A click on an empty spot near a character reaches the window or desktop icon underneath?' | Out-Null
Ask 'The animaEngine icon is in the notification area (perhaps under the ^ arrow)?' | Out-Null
Ask 'Right-click on that icon shows a menu (Edit mode, Pause, Quit...)?' | Out-Null
Ask 'The gear button in the top-right corner opens edit mode with the Welcome window?' | Out-Null
Ask 'In edit mode, a character can be dragged; let go, it falls back down?' | Out-Null
Ask 'With Notepad in front, Win+Alt+A opens and closes edit mode?' | Out-Null
if ($dpi -and $dpi.AppliedDPI -gt 96) {
    Ask 'At your scaling, the characters and the settings panel look sharp, not blurry?' | Out-Null
}
if ([System.Windows.Forms.Screen]::AllScreens.Count -gt 1) {
    Ask 'In edit mode, a character can be dragged onto the second screen and stays there?' | Out-Null
}
Write-Host ''
Wait-Key 'In edit mode: Appearance tab, turn ON "Start at login".'
$run = Get-ItemProperty -Path $RunKey -Name $App -ErrorAction SilentlyContinue
Check 'Start at login writes its Run entry' ($null -ne $run) $(if ($run) { $run.$App } else { '' })
Wait-Key 'Now turn "Start at login" OFF again.'
$run = Get-ItemProperty -Path $RunKey -Name $App -ErrorAction SilentlyContinue
Check 'turned off, the Run entry is gone' ($null -eq $run)
Wait-Key 'Appearance tab, About: click "Copy diagnostic report".'
try {
    $diag = Get-Clipboard -Raw
} catch {
    $diag = $null
}
if ($diag -and $diag -match 'animaEngine') {
    $diag | Out-File -FilePath (Join-Path $OutDir 'diagnostic-report.txt') -Encoding utf8
    Check 'the diagnostic report reaches the clipboard' $true
} else {
    Check 'the diagnostic report reaches the clipboard' $false 'the clipboard had something else'
}
Wait-Key 'Quit animaEngine from its notification-area menu.'
Start-Sleep -Seconds 3
Check 'Quit closes it' ($null -eq (Get-AppProcess))
Note ''

# -- Uninstall ------------------------------------------------------------
if ($KeepInstalled) {
    Note '== Left installed, as asked.'
} else {
    Note '== Uninstall'
    if (Get-AppProcess) { Get-AppProcess | Stop-Process -Force; Start-Sleep -Seconds 2 }
    $uninstaller = Join-Path $InstallDir 'uninstall.exe'
    if (Test-Path $uninstaller) {
        Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait
        # The uninstaller hands over to a copy of itself and returns early.
        for ($i = 0; $i -lt 30 -and (Test-Path $InstallDir); $i++) { Start-Sleep -Seconds 1 }
    }
    Check 'the install folder is gone' (-not (Test-Path $InstallDir))
    Check 'it is gone from Apps & features' ($null -eq (Get-ItemProperty -Path $UninstallKey -ErrorAction SilentlyContinue))
    Check 'it is gone from the Start menu' (-not (Test-Path $Shortcut))
    Check 'no Run entry is left' ($null -eq (Get-ItemProperty -Path $RunKey -Name $App -ErrorAction SilentlyContinue))
    if (Test-Path $DataDir) {
        Note "Settings and pictures are kept in $DataDir (by design; delete it by hand to start fresh)."
    }
}

$Report | Out-File -FilePath (Join-Path $OutDir 'report.txt') -Encoding utf8
Write-Host ''
Write-Host "Done. Send back the folder: $OutDir" -ForegroundColor Green
