$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$programData = if ([string]::IsNullOrWhiteSpace($env:ProgramData)) { 'C:\ProgramData' } else { $env:ProgramData }
$stateRoot = Join-Path $programData 'Omniphony'
$eqPresetPath = Join-Path $stateRoot 'eq-preset.txt'
$legacyEqPath = Join-Path $stateRoot 'personal-eq.txt'
$rightCompPath = Join-Path $stateRoot 'right-ear-comp.txt'
$stopPath = Join-Path $stateRoot 'tray.stop'

New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
Remove-Item -LiteralPath $stopPath -Force -ErrorAction SilentlyContinue

$createdNew = $false
$mutex = New-Object System.Threading.Mutex($true, 'Local\OmniphonyTray', [ref]$createdNew)
if (-not $createdNew) {
    $mutex.Dispose()
    exit 0
}

function Get-EqPreset {
    $text = $null
    try {
        if (Test-Path -LiteralPath $eqPresetPath) {
            $text = ([IO.File]::ReadAllText($eqPresetPath)).Trim().ToLowerInvariant()
        } elseif (Test-Path -LiteralPath $legacyEqPath) {
            $text = ([IO.File]::ReadAllText($legacyEqPath)).Trim().ToLowerInvariant()
        }
    } catch { }

    if ($text -in @('0', 'off', 'false', 'disabled', 'none')) { return 'off' }
    # Every historical enabled spelling migrates to the single supported tuned EQ.
    return 'on'
}

function Set-EqPreset([string]$Preset) {
    [IO.File]::WriteAllText($eqPresetPath, "$Preset`r`n", [Text.Encoding]::ASCII)
}

function Get-BoolSetting([string]$Path, [bool]$Default) {
    if (-not (Test-Path -LiteralPath $Path)) { return $Default }
    try {
        $value = ([IO.File]::ReadAllText($Path)).Trim().ToLowerInvariant()
        return $value -notin @('0', 'off', 'false', 'disabled')
    } catch {
        return $Default
    }
}

function Set-BoolSetting([string]$Path, [bool]$Enabled) {
    $text = if ($Enabled) { "1`r`n" } else { "0`r`n" }
    [IO.File]::WriteAllText($Path, $text, [Text.Encoding]::ASCII)
}

$notify = New-Object System.Windows.Forms.NotifyIcon
$notify.Icon = [System.Drawing.SystemIcons]::Application
$notify.Visible = $true

$menu = New-Object System.Windows.Forms.ContextMenuStrip
$statusItem = New-Object System.Windows.Forms.ToolStripMenuItem
$statusItem.Text = 'Omniphony Current renderer'
$statusItem.Enabled = $false
[void]$menu.Items.Add($statusItem)

$offItem = New-Object System.Windows.Forms.ToolStripMenuItem
$offItem.Text = 'EQ: Off (Current baseline)'
[void]$menu.Items.Add($offItem)

$onItem = New-Object System.Windows.Forms.ToolStripMenuItem
$onItem.Text = 'EQ: On (tuned)'
[void]$menu.Items.Add($onItem)

[void]$menu.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator))

$rightCompItem = New-Object System.Windows.Forms.ToolStripMenuItem
$rightCompItem.Text = 'Right-ear compensation'
[void]$menu.Items.Add($rightCompItem)

[void]$menu.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator))

$exitItem = New-Object System.Windows.Forms.ToolStripMenuItem
$exitItem.Text = 'Exit tray'
[void]$menu.Items.Add($exitItem)
$notify.ContextMenuStrip = $menu

function Update-TrayState {
    $preset = Get-EqPreset
    $rightComp = Get-BoolSetting $rightCompPath $true

    $offItem.Checked = $preset -eq 'off'
    $onItem.Checked = $preset -eq 'on'
    $rightCompItem.Checked = $rightComp

    $presetLabel = if ($preset -eq 'off') { 'Off' } else { 'On' }
    $rightLabel = if ($rightComp) { 'R+' } else { 'R0' }
    $notify.Text = "Omniphony Current | EQ: $presetLabel | $rightLabel"
}

function Select-EqPreset([string]$Preset) {
    try {
        Set-EqPreset $Preset
        Update-TrayState
    } catch {
        $notify.BalloonTipTitle = 'Omniphony'
        $notify.BalloonTipText = "Could not change the EQ setting: $($_.Exception.Message)"
        $notify.ShowBalloonTip(2500)
    }
}

$offItem.Add_Click({ Select-EqPreset 'off' })
$onItem.Add_Click({ Select-EqPreset 'on' })

$rightCompItem.Add_Click({
    try {
        Set-BoolSetting $rightCompPath (-not (Get-BoolSetting $rightCompPath $true))
        Update-TrayState
    } catch {
        $notify.BalloonTipTitle = 'Omniphony'
        $notify.BalloonTipText = "Could not change right-ear compensation: $($_.Exception.Message)"
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
