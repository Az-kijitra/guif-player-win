# Runs inside Windows Sandbox (see run-sandbox-test.mjs). Installs, runs, updates and uninstalls for real;
# the sandbox is discarded afterwards. ASCII only: Windows PowerShell reads this file as the system code page.
param([string]$Work)

$ErrorActionPreference = "Continue"
$results = Join-Path $Work "results"
$log = Join-Path $results "log.txt"
$new = Get-ChildItem (Join-Path $Work "installers") -Filter "guif-player_*_x64-setup.exe" | Where-Object { $_.Name -notlike "*0.1.1*" } | Select-Object -First 1
$old = Join-Path $Work "installers\guif-player_0.1.1_x64-setup.exe"
$legacyDb = Join-Path $Work "legacy.db"
$newVersion = $new.Name -replace "guif-player_(.*)_x64-setup.exe", '$1'

$installDir = Join-Path $env:LOCALAPPDATA "guif-player"
$exe = Join-Path $installDir "guif-player.exe"
$uninstaller = Join-Path $installDir "uninstall.exe"
$dataDir = Join-Path $env:LOCALAPPDATA "dev.guif.player"
$db = Join-Path $dataDir "guif_player.db"
$shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\guif-player.lnk"
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\guif-player"

$script:passed = 0
$script:failed = 0

function Write-Log($text) { Add-Content -Path $log -Value $text -Encoding UTF8 }
function Check($name, $ok, $detail = "") {
    if ($ok) { $script:passed++; $mark = "PASS" } else { $script:failed++; $mark = "FAIL" }
    Write-Log "$mark  $name  $detail"
}

function Wait-Until($seconds, [scriptblock]$condition) {
    $end = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $end) {
        if (& $condition) { return $true }
        Start-Sleep -Milliseconds 500
    }
    return $false
}

function Install($path) {
    $p = Start-Process -FilePath $path -ArgumentList "/S" -PassThru -Wait
    Write-Log "  installed $(Split-Path $path -Leaf), exit $($p.ExitCode)"
    Wait-Until 20 { Test-Path $exe } | Out-Null
}

function Uninstall-Silently {
    Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait | Out-Null
    # The uninstaller hands over to a copy of itself in the temp folder
    Wait-Until 30 { -not (Test-Path $exe) } | Out-Null
    Start-Sleep -Seconds 2
}

function Stop-App {
    Get-Process guif-player -ErrorAction SilentlyContinue | Stop-Process -Force
    Wait-Until 10 { -not (Get-Process guif-player -ErrorAction SilentlyContinue) } | Out-Null
    Start-Sleep -Seconds 1
}

# SQLite keeps PRAGMA user_version at byte 60 of the file, big endian
function Get-UserVersion($path) {
    $bytes = [System.IO.File]::ReadAllBytes($path)
    return ([int]$bytes[60] -shl 24) -bor ([int]$bytes[61] -shl 16) -bor ([int]$bytes[62] -shl 8) -bor [int]$bytes[63]
}

function Contains-Text($path, $text) {
    $content = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($path))
    return $content.Contains($text)
}

