$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$programData = if ([string]::IsNullOrWhiteSpace($env:ProgramData)) { 'C:\ProgramData' } else { $env:ProgramData }
$stateRoot = Join-Path $programData 'Omniphony'
$eqPresetPath = Join-Path $stateRoot 'eq-preset.txt'
$legacyEqPath = Join-Path $stateRoot 'personal-eq.txt'
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
    return 'on'
}

function Set-EqPreset([string]$Preset) {
    [IO.File]::WriteAllText($eqPresetPath, "$Preset`r`n", [Text.Encoding]::ASCII)
}

function Show-TrayMessage([string]$Text) {
    $notify.BalloonTipTitle = 'Omniphony'
    $notify.BalloonTipText = $Text
    $notify.ShowBalloonTip(2500)
}

function Restart-WindowsAudioService {
    try {
        $powershell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
        $restartCommand = @'
$ErrorActionPreference = 'Stop'
$timeout = [TimeSpan]::FromSeconds(15)

$audio = Get-Service -Name 'Audiosrv' -ErrorAction Stop
if ($audio.Status -ne 'Stopped') {
    Stop-Service -Name 'Audiosrv' -Force -ErrorAction Stop
    (Get-Service -Name 'Audiosrv' -ErrorAction Stop).WaitForStatus('Stopped', $timeout)
}

$builder = Get-Service -Name 'AudioEndpointBuilder' -ErrorAction Stop
if ($builder.Status -ne 'Running') {
    Start-Service -Name 'AudioEndpointBuilder' -ErrorAction Stop
    (Get-Service -Name 'AudioEndpointBuilder' -ErrorAction Stop).WaitForStatus('Running', $timeout)
}

Start-Service -Name 'Audiosrv' -ErrorAction Stop
(Get-Service -Name 'Audiosrv' -ErrorAction Stop).WaitForStatus('Running', $timeout)
'@
        $encodedCommand = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($restartCommand))
        $process = Start-Process -FilePath $powershell `
            -Verb RunAs `
            -ArgumentList @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-EncodedCommand', $encodedCommand) `
            -Wait `
            -PassThru
        if ($process.ExitCode -ne 0) {
            throw "Windows Audio restart exited with code $($process.ExitCode)."
        }
        Show-TrayMessage 'Windows Audio service restarted.'
    } catch {
        Show-TrayMessage "Could not restart Windows Audio: $($_.Exception.Message)"
    }
}

$notify = New-Object System.Windows.Forms.NotifyIcon
$notify.Icon = [System.Drawing.SystemIcons]::Application
$notify.Visible = $true

$menu = New-Object System.Windows.Forms.ContextMenuStrip
$statusItem = New-Object System.Windows.Forms.ToolStripMenuItem
$statusItem.Text = 'Omniphony'
$statusItem.Enabled = $false
[void]$menu.Items.Add($statusItem)

$eqItem = New-Object System.Windows.Forms.ToolStripMenuItem
[void]$menu.Items.Add($eqItem)

[void]$menu.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator))

$restartAudioItem = New-Object System.Windows.Forms.ToolStripMenuItem
$restartAudioItem.Text = 'Restart Windows Audio Service'
[void]$menu.Items.Add($restartAudioItem)

[void]$menu.Items.Add((New-Object System.Windows.Forms.ToolStripSeparator))

$exitItem = New-Object System.Windows.Forms.ToolStripMenuItem
$exitItem.Text = 'Exit tray'
[void]$menu.Items.Add($exitItem)
$notify.ContextMenuStrip = $menu

function Update-TrayState {
    $preset = Get-EqPreset
    $enabled = $preset -eq 'on'
    $eqItem.Checked = $enabled
    $eqItem.Text = if ($enabled) { 'EQ: On' } else { 'EQ: Off' }
    $notify.Text = if ($enabled) { 'Omniphony | EQ: On' } else { 'Omniphony | EQ: Off' }
}

function Toggle-Eq {
    try {
        $next = if ((Get-EqPreset) -eq 'on') { 'off' } else { 'on' }
        Set-EqPreset $next
        Update-TrayState
    } catch {
        Show-TrayMessage "Could not change the EQ setting: $($_.Exception.Message)"
    }
}

$eqItem.Add_Click({ Toggle-Eq })
$restartAudioItem.Add_Click({ Restart-WindowsAudioService })

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
