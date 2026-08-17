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
$efxValueName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
$efxModesValueName = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},7'
$disableSysFxValueName = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'

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

Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
$backup = $null
if (Test-Path -LiteralPath $backupPath) {
    try { $backup = Get-Content -LiteralPath $backupPath -Raw | ConvertFrom-Json }
    catch { Write-Warning "Could not read endpoint backup: $($_.Exception.Message)" }
}

Set-AudioServiceRunning $false
try {
    if ($backup) {
        if ([int]$backup.Version -ge 3 -and $backup.FxPath -and $backup.FxSnapshots) {
            $fxPath = [string]$backup.FxPath
            foreach ($property in $backup.FxSnapshots.PSObject.Properties) {
                Set-ValueSnapshot $fxPath $property.Name $property.Value
            }
            if ($backup.AudioProtection) {
                Set-ValueSnapshot 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG' $backup.AudioProtection
            }
        } elseif ($backup.EndpointGuid) {
            $fxPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$($backup.EndpointGuid)\FxProperties"
            if ($backup.Efx) { Set-ValueSnapshot $fxPath $efxValueName $backup.Efx }
            if ($backup.EfxModes) { Set-ValueSnapshot $fxPath $efxModesValueName $backup.EfxModes }
            if ($backup.DisableSysFx) { Set-ValueSnapshot $fxPath $disableSysFxValueName $backup.DisableSysFx }
        }
    }

    Remove-HklmTree "SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$apoClsid"
    Remove-HklmTree "SOFTWARE\Classes\CLSID\$apoClsid"

    if (Test-Path -LiteralPath $runtimeRoot) {
        Remove-Item -LiteralPath $runtimeRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
} finally {
    Set-AudioServiceRunning $true
}
Start-Sleep -Milliseconds 750

if (Test-Path -LiteralPath $backupPath) { Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue }
Write-Host 'Omniphony APO removed and the previous endpoint/audio state was restored.'
