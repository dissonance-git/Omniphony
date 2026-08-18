param(
    [string]$AppRoot = ''
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($AppRoot)) { $AppRoot = Join-Path $env:ProgramFiles 'Omniphony' }

$baselineUninstaller = Join-Path $here 'Uninstall-OmniphonyAPO.ps1'
$stateRoot = Join-Path $env:ProgramData 'Omniphony'
$streamBackupPath = Join-Path $stateRoot 'stream-backup.json'
$streamApoClsid = '{07D403D9-8A98-43EF-8C28-8651756D83BE}'
$sfxValue = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5'
$sfxModesValue = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},5'

function Open-Hklm64([string]$Path, [bool]$Writable) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $key = $base.OpenSubKey($Path, $Writable)
    return [pscustomobject]@{ Base = $base; Key = $key }
}

function Set-ValueSnapshot([Microsoft.Win32.RegistryKey]$Key, [string]$Name, $Snapshot) {
    if (-not [bool]$Snapshot.Exists) {
        $Key.DeleteValue($Name, $false)
        return
    }
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
    $Key.SetValue($Name, $value, $kind)
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

$streamBackup = $null
if (Test-Path -LiteralPath $streamBackupPath) {
    try { $streamBackup = Get-Content -LiteralPath $streamBackupPath -Raw | ConvertFrom-Json }
    catch { Write-Warning "Could not read native-surround backup: $($_.Exception.Message)" }
}

Set-AudioServiceRunning $false
try {
    if ($streamBackup -and $streamBackup.FxPath) {
        $opened = Open-Hklm64 ([string]$streamBackup.FxPath) $true
        try {
            if ($opened.Key) {
                Set-ValueSnapshot $opened.Key $sfxValue $streamBackup.Sfx
                Set-ValueSnapshot $opened.Key $sfxModesValue $streamBackup.SfxModes
            }
        } finally {
            if ($opened.Key) { $opened.Key.Dispose() }
            $opened.Base.Dispose()
        }
    }

    # Remove the additive stream APO global registration while AudioDG is down.
    # The baseline uninstaller removes the original endpoint APO immediately after.
    Remove-HklmTree "SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$streamApoClsid"
    Remove-HklmTree "SOFTWARE\Classes\CLSID\$streamApoClsid"
} finally {
    Set-AudioServiceRunning $true
}

& $baselineUninstaller -AppRoot $AppRoot

if (Test-Path -LiteralPath $streamBackupPath) {
    Remove-Item -LiteralPath $streamBackupPath -Force -ErrorAction SilentlyContinue
}
Write-Host 'Omniphony Windows audio integration removed; prior endpoint effect state was restored.'
