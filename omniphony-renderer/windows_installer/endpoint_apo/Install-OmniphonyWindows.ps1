param(
    [string]$PackageRoot = '',
    [string]$AppRoot = '',
    [switch]$AllowUnprotectedAudioDG
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
if ([string]::IsNullOrWhiteSpace($PackageRoot)) { $PackageRoot = $here }
if ([string]::IsNullOrWhiteSpace($AppRoot)) { $AppRoot = Join-Path $env:ProgramFiles 'Omniphony' }

$baselineInstaller = Join-Path $here 'Install-OmniphonyAPO.ps1'
$runtimeRoot = Join-Path $AppRoot 'APO'
$packageStreamApo = Join-Path $PackageRoot 'OmniphonyStreamAPO.dll'
$packageStreamSmoke = Join-Path $PackageRoot 'OmniphonyStreamApoSmoke.exe'
$installedStreamApo = Join-Path $runtimeRoot 'OmniphonyStreamAPO.dll'
$stateRoot = Join-Path $env:ProgramData 'Omniphony'
$endpointBackupPath = Join-Path $stateRoot 'endpoint-backup.json'
$streamBackupPath = Join-Path $stateRoot 'stream-backup.json'
$ctl = Join-Path $PackageRoot 'OmniphonyApoCtl.exe'
$mixProbe = Join-Path $PackageRoot 'OmniphonyMixProbe.exe'

$endpointApoClsid = '{A9333BFE-39C1-40FD-B4B0-ECC591410B47}'
$streamApoClsid = '{07D403D9-8A98-43EF-8C28-8651756D83BE}'
$efxValue = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
$sfxValue = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5'
$sfxModesValue = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},5'
$defaultMode = '{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}'

foreach ($path in @($baselineInstaller, $packageStreamApo, $packageStreamSmoke, $ctl, $mixProbe)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing Omniphony package file: $path" }
}

# Establish the already field-proven stereo endpoint path first. This owns the
# endpoint backup, AudioDG compatibility state, rollback and physical WASAPI gate.
& $baselineInstaller -PackageRoot $PackageRoot -AppRoot $AppRoot -AllowUnprotectedAudioDG:$AllowUnprotectedAudioDG

