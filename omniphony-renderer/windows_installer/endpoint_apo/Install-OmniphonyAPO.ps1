param(
    [string]$PhysicalOutput = '',
    [string]$PackageRoot = '',
    [string]$AppRoot = ''
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

if ([string]::IsNullOrWhiteSpace($PackageRoot)) { $PackageRoot = $here }
if ([string]::IsNullOrWhiteSpace($AppRoot)) {
    $AppRoot = Join-Path $env:ProgramFiles 'Omniphony'
    $supportRoot = $here
} else {
    $supportRoot = Join-Path $AppRoot 'support'
}

$runtimeRoot = Join-Path $AppRoot 'APO'
$legacyRuntimeRoot = Join-Path $AppRoot 'EndpointAPO'
$packageApo = Join-Path $PackageRoot 'OmniphonyAPO.dll'
$packageRealtime = Join-Path $PackageRoot 'omniphony_realtime.dll'
$installedApo = Join-Path $runtimeRoot 'OmniphonyAPO.dll'
$installedRealtime = Join-Path $runtimeRoot 'omniphony_realtime.dll'
$stateRoot = Join-Path $env:ProgramData 'Omniphony'
$backupPath = Join-Path $stateRoot 'endpoint-backup.json'
$logPath = Join-Path $stateRoot 'install-last.log'

$efxValueName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
$efxModesValueName = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},7'
$disableSysFxValueName = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'
$ourClsid = '{A9333BFE-39C1-40FD-B4B0-ECC591410B47}'
$defaultMode = '{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}'

New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null
$transcriptStarted = $false
try {
    Start-Transcript -Path $logPath -Force | Out-Null
    $transcriptStarted = $true
} catch {
    Write-Warning "Could not start installer transcript: $($_.Exception.Message)"
}

function Resolve-Tool([string]$Name) {
    $fromPackage = Join-Path $PackageRoot $Name
    if (Test-Path -LiteralPath $fromPackage) { return $fromPackage }
    $fromSupport = Join-Path $supportRoot $Name
    if (Test-Path -LiteralPath $fromSupport) { return $fromSupport }
    throw "Missing Omniphony installation helper: $Name"
}

$ctl = Resolve-Tool 'OmniphonyApoCtl.exe'
$endpointCtl = Resolve-Tool 'OmniphonyEndpointCtl.exe'
$realtimeSmoke = Resolve-Tool 'OmniphonyRealtimeSmoke.exe'
$apoSmoke = Resolve-Tool 'OmniphonyApoSmoke.exe'
$mixProbe = Resolve-Tool 'OmniphonyMixProbe.exe'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the Omniphony installer from an elevated context.'
}

foreach ($path in @($packageApo, $packageRealtime, $ctl, $endpointCtl, $realtimeSmoke, $apoSmoke, $mixProbe)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing package file: $path" }
}

function Stop-LegacyOmniphonyHost {
    $running = @(Get-Process -Name Omniphony -ErrorAction SilentlyContinue)
    foreach ($process in $running) {
        $path = $null
        try { $path = $process.Path } catch { }
        Write-Host "STOP_LEGACY_HOST PID=$($process.Id) PATH=$path"
        Stop-Process -Id $process.Id -Force -ErrorAction Stop
    }
    Start-Sleep -Milliseconds 250
    if (Get-Process -Name Omniphony -ErrorAction SilentlyContinue) {
        throw 'A legacy Omniphony process is still running after the installer attempted to stop it.'
    }
    Write-Host 'LEGACY_HOST_RUNNING 0'
}

function Set-AudioServiceRunning([bool]$Running) {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($Running) {
        if ($service.Status -ne 'Running') {
            Start-Service -Name AudioSrv
        }
    } else {
        if ($service.Status -ne 'Stopped') {
            Stop-Service -Name AudioSrv -Force
        }
    }
}

function Restart-AudioGraph {
    Write-Host 'AUDIO_GRAPH_RESET_BEGIN'
    Set-AudioServiceRunning $false
    Start-Sleep -Milliseconds 250
    Set-AudioServiceRunning $true
    Start-Sleep -Milliseconds 1000
    Write-Host 'AUDIO_GRAPH_RESET_OK'
}

