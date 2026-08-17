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
$packageApo = Join-Path $PackageRoot 'OmniphonyAPO.dll'
$packageRealtime = Join-Path $PackageRoot 'omniphony_realtime.dll'
$installedApo = Join-Path $runtimeRoot 'OmniphonyAPO.dll'
$installedRealtime = Join-Path $runtimeRoot 'omniphony_realtime.dll'
$stateRoot = Join-Path $env:ProgramData 'Omniphony'
$backupPath = Join-Path $stateRoot 'endpoint-backup.json'
$logPath = Join-Path $stateRoot 'install-last.log'

$apoClsid = '{A9333BFE-39C1-40FD-B4B0-ECC591410B47}'
$apoInterface = '{FD7F2B29-24D0-4B5C-B177-592C39F9CA10}'
$defaultMode = '{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}'
$efxValueName = '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7'
$efxModesValueName = '{d3993a3f-99c2-4402-b5ec-a92a0367664b},7'
$disableSysFxValueName = '{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5'
$fxValueNames = @(
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},1',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},2',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},5',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},6',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},7',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},13',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},14',
    '{d04e05a6-594b-4fb6-a80d-01af5eed7d1d},15'
)
$knownEqualizerApoClsids = @(
    '{EACD2258-FCAC-4FF4-B36D-419E924A6D79}',
    '{EC1CC9CE-FAED-4822-828A-82A81A6F018F}'
)

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

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the Omniphony installer from an elevated context.'
}
foreach ($path in @($packageApo, $packageRealtime, $ctl, $endpointCtl, $realtimeSmoke, $apoSmoke, $mixProbe)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing package file: $path" }
}

function Open-Hklm64([string]$Path, [bool]$Writable, [bool]$Create) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    $key = $null
    if ($Create) { $key = $base.CreateSubKey($Path, $Writable) }
    else { $key = $base.OpenSubKey($Path, $Writable) }
    if (-not $key) {
        $base.Dispose()
        throw "Could not open HKLM\$Path"
    }
    return [pscustomobject]@{ Base = $base; Key = $key; Path = $Path }
}

