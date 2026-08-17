param(
    [string]$PhysicalOutput = 'Dan Clark Noire X',
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
    if ($running.Count -eq 0) {
        Write-Host 'LEGACY_HOST_RUNNING 0'
        return
    }

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

function Rollback-EndpointAttachment {
    Write-Warning 'Rolling back Omniphony APO attachment.'
    try {
        & $ctl detach $PhysicalOutput 'FiiO' 'Noire' | Out-Host
    } catch {
        Write-Warning "APO detach rollback failed: $($_.Exception.Message)"
    }
    try {
        Set-AudioServiceRunning $false
        Unregister-InstalledApoBestEffort
        Set-AudioServiceRunning $true
        Start-Sleep -Milliseconds 750
    } catch {
        Write-Warning "Audio rollback failed: $($_.Exception.Message)"
        try { Set-AudioServiceRunning $true } catch { }
    }
}

# Fail before touching Windows audio if the portable engine cannot initialize.
& $realtimeSmoke $packageRealtime
if ($LASTEXITCODE -ne 0) {
    throw "Omniphony realtime ABI self-test failed before installation: $LASTEXITCODE"
}

# Migration invariant: the old loopback/tray process must not coexist with the
# native APO. It can otherwise repair obsolete routing or hold audio state while
# the physical endpoint is being reconfigured.
Stop-LegacyOmniphonyHost

# Future upgrades are safe even if AudioDG currently has the installed APO DLL
# loaded: stop AudioSrv before replacing the two runtime files.
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
}
finally {
    Set-AudioServiceRunning $true
}
Start-Sleep -Milliseconds 500

# Exercise the registered Program Files copy before endpoint association.
& $apoSmoke
if ($LASTEXITCODE -ne 0) {
    Unregister-InstalledApoBestEffort
    throw "Omniphony APO processing self-test failed before endpoint attachment: $LASTEXITCODE"
}

& $ctl attach $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    Unregister-InstalledApoBestEffort
    throw "Physical-endpoint APO attachment failed: $LASTEXITCODE"
}

& $endpointCtl set-default-name $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    Rollback-EndpointAttachment
    throw "Could not restore the physical output as Windows default: $LASTEXITCODE"
}

Restart-Service -Name AudioSrv -Force
Start-Sleep -Milliseconds 1000

& $ctl status $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    Rollback-EndpointAttachment
    throw 'APO registry state did not survive Windows Audio restart.'
}

# This is the physical Windows-audio gate that the first Current package lacked.
# It exercises the same shared-mode GetMixFormat boundary that foobar hits.
& $mixProbe $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    $probeExit = $LASTEXITCODE
    Rollback-EndpointAttachment
    throw "Physical endpoint failed the post-attach WASAPI GetMixFormat probe: $probeExit"
}

Write-Host ''
Write-Host 'OMNIPHONY_APO_INSTALL_OK 1'
Write-Host "Runtime installed at: $runtimeRoot"
Write-Host 'Current includes the primary Noire X personal output-EQ and right-ear correction profile.'
Write-Host 'The FiiO / Noire endpoint remains the Windows default.'
Write-Host 'No Omniphony playback device or virtual audio driver is required.'
Write-Host 'The physical endpoint passed the post-restart WASAPI GetMixFormat gate.'
Write-Host 'The legacy Omniphony process is not running.'
