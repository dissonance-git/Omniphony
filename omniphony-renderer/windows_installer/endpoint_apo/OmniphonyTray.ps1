$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$programData = if ([string]::IsNullOrWhiteSpace($env:ProgramData)) { 'C:\ProgramData' } else { $env:ProgramData }
$stateRoot = Join-Path $programData 'Omniphony'
$eqPath = Join-Path $stateRoot 'personal-eq.txt'
$stopPath = Join-Path $stateRoot 'tray.stop'

New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
Remove-Item -LiteralPath $stopPath -Force -ErrorAction SilentlyContinue

$createdNew = $false
$mutex = New-Object System.Threading.Mutex($true, 'Local\OmniphonyTray', [ref]$createdNew)
if (-not $createdNew) {
    $mutex.Dispose()
    exit 0
}

function Get-PersonalEqEnabled {
    if (-not (Test-Path -LiteralPath $eqPath)) { return $true }
    try {
        $value = ([IO.File]::ReadAllText($eqPath)).Trim().ToLowerInvariant()
        return $value -notin @('0', 'off', 'false', 'disabled')
    } catch {
        return $true
    }
}

function Set-PersonalEqEnabled([bool]$Enabled) {
    $text = if ($Enabled) { "1`r`n" } else { "0`r`n" }
    [IO.File]::WriteAllText($eqPath, $text, [Text.Encoding]::ASCII)
}

$notify = New-Object System.Windows.Forms.NotifyIcon
$notify.Icon = [System.Drawing.SystemIcons]::Application
$notify.Visible = $true

$menu = New-Object System.Windows.Forms.ContextMenuStrip
$statusItem = New-Object System.Windows.Forms.ToolStripMenuItem
$statusItem.Text = 'Omniphony Current renderer'
$statusItem.Enabled = $false
[void]$menu.Items.Add($statusItem)

$eqItem = New-Object System.Windows.Forms.ToolStripMenuItem
$eqItem.Text = 'Noire X personal EQ'
[void]$menu.Items.Add($eqItem)

[void]$menu.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator))

$exitItem = New-Object System.Windows.Forms.ToolStripMenuItem
$exitItem.Text = 'Exit tray'
[void]$menu.Items.Add($exitItem)
$notify.ContextMenuStrip = $menu

function Update-TrayState {
    $enabled = Get-PersonalEqEnabled
    $eqItem.Checked = $enabled
    $state = if ($enabled) { 'On' } else { 'Off' }
    $notify.Text = "Omniphony Current | Noire X EQ: $state"
}

$eqItem.Add_Click({
    try {
        $enabled = -not (Get-PersonalEqEnabled)
        Set-PersonalEqEnabled $enabled
        Update-TrayState
    } catch {
        $notify.BalloonTipTitle = 'Omniphony'
        $notify.BalloonTipText = "Could not change the personal EQ setting: $($_.Exception.Message)"
        $notify.ShowBalloonTip(2500)
    }
})

$exitItem.Add_Click({
    $notify.Visible = $false
    [System.Windows.Forms.Application]::Exit()
})

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 500
$timer.Add_Tick({
    if ((Test-Path -LiteralPath $stopPath) -or -not (Test-Path -LiteralPath $PSCommandPath)) {
        $notify.Visible = $false
        [System.Windows.Forms.Application]::Exit()
        return
    }
    Update-TrayState
})

try {
    Update-TrayState
    $timer.Start()
    [System.Windows.Forms.Application]::Run()
} finally {
    $timer.Stop()
    $timer.Dispose()
    $notify.Visible = $false
    $notify.Dispose()
    $menu.Dispose()
    try { $mutex.ReleaseMutex() } catch { }
    $mutex.Dispose()
}
