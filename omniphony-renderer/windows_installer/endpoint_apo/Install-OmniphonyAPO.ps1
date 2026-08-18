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
    throw 'Omniphony 0.1 uses an unsigned user-mode endpoint APO. Its installer must explicitly enable the Windows unprotected AudioDG compatibility mode.'
}
Write-Host 'OMNIPHONY_UNSIGNED_APO_MODE 1'
Write-Host 'AudioDG compatibility mode is enabled for the installed Omniphony APO and restored to the previous machine state by rollback or uninstall.'

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

function Wait-ServiceState([string]$Name, [System.ServiceProcess.ServiceControllerStatus]$State) {
    $service = Get-Service -Name $Name -ErrorAction Stop
    $service.WaitForStatus($State, [TimeSpan]::FromSeconds(10))
    $service.Refresh()
    if ($service.Status -ne $State) {
        throw "Windows service '$Name' did not reach state '$State'. observed=$($service.Status)"
    }
}

function Set-AudioServiceRunning([bool]$Running) {
    if ($Running) {
        $builder = Get-Service -Name AudioEndpointBuilder -ErrorAction Stop
        if ($builder.Status -ne 'Running') { Start-Service -Name AudioEndpointBuilder }
        Wait-ServiceState 'AudioEndpointBuilder' ([System.ServiceProcess.ServiceControllerStatus]::Running)

        $service = Get-Service -Name AudioSrv -ErrorAction Stop
        if ($service.Status -ne 'Running') { Start-Service -Name AudioSrv }
        Wait-ServiceState 'AudioSrv' ([System.ServiceProcess.ServiceControllerStatus]::Running)
        Start-Sleep -Milliseconds 500
        return
    }

    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($service.Status -ne 'Stopped') { Stop-Service -Name AudioSrv -Force }
    Wait-ServiceState 'AudioSrv' ([System.ServiceProcess.ServiceControllerStatus]::Stopped)
}

function Restart-AudioGraph {
    Write-Host 'AUDIO_GRAPH_RESET_BEGIN'
    Set-AudioServiceRunning $false
    Start-Sleep -Milliseconds 250
    Set-AudioServiceRunning $true
    Write-Host 'AUDIO_GRAPH_RESET_OK'
}

function Invoke-EndpointCtlCapture([string[]]$Arguments) {
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $lines = @(& $endpointCtl @Arguments 2>&1 | ForEach-Object { "$_" })
        $code = $LASTEXITCODE
        if ($null -eq $code) { $code = 0 }
        return [pscustomobject]@{ Code = [int]$code; Lines = [string[]]$lines }
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
}

$script:lastEndpointProbe = ''
function Try-GetCurrentDefaultEndpoint {
    $result = Invoke-EndpointCtlCapture @('get-default')
    $script:lastEndpointProbe = "helper=$($result.Code) output=$($result.Lines -join ' | ')"
    $line = $result.Lines | Where-Object { $_.StartsWith("DEFAULT`t") } | Select-Object -First 1
    if ($result.Code -ne 0 -or -not $line) { return $null }
    $parts = $line -split "`t", 3
    if ($parts.Count -lt 3) { return $null }
    return [pscustomobject]@{ Id = $parts[1]; Name = $parts[2] }
}

function Get-KnownEndpointBackup {
    if (-not (Test-Path -LiteralPath $backupPath)) { return $null }
    try {
        $saved = Get-Content -LiteralPath $backupPath -Raw | ConvertFrom-Json
        $id = [string]$saved.EndpointId
        $name = [string]$saved.EndpointName
        if ([string]::IsNullOrWhiteSpace($id) -or [string]::IsNullOrWhiteSpace($name)) { return $null }
        if (-not [string]::IsNullOrWhiteSpace($PhysicalOutput) -and
            $name.IndexOf($PhysicalOutput, [StringComparison]::OrdinalIgnoreCase) -lt 0) {
            Write-Warning "Ignoring endpoint backup because it does not match requested physical output '$PhysicalOutput': $name"
            return $null
        }
        return [pscustomobject]@{ Id = $id; Name = $name }
    }
    catch {
        Write-Warning "Could not read endpoint backup for recovery: $($_.Exception.Message)"
        return $null
    }
}