function Set-AudioServiceRunning([bool]$Running) {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($Running -and $service.Status -ne 'Running') { Start-Service -Name AudioSrv }
    if ((-not $Running) -and $service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
}

function Restart-AudioGraph {
    Set-AudioServiceRunning $false
    Start-Sleep -Milliseconds 250
    Set-AudioServiceRunning $true
    Start-Sleep -Milliseconds 1000
}

function Get-ValueSnapshot([Microsoft.Win32.RegistryKey]$Key, [string]$Name) {
    if ($Key.GetValueNames() -notcontains $Name) {
        return [ordered]@{ Exists = $false; Kind = ''; Value = $null }
    }
    $kind = $Key.GetValueKind($Name).ToString()
    $value = $Key.GetValue($Name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if ($kind -eq 'Binary' -and $null -ne $value) { $value = [Convert]::ToBase64String([byte[]]$value) }
    return [ordered]@{ Exists = $true; Kind = $kind; Value = $value }
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

function Get-EndpointFxPath([string]$EndpointId) {
    $lines = @(& $ctl status-id $EndpointId 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    if ($code -ne 0 -and $code -ne 3) {
        throw "Could not resolve endpoint registry identity. helper=$code output=$($lines -join ' | ')"
    }
    $line = $lines | Where-Object { $_.StartsWith("ENDPOINT`t") } | Select-Object -First 1
    if (-not $line) { throw 'Endpoint helper did not return an ENDPOINT identity line.' }
    $parts = $line -split "`t", 4
    if ($parts.Count -lt 4 -or [string]::IsNullOrWhiteSpace($parts[2])) {
        throw "Malformed ENDPOINT identity: $line"
    }
    return "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$($parts[2])\FxProperties"
}

function Open-FxWritable([string]$Path) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $key = $base.OpenSubKey($Path, $true)
    if (-not $key) {
        $base.Dispose()
        throw "Could not open writable endpoint FxProperties after baseline attachment: HKLM\$Path"
    }
    return [pscustomobject]@{ Base = $base; Key = $key }
}

function Register-StreamApo {
    $regsvr32 = Join-Path $env:WINDIR 'System32\regsvr32.exe'
    $process = Start-Process -FilePath $regsvr32 -ArgumentList @('/s', $installedStreamApo) -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Stream APO registration failed: $($process.ExitCode)" }
}

function Unregister-StreamApo {
    if (-not (Test-Path -LiteralPath $installedStreamApo)) { return }
    $regsvr32 = Join-Path $env:WINDIR 'System32\regsvr32.exe'
    $process = Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $installedStreamApo) -Wait -PassThru
    if ($process.ExitCode -ne 0) { Write-Warning "Stream APO unregister returned $($process.ExitCode)" }
}

$endpointBackup = Get-Content -LiteralPath $endpointBackupPath -Raw | ConvertFrom-Json
$endpointId = [string]$endpointBackup.EndpointId
$endpointName = [string]$endpointBackup.EndpointName
$fxPath = Get-EndpointFxPath $endpointId
$streamRegistered = $false
$streamSnapshot = $null
$migrated = $false

try {
    Set-AudioServiceRunning $false
    try {
        Copy-Item -LiteralPath $packageStreamApo -Destination $installedStreamApo -Force
        Register-StreamApo
        $streamRegistered = $true
    } finally {
        Set-AudioServiceRunning $true
    }

    # Exercise both stereo Current and synthetic authored 7.1.4 -> stereo before
    # changing the real endpoint. The smoke loads the same realtime DLL that the
    # live SFX will load from Program Files.
    & $packageStreamSmoke
    if ($LASTEXITCODE -ne 0) { throw "Native-surround stream APO smoke failed: $LASTEXITCODE" }

    $opened = Open-FxWritable $fxPath
    try {
        $priorSfx = Get-ValueSnapshot $opened.Key $sfxValue
        $priorSfxModes = Get-ValueSnapshot $opened.Key $sfxModesValue
        $streamSnapshot = [ordered]@{
            Version = 1
            EndpointId = $endpointId
            EndpointName = $endpointName
            FxPath = $fxPath
            Sfx = $priorSfx
            SfxModes = $priorSfxModes
        }
        $streamSnapshot | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $streamBackupPath -Encoding UTF8

        if ([bool]$priorSfx.Exists -and
            -not [string]::Equals([string]$priorSfx.Value, $streamApoClsid, [StringComparison]::OrdinalIgnoreCase)) {
            Write-Warning "NATIVE_SURROUND_SKIPPED_FOREIGN_SFX $($priorSfx.Value)"
            Unregister-StreamApo
            Write-Host 'OMNIPHONY_WINDOWS_INSTALL_OK 1'
            Write-Host 'Native-surround SFX was not installed because the endpoint already owns another stream effect. Stereo Current remains active.'
            return
        }

        $opened.Key.SetValue($sfxValue, $streamApoClsid, [Microsoft.Win32.RegistryValueKind]::String)
        if (-not [bool]$priorSfxModes.Exists) {
            $opened.Key.SetValue($sfxModesValue, [string[]]@($defaultMode), [Microsoft.Win32.RegistryValueKind]::MultiString)
        }

        $currentEfx = [string]$opened.Key.GetValue($efxValue, '')
        if ([string]::Equals($currentEfx, $endpointApoClsid, [StringComparison]::OrdinalIgnoreCase)) {
            $opened.Key.DeleteValue($efxValue, $false)
        }

        $verify = [string]$opened.Key.GetValue($sfxValue, '')
        if (-not [string]::Equals($verify, $streamApoClsid, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Native-surround SFX registry verification failed.'
        }
        $migrated = $true
    } finally {
        $opened.Key.Dispose()
        $opened.Base.Dispose()
    }

    Restart-AudioGraph
    & $mixProbe $endpointName
    if ($LASTEXITCODE -ne 0) { throw "Physical endpoint failed after native-surround migration: $LASTEXITCODE" }

    Write-Host 'OMNIPHONY_WINDOWS_INSTALL_OK 1'
    Write-Host 'AUDIO_INGRESS stereo=Current multichannel=authored-speaker-bed output=binaural-stereo'
    Write-Host 'NATIVE_SURROUND_SFX 1'
}
catch {
    $failure = $_
    Write-Warning "NATIVE_SURROUND_MIGRATION_FAILED: $($failure.Exception.Message)"

    try {
        if ($streamSnapshot) {
            $opened = Open-FxWritable $fxPath
            try {
                Set-ValueSnapshot $opened.Key $sfxValue $streamSnapshot.Sfx
                Set-ValueSnapshot $opened.Key $sfxModesValue $streamSnapshot.SfxModes
            } finally {
                $opened.Key.Dispose()
                $opened.Base.Dispose()
            }
        }
        # Return to the already-proven endpoint path. This is deliberately a
        # successful product fallback, not a failed installation.
        & $ctl attach-id $endpointId
        Restart-AudioGraph
        & $mixProbe $endpointName
    } catch {
        throw "Native-surround migration failed and stereo rollback also failed: $($_.Exception.Message)"
    } finally {
        if ($streamRegistered) { try { Unregister-StreamApo } catch { Write-Warning $_ } }
    }

    Write-Host 'OMNIPHONY_WINDOWS_INSTALL_OK 1'
    Write-Host 'NATIVE_SURROUND_SFX 0'
    Write-Host 'Stereo Current baseline restored automatically.'
}
