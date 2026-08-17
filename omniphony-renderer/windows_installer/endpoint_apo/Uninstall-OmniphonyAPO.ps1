param(
    [string]$PhysicalOutput = '',
    [string]$AppRoot = ''
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

if ([string]::IsNullOrWhiteSpace($AppRoot)) {
    $AppRoot = Join-Path $env:ProgramFiles 'Omniphony'
    $supportRoot = $here
} else {
    $supportRoot = Join-Path $AppRoot 'support'
}

$runtimeRoot = Join-Path $AppRoot 'APO'
$legacyRuntimeRoot = Join-Path $AppRoot 'EndpointAPO'
$installedApo = Join-Path $runtimeRoot 'OmniphonyAPO.dll'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"
$stateRoot = Join-Path $env:ProgramData 'Omniphony'
$backupPath = Join-Path $stateRoot 'endpoint-backup.json'
$logPath = Join-Path $stateRoot 'uninstall-last.log'

$efxValueName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
$efxModesValueName = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},7'
$disableSysFxValueName = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'

New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
$transcriptStarted = $false
try {
    Start-Transcript -Path $logPath -Force | Out-Null
    $transcriptStarted = $true
} catch { }

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the Omniphony uninstaller from an elevated context.'
}

function Set-AudioServiceRunning([bool]$Running) {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($Running) {
        if ($service.Status -ne 'Running') { Start-Service -Name AudioSrv }
    } else {
        if ($service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
    }
}

function Open-FxKey([string]$EndpointGuid, [bool]$Writable) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $path = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$EndpointGuid\FxProperties"
    $key = $base.OpenSubKey($path, $Writable)
    if (-not $key) {
        $base.Dispose()
        throw "Could not open physical endpoint FxProperties: HKLM\$path"
    }
    return [pscustomobject]@{ Base = $base; Key = $key; Path = $path }
}

function Set-RegistrySnapshot([string]$EndpointGuid, [string]$Name, $Snapshot) {
    $opened = Open-FxKey $EndpointGuid $true
    try {
        if (-not [bool]$Snapshot.Exists) {
            $opened.Key.DeleteValue($Name, $false)
            return
        }

        $kindName = [string]$Snapshot.Kind
        $kind = [Microsoft.Win32.RegistryValueKind][Enum]::Parse(
            [Microsoft.Win32.RegistryValueKind], $kindName)
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
    } finally {
        $opened.Key.Dispose()
        $opened.Base.Dispose()
    }
}

function Restore-Backup {
    if (-not (Test-Path -LiteralPath $backupPath)) {
        Write-Warning 'No endpoint backup was found; runtime will be removed without changing unrelated endpoint effects.'
        return
    }

    $backup = Get-Content -LiteralPath $backupPath -Raw | ConvertFrom-Json
    $guid = [string]$backup.EndpointGuid
    if ([string]::IsNullOrWhiteSpace($guid)) {
        throw 'Endpoint backup does not contain an endpoint GUID.'
    }

    Write-Host "RESTORE_ENDPOINT`t$($backup.EndpointName)`t$guid"
    Set-RegistrySnapshot $guid $efxValueName $backup.Efx
    Set-RegistrySnapshot $guid $efxModesValueName $backup.EfxModes
    Set-RegistrySnapshot $guid $disableSysFxValueName $backup.DisableSysFx
}

try {
    Get-Process -Name Omniphony -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue

    Set-AudioServiceRunning $false
    try {
        Restore-Backup

        if (Test-Path -LiteralPath $installedApo) {
            $unregister = Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $installedApo) -Wait -PassThru
            if ($unregister.ExitCode -ne 0) {
                throw "APO COM unregistration failed: $($unregister.ExitCode)"
            }
        }

        if (Test-Path -LiteralPath $runtimeRoot) {
            Remove-Item -LiteralPath $runtimeRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
        if (Test-Path -LiteralPath $legacyRuntimeRoot) {
            Remove-Item -LiteralPath $legacyRuntimeRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    } finally {
        Set-AudioServiceRunning $true
    }

    Start-Sleep -Milliseconds 1000
    Remove-Item -LiteralPath $backupPath -Force -ErrorAction SilentlyContinue

    Write-Host 'OMNIPHONY_APO_UNINSTALL_OK 1'
    Write-Host 'The endpoint effect/enhancement state from before Omniphony was restored.'
    Write-Host "Diagnostics: $logPath"
}
finally {
    if ($transcriptStarted) {
        try { Stop-Transcript | Out-Null } catch { }
    }
}