function Save-Screenshot($name) {
    Add-Type -AssemblyName System.Windows.Forms, System.Drawing
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bitmap.Save((Join-Path $results $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose(); $bitmap.Dispose()
}

Write-Log "testing $($new.Name)"
Write-Log "webview2: $((Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv)"

# ---------------------------------------------------------------- A. a fresh install
Write-Log "--- fresh install"
Install $new.FullName
Check "program installed" (Test-Path $exe)
Check "uninstaller installed" (Test-Path $uninstaller)
Check "license shipped" (Test-Path (Join-Path $installDir "LICENSE.txt"))
Check "third-party licenses shipped" (Test-Path (Join-Path $installDir "THIRD_PARTY_LICENSES.txt"))
Check "start menu shortcut" (Test-Path $shortcut)
$entry = Get-ItemProperty $uninstallKey -ErrorAction SilentlyContinue
Check "listed in installed apps with the version" ($entry.DisplayVersion -eq $newVersion) "version=$($entry.DisplayVersion)"
Check "no database before the first start" (-not (Test-Path $db))

Start-Process -FilePath $exe
$windowed = Wait-Until 25 { (Get-Process guif-player -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -eq "guif-player" }) }
Check "the app opens a window" $windowed
Start-Sleep -Seconds 6
Save-Screenshot "fresh-install.png"
Check "it is still running after a few seconds" ([bool](Get-Process guif-player -ErrorAction SilentlyContinue))
Check "database in the new folder" (Test-Path $db)
Check "no database in the install folder" (-not (Test-Path (Join-Path $installDir "guif_player.db")))
Stop-App
if (Test-Path $db) { Check "database has a schema version" ((Get-UserVersion $db) -ge 1) "user_version=$(Get-UserVersion $db)" }

# ---------------------------------------------------------------- B. uninstalling keeps the data by default
Write-Log "--- uninstall, keeping data"
Uninstall-Silently
Check "program removed" (-not (Test-Path $exe))
Check "uninstaller removed" (-not (Test-Path $uninstaller))
Check "install folder removed" (-not (Test-Path $installDir))
Check "shortcut removed" (-not (Test-Path $shortcut))
Check "no longer listed in installed apps" (-not (Test-Path $uninstallKey))
Check "the data stays" (Test-Path $db)

# ---------------------------------------------------------------- C. the "delete the application data" check box
Write-Log "--- uninstall, deleting data"
Install $new.FullName
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32Button {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr SendMessageW(IntPtr hWnd, uint msg, IntPtr w, IntPtr l);
    public static bool IsChecked(IntPtr h) { return SendMessageW(h, 0xF0, IntPtr.Zero, IntPtr.Zero).ToInt64() == 1; }
    public static void Click(IntPtr h) { SendMessageW(h, 0xF5, IntPtr.Zero, IntPtr.Zero); }
}
"@
$tree = [System.Windows.Automation.TreeScope]
$root = [System.Windows.Automation.AutomationElement]::RootElement
$all = [System.Windows.Automation.Condition]::TrueCondition
function Find-Dialog {
    foreach ($window in $root.FindAll($tree::Children, $all)) {
        if ($window.Current.Name -like "*guif-player*" -and $window.Current.ClassName -eq "#32770") { return $window }
    }
    return $null
}
# NSIS's own buttons have control ids; the check box Tauri adds has none, and shows up only as a plain pane here
function Find-Button($dialog, $id) {
    foreach ($e in $dialog.FindAll($tree::Descendants, $all)) {
        if ($e.Current.ClassName -eq "Button" -and $e.Current.AutomationId -eq $id) { return $e }
    }
    return $null
}
function Handle-Of($element) { return [IntPtr]$element.Current.NativeWindowHandle }

Start-Process -FilePath $uninstaller | Out-Null
$dialog = $null
Wait-Until 20 { $script:dialog = Find-Dialog; [bool]$script:dialog } | Out-Null
$box = if ($dialog) { Find-Button $dialog "" } else { $null }
if ($box) {
    Write-Log "  uninstaller window: $($dialog.Current.Name), check box: $($box.Current.Name)"
    Check "the check box is off by default" (-not [Win32Button]::IsChecked((Handle-Of $box)))
    [Win32Button]::Click((Handle-Of $box))
    Check "the check box can be turned on" ([Win32Button]::IsChecked((Handle-Of $box)))
    Save-Screenshot "uninstall-dialog.png"
    [Win32Button]::Click((Handle-Of (Find-Button $dialog "1")))
    Wait-Until 30 { -not (Test-Path $exe) } | Out-Null
    Start-Sleep -Seconds 2
    # The last page: close it
    $last = Find-Dialog
    if ($last) {
        $close = Find-Button $last "1"
        if ($close) { [Win32Button]::Click((Handle-Of $close)) }
    }
    Start-Sleep -Seconds 1
    Check "program removed" (-not (Test-Path $exe))
    Check "the data is deleted when asked" (-not (Test-Path $db)) "dataDir exists: $(Test-Path $dataDir)"
} else {
    Check "uninstaller check box found" $false "error: $($Error[0])"
    Save-Screenshot "uninstall-not-found.png"
    if (Test-Path $uninstaller) { Uninstall-Silently }
}
Remove-Item $dataDir -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue

# ---------------------------------------------------------------- D. updating from 0.1.1
Write-Log "--- update from 0.1.1"
if (Test-Path $old) {
    Install $old
    Check "0.1.1 installed" (Test-Path $exe)
    Copy-Item $legacyDb (Join-Path $installDir "guif_player.db")
    Check "the old database sits in the install folder" (Test-Path (Join-Path $installDir "guif_player.db"))

    Install $new.FullName
    $entry = Get-ItemProperty $uninstallKey -ErrorAction SilentlyContinue
    Check "the version is updated" ($entry.DisplayVersion -eq $newVersion) "version=$($entry.DisplayVersion)"
    Check "the update leaves the old database alone" (Test-Path (Join-Path $installDir "guif_player.db"))

    Start-Process -FilePath $exe
    Wait-Until 25 { (Get-Process guif-player -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -eq "guif-player" }) } | Out-Null
    Start-Sleep -Seconds 6
    Save-Screenshot "after-update.png"
    Check "the database moved to the new folder" (Test-Path $db)
    Check "the old location is empty" (-not (Test-Path (Join-Path $installDir "guif_player.db")))
    Check "the program stayed in the install folder" (Test-Path $exe)
    Stop-App
    if (Test-Path $db) {
        Check "the data survived" (Contains-Text $db "old favourites")
        Check "the schema version was added" ((Get-UserVersion $db) -ge 1) "user_version=$(Get-UserVersion $db)"
    }
    Uninstall-Silently
    Check "uninstalling the updated app removes the install folder" (-not (Test-Path $installDir))
    Check "uninstalling the updated app keeps the data" (Test-Path $db)
} else {
    Write-Log "SKIP  the 0.1.1 installer is not available"
}

Write-Log "RESULT passed=$($script:passed) failed=$($script:failed)"
Set-Content -Path (Join-Path $results "done.txt") -Value "done"
