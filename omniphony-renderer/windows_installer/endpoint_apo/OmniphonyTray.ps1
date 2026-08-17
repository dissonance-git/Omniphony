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

    switch ($text) {
        { $_ -in @('0', 'off', 'false', 'disabled', 'none') } { return 'off' }
        { $_ -in @('native', 'omniphony', 'omniphony-native', 'omniphony_tuned') } { return 'native' }
        default { return 'legacy' }
    }
}

function Set-EqPreset([string]$Preset) {
    [IO.File]::WriteAllText($eqPresetPath, "$Preset`r`n", [Text.Encoding]::ASCII)
}

function Get-RightCompEnabled {
    if (-not (Test-Path -LiteralPath $rightCompPath)) { return $true }
    try {
        $value = ([IO.File]::ReadAllText($rightCompPath)).Trim().ToLowerInvariant()
        return $value -notin @('0', 'off', 'false', 'disabled')
    } catch {
        return $true
    }
}

function Set-RightCompEnabled([bool]$Enabled) {
    $text = if ($Enabled) { "1`r`n" } else { "0`r`n" }
    [IO.File]::WriteAllText($rightCompPath, $text, [Text.Encoding]::ASCII)
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

$legacyItem = New-Object System.Windows.Forms.ToolStripMenuItem
$legacyItem.Text = 'EQ: Legacy DTS-era'
[void]$menu.Items.Add($legacyItem)

$nativeItem = New-Object System.Windows.Forms.ToolStripMenuItem
$nativeItem.Text = 'EQ: Omniphony tuned'
[void]$menu.Items.Add($nativeItem)

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
    $rightComp = Get-RightCompEnabled

    $offItem.Checked = $preset -eq 'off'
    $legacyItem.Checked = $preset -eq 'legacy'
    $nativeItem.Checked = $preset -eq 'native'
    $rightCompItem.Checked = $rightComp

    $presetLabel = switch ($preset) {
        'off' { 'Off' }
        'native' { 'Native' }
        default { 'Legacy' }
    }
    $rightLabel = if ($rightComp) { 'On' } else { 'Off' }
    $notify.Text = "Omniphony | EQ: $presetLabel | Right comp: $rightLabel"
}

function Select-EqPreset([string]$Preset) {
    try {
        Set-EqPreset $Preset
        Update-TrayState
    } catch {
        $notify.BalloonTipTitle = 'Omniphony'
        $notify.BalloonTipText = "Could not change the EQ preset: $($_.Exception.Message)"
        $notify.ShowBalloonTip(2500)
    }
}

$offItem.Add_Click({ Select-EqPreset 'off' })
$legacyItem.Add_Click({ Select-EqPreset 'legacy' })
$nativeItem.Add_Click({ Select-EqPreset 'native' })

$rightCompItem.Add_Click({
    try {
        Set-RightCompEnabled (-not (Get-RightCompEnabled))
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
