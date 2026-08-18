param(
    [string]$PhysicalOutput = '',
    [string]$PackageRoot = '',
    [string]$AppRoot = '',
    [switch]$AllowUnprotectedAudioDG
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
if (-not $AllowUnprotectedAudioDG) {
    throw 'This is the legacy development bring-up installer. It requires -AllowUnprotectedAudioDG because it temporarily sets DisableProtectedAudioDG=1. Use the production DriverStore package path for protected AudioDG deployment.'
}
Write-Warning 'DEVELOPMENT INSTALL: explicit -AllowUnprotectedAudioDG accepted. AudioDG protection will be disabled only for this bring-up path and restored by rollback/uninstall state handling.'

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
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
        [Microsoft.Win32.RegistryHive]::LocalMachine,
        [Microsoft.Win32.RegistryView]::Registry64)
    try {
        $key = $base.OpenSubKey($Path, $false)
        if (-not $key) { return [ordered]@{ Exists = $false; Kind = ''; Value = $null } }
        try {
            if ($key.GetValueNames() -notcontains $Name) {
                return [ordered]@{ Exists = $false; Kind = ''; Value = $null }
            }
            $kind = $key.GetValueKind($Name).ToString()
            $value = $key.GetValue($Name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            if ($kind -eq 'Binary' -and $null -ne $value) { $value = [Convert]::ToBase64String([byte[]]$value) }
            return [ordered]@{ Exists = $true; Kind = $kind; Value = $value }
        } finally { $key.Dispose() }
    } finally { $base.Dispose() }
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
    } finally { $opened.Key.Dispose(); $opened.Base.Dispose() }
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
    if ($code -ne 0 -or -not $line) {
        throw "Could not resolve default render endpoint. helper=$code output=$($lines -join ' | ')"
    }
    $parts = $line -split "`t", 3
    return [pscustomobject]@{ Id = $parts[1]; Name = $parts[2] }
}

function Get-ApoStatus([string]$EndpointId) {
    $lines = @(& $ctl status-id $EndpointId.Id 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    if ($code -ne 0 -and $code -ne 3) {
        throw "Could not inspect endpoint APO state. helper=$code output=$($lines -join ' | ')"
    }
    $efxLine = $lines | Where-Object { $_.StartsWith("EFX`t") } | Select-Object -First 1
    $disabledLine = $lines | Where-Object { $_.StartsWith("ENHANCEMENTS_DISABLED`t") } | Select-Object -First 1
    $efx = if ($efxLine) { ($efxLine -split "`t", 2)[1] } else { '<unknown>' }
    $disabled = if ($disabledLine) { [int](($disabledLine -split "`t", 2)[1]) } else { -1 }
    return [pscustomobject]@{ Efx = $efx; EnhancementsDisabled = $disabled; IsOmniphony = ($code -eq 0) }
}

function Save-Backup($Backup) {
    $Backup | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $backupPath -Encoding UTF8
    Write-Host "ENDPOINT_BACKUP_SAVED $backupPath"
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
$defaultEndpoint = $null
$attached = $false
try {
    & $realtimeSmoke $packageRealtime
    if ($LASTEXITCODE -ne 0) { throw "Realtime renderer self-test failed: $LASTEXITCODE" }

    Stop-LegacyOmniphonyHost
    Set-AudioServiceRunning $true
    $defaultEndpoint = Get-CurrentDefaultEndpoint
    $status = Get-ApoStatus $defaultEndpoint.Id
    Write-Host "TARGET_DEFAULT`t$($defaultEndpoint.Name)`t$($defaultEndpoint.Id)"
    Write-Host "PREVIOUS_EFX`t$($status.Efx)"
    Write-Host "PREVIOUS_ENHANCEMENTS_DISABLED`t$($status.EnhancementsDisabled)"

    if ($status.Efx -ne '<absent>' -and -not $status.IsOmniphony) {
        throw "The default endpoint already has a different endpoint effect: $($status.Efx)"
    }

    $backup = [ordered]@{
        Version = 4
        EndpointId = $defaultEndpoint.Id
        EndpointName = $defaultEndpoint.Name
        PriorOmniphonyEfx = [bool]$status.IsOmniphony
        PriorEnhancementsDisabled = [int]$status.EnhancementsDisabled
        AudioProtection = Get-ValueSnapshot 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG'
    }
    Save-Backup $backup

    & $mixProbe $defaultEndpoint.Name
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "PREINSTALL_WASAPI_FAILED $LASTEXITCODE; continuing because the endpoint effect registration is about to be repaired."
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
    } finally {
        Set-AudioServiceRunning $true
    }

    Start-Sleep -Milliseconds 500
    $attachOutput = @(& $ctl attach-id $defaultEndpoint.Id 2>&1 | ForEach-Object { "$_" })
    $attachCode = $LASTEXITCODE
    $attachOutput | ForEach-Object { Write-Host $_ }
    if ($attachCode -ne 0) {
        throw "Windows audio policy could not attach/enable the Omniphony endpoint effect. helper=$attachCode"
    }
    $attached = $true

    Restart-AudioGraph

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

    if ($defaultEndpoint) {
        try {
            Set-AudioServiceRunning $true
            $bypassOutput = @(& $ctl bypass-id $defaultEndpoint.Id 2>&1 | ForEach-Object { "$_" })
            $bypassOutput | ForEach-Object { Write-Warning "FAILSAFE $($_)" }
        } catch {
            Write-Warning "Could not force the endpoint into system-effects bypass: $($_.Exception.Message)"
        }
        try {
            $detachOutput = @(& $ctl detach-id $defaultEndpoint.Id 2>&1 | ForEach-Object { "$_" })
            $detachOutput | ForEach-Object { Write-Warning "FAILSAFE $($_)" }
        } catch {
            Write-Warning "Could not detach the failed Omniphony endpoint effect: $($_.Exception.Message)"
        }
        try { Restart-AudioGraph } catch { Write-Warning "Audio graph restart during rollback failed: $($_.Exception.Message)" }
    }

    try { Unregister-OmniphonyApo } catch { Write-Warning "Global APO rollback warning: $($_.Exception.Message)" }
    if ($backup -and $backup.AudioProtection) {
        try {
            Set-ValueSnapshot 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG' $backup.AudioProtection
        } catch {
            Write-Warning "Audio-protection rollback warning: $($_.Exception.Message)"
        }
    }

    throw $failure
}
finally {
    if ($transcriptStarted) { try { Stop-Transcript | Out-Null } catch { } }
}
