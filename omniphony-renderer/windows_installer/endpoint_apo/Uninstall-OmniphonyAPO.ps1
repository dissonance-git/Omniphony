param(
    [string]$PhysicalOutput = '',
    [string]$AppRoot = ''
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($AppRoot)) { $AppRoot = Join-Path $env:ProgramFiles 'Omniphony' }

$runtimeRoot = Join-Path $AppRoot 'APO'
$stateRoot = Join-Path $env:ProgramData 'Omniphony'
$backupPath = Join-Path $stateRoot 'endpoint-backup.json'
$apoClsid = '{A9333BFE-39C1-40FD-B4B0-ECC591410B47}'
$ctl = Join-Path $here 'OmniphonyApoCtl.exe'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the Omniphony uninstaller from an elevated context.'
}

function Open-Hklm64([string]$Path, [bool]$Writable, [bool]$Create) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $key = $null
    if ($Create) { $key = $base.CreateSubKey($Path, $Writable) }
    else { $key = $base.OpenSubKey($Path, $Writable) }
    if (-not $key) { $base.Dispose(); throw "Could not open HKLM\$Path" }
    return [pscustomobject]@{ Base = $base; Key = $key }
}

function Set-ValueSnapshot([string]$Path, [string]$Name, $Snapshot) {
    $opened = Open-Hklm64 $Path $true $true
    try {
        if (-not [bool]$Snapshot.Exists) { $opened.Key.DeleteValue($Name, $false); return }
        $kindName = [string]$Snapshot.Kind
        $kind = [Microsoft.Win32.RegistryValueKind][Enum]::Parse([Microsoft.Win32.RegistryValueKind], $kindName)
        $value = $Snapshot.Value
        switch ($kindName) {
            'Binary'       { $value = [Convert]::FromBase64String([string]$value) }
            'MultiString'  { $value = [string[]]@($value) }
            'DWord'        { $value = [int]$value }
            'QWord'        { $value = [long]$value }
            'String'       { $value = [string]$value }
            'ExpandString' { $value = [string]$value }
        }
        $opened.Key.SetValue($Name, $value, $kind)
    } finally { $opened.Key.Dispose(); $opened.Base.Dispose() }
}

function Remove-HklmTree([string]$Path) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    try { $base.DeleteSubKeyTree($Path, $false) } finally { $base.Dispose() }
}

function Set-AudioServiceRunning([bool]$Running) {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($Running -and $service.Status -ne 'Running') { Start-Service -Name AudioSrv }
    if ((-not $Running) -and $service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
}

function Restart-AudioGraph {
    Set-AudioServiceRunning $false
    Start-Sleep -Milliseconds 250
    Set-AudioServiceRunning $true
    Start-Sleep -Milliseconds 750
}

Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
$backup = $null
if (Test-Path -LiteralPath $backupPath) {
    try { $backup = Get-Content -LiteralPath $backupPath -Raw | ConvertFrom-Json }
    catch { Write-Warning "Could not read endpoint backup: $($_.Exception.Message)" }
}

Set-AudioServiceRunning $true
if ($backup -and [int]$backup.Version -ge 4 -and $backup.EndpointId -and (Test-Path -LiteralPath $ctl)) {
    try {
        & $ctl bypass-id ([string]$backup.EndpointId)
        if ($LASTEXITCODE -ne 0) { Write-Warning "Endpoint bypass returned $LASTEXITCODE" }
    } catch { Write-Warning "Could not bypass Omniphony before removal: $($_.Exception.Message)" }

    try {
        & $ctl detach-id ([string]$backup.EndpointId)
        if ($LASTEXITCODE -ne 0) { Write-Warning "Endpoint detach returned $LASTEXITCODE" }
    } catch { Write-Warning "Could not detach Omniphony before removal: $($_.Exception.Message)" }
}

Set-AudioServiceRunning $false
try {
    Remove-HklmTree "SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$apoClsid"
    Remove-HklmTree "SOFTWARE\Classes\CLSID\$apoClsid"

    if ($backup -and $backup.AudioProtection) {
        Set-ValueSnapshot 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG' $backup.AudioProtection
    }

    if (Test-Path -LiteralPath $runtimeRoot) {
        Remove-Item -LiteralPath $runtimeRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
} finally {
    Set-AudioServiceRunning $true
}

if ($backup -and [int]$backup.Version -ge 4 -and $backup.EndpointId -and (Test-Path -LiteralPath $ctl)) {
    if ([int]$backup.PriorEnhancementsDisabled -eq 0) {
        try {
            & $ctl enable-effects-id ([string]$backup.EndpointId)
            if ($LASTEXITCODE -ne 0) { Write-Warning "Restoring the prior enhancements setting returned $LASTEXITCODE" }
        } catch { Write-Warning "Could not restore the prior enhancements setting: $($_.Exception.Message)" }
    }
}

try { Restart-AudioGraph } catch { Write-Warning "Final audio graph restart failed: $($_.Exception.Message)" }

if (Test-Path -LiteralPath $backupPath) { Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue }
Write-Host 'Omniphony APO removed. Endpoint effects were detached through Windows audio policy and plain audio remains available.'