function Resolve-InstallEndpoint {
    $resolved = Try-GetCurrentDefaultEndpoint
    if ($resolved) { return $resolved }

    Write-Warning "DEFAULT_ENDPOINT_RECOVERY audio-graph-reset; $script:lastEndpointProbe"
    Restart-AudioGraph
    $resolved = Try-GetCurrentDefaultEndpoint
    if ($resolved) {
        Write-Host 'DEFAULT_ENDPOINT_RECOVERY_OK SOURCE=audio-graph-reset'
        return $resolved
    }

    $known = Get-KnownEndpointBackup
    if ($known) {
        Write-Warning "DEFAULT_ENDPOINT_RECOVERY known-endpoint-id $($known.Id) $($known.Name)"
        $setResult = Invoke-EndpointCtlCapture @('set-default-id', $known.Id)
        $setResult.Lines | ForEach-Object { Write-Host $_ }

        Restart-AudioGraph
        $resolved = Try-GetCurrentDefaultEndpoint
        if ($resolved -and [string]::Equals($resolved.Id, $known.Id, [StringComparison]::OrdinalIgnoreCase)) {
            Write-Host 'DEFAULT_ENDPOINT_RECOVERY_OK SOURCE=endpoint-backup'
            return $resolved
        }

        throw "Known Omniphony endpoint did not return ACTIVE after default-role reassertion and audio reset. endpoint=$($known.Id) set_helper=$($setResult.Code) $script:lastEndpointProbe"
    }

    throw "Could not resolve an ACTIVE render endpoint and no verified Omniphony endpoint backup was available. $script:lastEndpointProbe"
}

function Get-ApoStatus([string]$EndpointId) {
    $lines = @(& $ctl status-id $EndpointId 2>&1 | ForEach-Object { "$_" })
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
$hadKnownBackupAtStart = Test-Path -LiteralPath $backupPath
$globalRegistrationTouched = $false
try {
    & $realtimeSmoke $packageRealtime
    if ($LASTEXITCODE -ne 0) { throw "Realtime renderer self-test failed: $LASTEXITCODE" }

    Stop-LegacyOmniphonyHost

    # A prior failed preflight used to unregister the globally installed APO even
    # though an already-working endpoint still referenced it. If local endpoint
    # state exists, repair that project-owned registration before asking Core
    # Audio to enumerate the endpoint again.
    if ($hadKnownBackupAtStart) {
        Set-AudioServiceRunning $false
        try {
            New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
            Copy-Item -LiteralPath $packageApo -Destination $installedApo -Force
            Copy-Item -LiteralPath $packageRealtime -Destination $installedRealtime -Force
            Register-OmniphonyApo $installedApo
            $globalRegistrationTouched = $true
        }
        finally {
            Set-AudioServiceRunning $true
        }
        Write-Host 'KNOWN_ENDPOINT_GLOBAL_APO_REPAIR_OK 1'
    }
    else {
        Set-AudioServiceRunning $true
    }

    $defaultEndpoint = Resolve-InstallEndpoint
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
        $globalRegistrationTouched = $true
        Set-RegDword 'SOFTWARE\Microsoft\Windows\CurrentVersion\Audio' 'DisableProtectedAudioDG' 1
    } finally {
        Set-AudioServiceRunning $true
    }

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

    # Never dismantle a previously known installation merely because endpoint
    # discovery failed. That leaves the physical endpoint referencing an
    # unregistered effect and can make the entire render graph disappear.
    if ($globalRegistrationTouched -and -not $hadKnownBackupAtStart) {
        try { Unregister-OmniphonyApo } catch { Write-Warning "Global APO rollback warning: $($_.Exception.Message)" }
    }
    elseif ($globalRegistrationTouched) {
        Write-Warning 'FAILSAFE retaining global Omniphony APO registration for previously known endpoint state.'
    }

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