function Get-CurrentDefaultEndpoint {
    $lines = @(& $endpointCtl get-default 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    $line = $lines | Where-Object { $_.StartsWith("DEFAULT`t") } | Select-Object -First 1
    if ($code -ne 0 -or -not $line) {
        throw "Could not resolve the current Windows default render endpoint. helper=$code output=$($lines -join ' | ')"
    }
    $parts = $line -split "`t", 3
    if ($parts.Count -lt 3) { throw "Malformed default-endpoint response: $line" }
    return [pscustomobject]@{ Id = $parts[1]; Name = $parts[2] }
}

function Get-ApoEndpointById([string]$EndpointId) {
    $lines = @(& $ctl list 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        throw "Could not enumerate active render endpoints for APO attachment. helper=$code output=$($lines -join ' | ')"
    }
    foreach ($line in $lines) {
        if (-not $line.StartsWith("ENDPOINT`t")) { continue }
        $parts = $line -split "`t", 4
        if ($parts.Count -lt 4) { continue }
        if ([string]::Equals($parts[3], $EndpointId, [StringComparison]::OrdinalIgnoreCase)) {
            return [pscustomobject]@{ Name = $parts[1]; Guid = $parts[2]; Id = $parts[3] }
        }
    }
    throw "The current Windows default endpoint was not found in the active APO endpoint list: $EndpointId"
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

function Get-RegistrySnapshot([string]$EndpointGuid, [string]$Name) {
    $opened = Open-FxKey $EndpointGuid $false
    try {
        if ($opened.Key.GetValueNames() -notcontains $Name) {
            return [ordered]@{ Exists = $false; Kind = ''; Value = $null }
        }
        $kind = $opened.Key.GetValueKind($Name).ToString()
        $value = $opened.Key.GetValue(
            $Name, $null,
            [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if ($kind -eq 'Binary' -and $null -ne $value) {
            $value = [Convert]::ToBase64String([byte[]]$value)
        }
        return [ordered]@{ Exists = $true; Kind = $kind; Value = $value }
    } finally {
        $opened.Key.Dispose()
        $opened.Base.Dispose()
    }
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

function Read-Backup {
    if (-not (Test-Path -LiteralPath $backupPath)) { return $null }
    try {
        return (Get-Content -LiteralPath $backupPath -Raw | ConvertFrom-Json)
    } catch {
        Write-Warning "Ignoring unreadable endpoint backup: $($_.Exception.Message)"
        return $null
    }
}

function Save-Backup($Backup) {
    $Backup | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $backupPath -Encoding UTF8
    Write-Host "ENDPOINT_BACKUP_SAVED $backupPath"
}

function Capture-EndpointBackup($DefaultEndpoint, $ApoEndpoint) {
    $existing = Read-Backup
    $currentEfx = Get-RegistrySnapshot $ApoEndpoint.Guid $efxValueName

    if ($existing -and
        [string]::Equals([string]$existing.EndpointId, $DefaultEndpoint.Id, [StringComparison]::OrdinalIgnoreCase) -and
        [bool]$currentEfx.Exists -and
        [string]::Equals([string]$currentEfx.Value, $ourClsid, [StringComparison]::OrdinalIgnoreCase)) {
        Write-Host 'ENDPOINT_BACKUP_REUSED 1'
        return $existing
    }

    $backupEfx = $currentEfx
    if (-not $existing -and
        [bool]$currentEfx.Exists -and
        [string]::Equals([string]$currentEfx.Value, $ourClsid, [StringComparison]::OrdinalIgnoreCase)) {
        $backupEfx = [ordered]@{ Exists = $false; Kind = ''; Value = $null }
    }

    $backup = [ordered]@{
        Version = 2
        EndpointId = $DefaultEndpoint.Id
        EndpointName = $DefaultEndpoint.Name
        EndpointGuid = $ApoEndpoint.Guid
        Efx = $backupEfx
        EfxModes = Get-RegistrySnapshot $ApoEndpoint.Guid $efxModesValueName
        DisableSysFx = Get-RegistrySnapshot $ApoEndpoint.Guid $disableSysFxValueName
    }
    Save-Backup $backup
    return $backup
}

function Restore-EndpointBackup($Backup) {
    if (-not $Backup) { return }
    $guid = [string]$Backup.EndpointGuid
    if ([string]::IsNullOrWhiteSpace($guid)) { return }

    Write-Warning "Restoring previous endpoint effect state for $($Backup.EndpointName)"
    Set-RegistrySnapshot $guid $efxValueName $Backup.Efx
    Set-RegistrySnapshot $guid $efxModesValueName $Backup.EfxModes
    Set-RegistrySnapshot $guid $disableSysFxValueName $Backup.DisableSysFx
}

function Apply-OmniphonyEndpointState($ApoEndpoint) {
    $opened = Open-FxKey $ApoEndpoint.Guid $true
    try {
        $opened.Key.SetValue(
            $efxValueName, $ourClsid,
            [Microsoft.Win32.RegistryValueKind]::String)

        if ($opened.Key.GetValueNames() -notcontains $efxModesValueName) {
            $opened.Key.SetValue(
                $efxModesValueName, [string[]]@($defaultMode),
                [Microsoft.Win32.RegistryValueKind]::MultiString)
        }

        $opened.Key.SetValue(
            $disableSysFxValueName, [int]0,
            [Microsoft.Win32.RegistryValueKind]::DWord)
    } finally {
        $opened.Key.Dispose()
        $opened.Base.Dispose()
    }
    Write-Host "APO_ATTACHED_DEFAULT`t$($ApoEndpoint.Name)`t$($ApoEndpoint.Guid)"
}

function Test-OmniphonyRegistryState([string]$EndpointGuid) {
    $efx = Get-RegistrySnapshot $EndpointGuid $efxValueName
    $disabled = Get-RegistrySnapshot $EndpointGuid $disableSysFxValueName
    return (
        [bool]$efx.Exists -and
        [string]::Equals([string]$efx.Value, $ourClsid, [StringComparison]::OrdinalIgnoreCase) -and
        [bool]$disabled.Exists -and
        ([int]$disabled.Value -eq 0)
    )
}

function Unregister-InstalledApoBestEffort {
    if (Test-Path -LiteralPath $installedApo) {
        try {
            $unregister = Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $installedApo) -Wait -PassThru
            if ($unregister.ExitCode -ne 0) {
                Write-Warning "APO rollback unregistration returned $($unregister.ExitCode)"
            }
        } catch {
            Write-Warning "APO rollback unregistration failed: $($_.Exception.Message)"
        }
    }
}

$backup = $null
$endpointTouched = $false

try {
    & $realtimeSmoke $packageRealtime
    if ($LASTEXITCODE -ne 0) {
        throw "Omniphony realtime ABI self-test failed before installation: $LASTEXITCODE"
    }

    Stop-LegacyOmniphonyHost
    Restart-AudioGraph

    $defaultEndpoint = Get-CurrentDefaultEndpoint
    $apoEndpoint = Get-ApoEndpointById $defaultEndpoint.Id
    Write-Host "TARGET_DEFAULT`t$($defaultEndpoint.Name)`t$($defaultEndpoint.Id)"
    Write-Host "TARGET_GUID`t$($apoEndpoint.Guid)"

    & $mixProbe $defaultEndpoint.Name
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Pre-install WASAPI probe failed with $LASTEXITCODE; continuing after the audio-graph reset so Omniphony can replace stale endpoint effects."
    } else {
        Write-Host 'PREINSTALL_WASAPI_OK 1'
    }

    $backup = Capture-EndpointBackup $defaultEndpoint $apoEndpoint

    Set-AudioServiceRunning $false
    try {
        New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
        Copy-Item -LiteralPath $packageApo -Destination $installedApo -Force
        Copy-Item -LiteralPath $packageRealtime -Destination $installedRealtime -Force

        if (Test-Path -LiteralPath $legacyRuntimeRoot) {
            Remove-Item -LiteralPath $legacyRuntimeRoot -Recurse -Force -ErrorAction SilentlyContinue
        }

        $register = Start-Process -FilePath $regsvr32 -ArgumentList @('/s', $installedApo) -Wait -PassThru
        if ($register.ExitCode -ne 0) {
            throw "APO COM registration failed from installed runtime path: $($register.ExitCode)"
        }

        Apply-OmniphonyEndpointState $apoEndpoint
        $endpointTouched = $true
    } finally {
        Set-AudioServiceRunning $true
    }

    Start-Sleep -Milliseconds 1000

    & $apoSmoke
    if ($LASTEXITCODE -ne 0) {
        throw "Omniphony APO processing self-test failed after endpoint attachment: $LASTEXITCODE"
    }

    $after = Get-CurrentDefaultEndpoint
    if (-not [string]::Equals($after.Id, $defaultEndpoint.Id, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Windows default render endpoint changed during installation. before=$($defaultEndpoint.Id) after=$($after.Id)"
    }

    if (-not (Test-OmniphonyRegistryState $apoEndpoint.Guid)) {
        throw 'Omniphony endpoint effect state did not survive the Windows Audio restart.'
    }

    & $mixProbe $defaultEndpoint.Name
    if ($LASTEXITCODE -ne 0) {
        throw "Physical endpoint failed the post-attach WASAPI GetMixFormat probe: $LASTEXITCODE"
    }

    Write-Host ''
    Write-Host 'OMNIPHONY_APO_INSTALL_OK 1'
    Write-Host "Runtime installed at: $runtimeRoot"
    Write-Host "Endpoint: $($defaultEndpoint.Name)"
    Write-Host 'The existing Windows default output was preserved.'
    Write-Host 'System effects were enabled for the target endpoint so AudioDG can load Omniphony.'
    Write-Host 'The prior endpoint-effect state is saved for rollback/uninstall.'
    Write-Host "Diagnostics: $logPath"
}
catch {
    $failure = $_
    Write-Warning "OMNIPHONY_INSTALL_FAILED: $($failure.Exception.Message)"
    try {
        Set-AudioServiceRunning $false
        if ($endpointTouched -and $backup) {
            Restore-EndpointBackup $backup
        }
        Unregister-InstalledApoBestEffort
    } catch {
        Write-Warning "Rollback encountered an additional error: $($_.Exception.Message)"
    } finally {
        try {
            Set-AudioServiceRunning $true
            Start-Sleep -Milliseconds 1000
        } catch { }
    }
    throw $failure
}
finally {
    if ($transcriptStarted) {
        try { Stop-Transcript | Out-Null } catch { }
    }
}