function Get-ValueSnapshot([string]$Path, [string]$Name) {
    $opened = Open-Hklm64 $Path $false $false
    try {
        if ($opened.Key.GetValueNames() -notcontains $Name) {
            return [ordered]@{ Exists = $false; Kind = ''; Value = $null }
        }
        $kind = $opened.Key.GetValueKind($Name).ToString()
        $value = $opened.Key.GetValue($Name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if ($kind -eq 'Binary' -and $null -ne $value) { $value = [Convert]::ToBase64String([byte[]]$value) }
        return [ordered]@{ Exists = $true; Kind = $kind; Value = $value }
    } finally {
        $opened.Key.Dispose(); $opened.Base.Dispose()
    }
}

function Set-ValueSnapshot([string]$Path, [string]$Name, $Snapshot) {
    $opened = Open-Hklm64 $Path $true $true
    try {
        if (-not [bool]$Snapshot.Exists) {
            $opened.Key.DeleteValue($Name, $false)
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
        $opened.Key.SetValue($Name, $value, $kind)
    } finally {
        $opened.Key.Dispose(); $opened.Base.Dispose()
    }
}

function Set-RegString([string]$Path, [string]$Name, [string]$Value) {
    $opened = Open-Hklm64 $Path $true $true
    try { $opened.Key.SetValue($Name, $Value, [Microsoft.Win32.RegistryValueKind]::String) }
    finally { $opened.Key.Dispose(); $opened.Base.Dispose() }
}

function Set-RegDword([string]$Path, [string]$Name, [int]$Value) {
    $opened = Open-Hklm64 $Path $true $true
    try { $opened.Key.SetValue($Name, $Value, [Microsoft.Win32.RegistryValueKind]::DWord) }
    finally { $opened.Key.Dispose(); $opened.Base.Dispose() }
}

function Remove-HklmTree([string]$Path) {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    try { $base.DeleteSubKeyTree($Path, $false) } finally { $base.Dispose() }
}

function Stop-LegacyOmniphonyHost {
    Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 200
    if (Get-Process -Name Omniphony -ErrorAction SilentlyContinue) {
        throw 'A legacy Omniphony process is still running.'
    }
    Write-Host 'LEGACY_HOST_RUNNING 0'
}

function Set-AudioServiceRunning([bool]$Running) {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($Running -and $service.Status -ne 'Running') { Start-Service -Name AudioSrv }
    if ((-not $Running) -and $service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
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
    if ($code -ne 0 -or -not $line) { throw "Could not resolve default render endpoint. helper=$code output=$($lines -join ' | ')" }
    $parts = $line -split "`t", 3
    return [pscustomobject]@{ Id = $parts[1]; Name = $parts[2] }
}

function Get-ApoEndpointById([string]$EndpointId) {
    $lines = @(& $ctl list 2>&1 | ForEach-Object { "$_" })
    if ($LASTEXITCODE -ne 0) { throw "Could not enumerate render endpoints: $($lines -join ' | ')" }
    foreach ($line in $lines) {
        if (-not $line.StartsWith("ENDPOINT`t")) { continue }
        $parts = $line -split "`t", 4
        if ($parts.Count -ge 4 -and [string]::Equals($parts[3], $EndpointId, [StringComparison]::OrdinalIgnoreCase)) {
            return [pscustomobject]@{ Name = $parts[1]; Guid = $parts[2]; Id = $parts[3] }
        }
    }
    throw "Default endpoint was not found in APO endpoint list: $EndpointId"
}

function Get-StringValues($Snapshot) {
    if (-not [bool]$Snapshot.Exists) { return @() }
    $kind = [string]$Snapshot.Kind
    if ($kind -eq 'String' -or $kind -eq 'ExpandString') { return @([string]$Snapshot.Value) }
    if ($kind -eq 'MultiString') { return [string[]]@($Snapshot.Value) }
    return @()
}

function Get-ComServerInfo([string]$Clsid) {
    $path = "SOFTWARE\Classes\CLSID\$Clsid\InprocServer32"
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    try {
        $key = $base.OpenSubKey($path, $false)
        if (-not $key) { return [pscustomobject]@{ Registered = $false; Path = ''; Exists = $false } }
        try { $server = [string]$key.GetValue($null, '') } finally { $key.Dispose() }
        if ([string]::IsNullOrWhiteSpace($server)) { return [pscustomobject]@{ Registered = $true; Path = ''; Exists = $false } }
        $expanded = [Environment]::ExpandEnvironmentVariables($server.Trim('"'))
        return [pscustomobject]@{ Registered = $true; Path = $expanded; Exists = (Test-Path -LiteralPath $expanded -PathType Leaf) }
    } finally { $base.Dispose() }
}

function Find-StaleFxRepairs([string]$FxPath, [hashtable]$Snapshots) {
    $repairs = @()
    foreach ($name in $fxValueNames) {
        $snapshot = Get-ValueSnapshot $FxPath $name
        $values = @(Get-StringValues $snapshot)
        if ($values.Count -eq 0) { continue }
        $keep = New-Object System.Collections.Generic.List[string]
        $changed = $false
        foreach ($clsid in $values) {
            if ($clsid -notmatch '^\{[0-9A-Fa-f-]{36}\}$') { $keep.Add($clsid); continue }
            $server = Get-ComServerInfo $clsid
            Write-Host "FX_DIAG`t$name`t$clsid`tREGISTERED=$($server.Registered)`tEXISTS=$($server.Exists)`tPATH=$($server.Path)"
            $knownEapo = $knownEqualizerApoClsids -contains $clsid.ToUpperInvariant()
            $stale = ($server.Registered -and -not $server.Exists) -or ((-not $server.Registered) -and $knownEapo)
            if ($stale) {
                Write-Warning "STALE_FX_WILL_REMOVE $name $clsid"
                $changed = $true
            } else {
                $keep.Add($clsid)
            }
        }
        if ($changed) {
            if (-not $Snapshots.Contains($name)) { $Snapshots[$name] = $snapshot }
            $repairs += [pscustomobject]@{ Name = $name; Kind = [string]$snapshot.Kind; Keep = [string[]]$keep.ToArray() }
        }
    }
    return $repairs
}

function Apply-FxRepairs([string]$FxPath, $Repairs) {
    foreach ($repair in @($Repairs)) {
        $opened = Open-Hklm64 $FxPath $true $false
        try {
            $values = [string[]]@($repair.Keep)
            if ($values.Count -eq 0) {
                $opened.Key.DeleteValue([string]$repair.Name, $false)
            } elseif ([string]$repair.Kind -eq 'MultiString') {
                $opened.Key.SetValue([string]$repair.Name, $values, [Microsoft.Win32.RegistryValueKind]::MultiString)
            } else {
                $opened.Key.SetValue([string]$repair.Name, $values[0], [Microsoft.Win32.RegistryValueKind]::String)
            }
            Write-Host "STALE_FX_REPAIRED`t$($repair.Name)"
        } finally { $opened.Key.Dispose(); $opened.Base.Dispose() }
    }
}

function Save-Backup($Backup) {
    $Backup | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $backupPath -Encoding UTF8
    Write-Host "ENDPOINT_BACKUP_SAVED $backupPath"
}

function Restore-Backup($Backup) {
    if (-not $Backup) { return }
    $fxPath = [string]$Backup.FxPath
    if ($Backup.FxSnapshots) {
        foreach ($property in $Backup.FxSnapshots.PSObject.Properties) {
            Set-ValueSnapshot $fxPath $property.Name $property.Value
        }
    }
    if ($Backup.AudioProtection) {
        Set-ValueSnapshot 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG' $Backup.AudioProtection
    }
}

function Register-OmniphonyApo([string]$DllPath) {
    $classPath = "SOFTWARE\Classes\CLSID\$apoClsid"
    $serverPath = "$classPath\InprocServer32"
    $apoPath = "SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$apoClsid"

    Remove-HklmTree $classPath
    Remove-HklmTree $apoPath

    Set-RegString $classPath '' 'Omniphony Endpoint APO'
    Set-RegString $serverPath '' $DllPath
    Set-RegString $serverPath 'ThreadingModel' 'Both'

    Set-RegString $apoPath 'FriendlyName' 'Omniphony Endpoint APO'
    Set-RegString $apoPath 'Copyright' 'Omniphony downstream fork'
    Set-RegDword $apoPath 'MajorVersion' 1
    Set-RegDword $apoPath 'MinorVersion' 0
    Set-RegDword $apoPath 'Flags' 15
    Set-RegDword $apoPath 'MinInputConnections' 1
    Set-RegDword $apoPath 'MaxInputConnections' 1
    Set-RegDword $apoPath 'MinOutputConnections' 1
    Set-RegDword $apoPath 'MaxOutputConnections' 1
    Set-RegDword $apoPath 'MaxInstances' -1
    Set-RegDword $apoPath 'NumAPOInterfaces' 1
    Set-RegString $apoPath 'APOInterface0' $apoInterface
    Write-Host 'APO_GLOBAL_REGISTRATION_OK 1'
}

function Unregister-OmniphonyApo {
    Remove-HklmTree "SOFTWARE\Classes\AudioEngine\AudioProcessingObjects\$apoClsid"
    Remove-HklmTree "SOFTWARE\Classes\CLSID\$apoClsid"
}

$backup = $null
try {
    & $realtimeSmoke $packageRealtime
    if ($LASTEXITCODE -ne 0) { throw "Realtime renderer self-test failed: $LASTEXITCODE" }

    Stop-LegacyOmniphonyHost
    Restart-AudioGraph

    $defaultEndpoint = Get-CurrentDefaultEndpoint
    $apoEndpoint = Get-ApoEndpointById $defaultEndpoint.Id
    $fxPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$($apoEndpoint.Guid)\FxProperties"
    Write-Host "TARGET_DEFAULT`t$($defaultEndpoint.Name)`t$($defaultEndpoint.Id)"
    Write-Host "TARGET_GUID`t$($apoEndpoint.Guid)"

    $snapshots = [ordered]@{}
    foreach ($name in @($efxValueName, $efxModesValueName, $disableSysFxValueName)) {
        $snapshots[$name] = Get-ValueSnapshot $fxPath $name
    }
    $repairs = @(Find-StaleFxRepairs $fxPath $snapshots)
    $backup = [ordered]@{
        Version = 3
        EndpointId = $defaultEndpoint.Id
        EndpointName = $defaultEndpoint.Name
        EndpointGuid = $apoEndpoint.Guid
        FxPath = $fxPath
        FxSnapshots = $snapshots
        AudioProtection = Get-ValueSnapshot 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG'
    }
    Save-Backup $backup

    if ($repairs.Count -gt 0) {
        Apply-FxRepairs $fxPath $repairs
        Restart-AudioGraph
    }

    & $mixProbe $defaultEndpoint.Name
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "PREINSTALL_WASAPI_FAILED $LASTEXITCODE; continuing so Omniphony can replace the endpoint effect."
    } else {
        Write-Host 'PREINSTALL_WASAPI_OK 1'
    }

    Set-AudioServiceRunning $false
    try {
        New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
        Copy-Item -LiteralPath $packageApo -Destination $installedApo -Force
        Copy-Item -LiteralPath $packageRealtime -Destination $installedRealtime -Force

        Register-OmniphonyApo $installedApo
        Set-RegDword 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG' 1

        $opened = Open-Hklm64 $fxPath $true $false
        try {
            $opened.Key.SetValue($efxValueName, $apoClsid, [Microsoft.Win32.RegistryValueKind]::String)
            if ($opened.Key.GetValueNames() -notcontains $efxModesValueName) {
                $opened.Key.SetValue($efxModesValueName, [string[]]@($defaultMode), [Microsoft.Win32.RegistryValueKind]::MultiString)
            }
            $opened.Key.SetValue($disableSysFxValueName, 0, [Microsoft.Win32.RegistryValueKind]::DWord)
        } finally { $opened.Key.Dispose(); $opened.Base.Dispose() }
    } finally {
        Set-AudioServiceRunning $true
    }

    Start-Sleep -Milliseconds 1200
    & $apoSmoke
    if ($LASTEXITCODE -ne 0) { throw "Omniphony APO COM/processing smoke failed: $LASTEXITCODE" }

    & $mixProbe $defaultEndpoint.Name
    if ($LASTEXITCODE -ne 0) { throw "Physical endpoint failed post-install WASAPI probe: $LASTEXITCODE" }

    Write-Host ''
    Write-Host 'OMNIPHONY_APO_INSTALL_OK 1'
    Write-Host "Runtime installed at: $runtimeRoot"
    Write-Host "Endpoint: $($defaultEndpoint.Name)"
    Write-Host 'Current renderer is active through the endpoint APO.'
    Write-Host "Diagnostics: $logPath"
}
catch {
    $failure = $_
    Write-Warning "OMNIPHONY_INSTALL_FAILED: $($failure.Exception.Message)"
    try {
        Set-AudioServiceRunning $false
        if ($backup) { Restore-Backup $backup }
        Unregister-OmniphonyApo
    } catch {
        Write-Warning "Rollback warning: $($_.Exception.Message)"
    } finally {
        try { Set-AudioServiceRunning $true } catch { }
    }
    throw $failure
}
finally {
    if ($transcriptStarted) { try { Stop-Transcript | Out-Null } catch { } }
}
