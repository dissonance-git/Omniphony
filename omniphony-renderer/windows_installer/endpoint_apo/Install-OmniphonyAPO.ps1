param(
    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$packageApo = Join-Path $here 'OmniphonyAPO.dll'
$ctl = Join-Path $here 'OmniphonyApoCtl.exe'
$endpointCtl = Join-Path $here 'OmniphonyEndpointCtl.exe'
$packageRealtime = Join-Path $here 'omniphony_realtime.dll'
$realtimeSmoke = Join-Path $here 'OmniphonyRealtimeSmoke.exe'
$apoSmoke = Join-Path $here 'OmniphonyApoSmoke.exe'
$mixProbe = Join-Path $here 'OmniphonyMixProbe.exe'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"
$runtimeRoot = Join-Path $env:ProgramFiles 'Omniphony\APO'
$installedApo = Join-Path $runtimeRoot 'OmniphonyAPO.dll'
$installedRealtime = Join-Path $runtimeRoot 'omniphony_realtime.dll'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run Install-OmniphonyAPO.ps1 from an elevated PowerShell window.'
}

foreach ($path in @($packageApo, $ctl, $endpointCtl, $packageRealtime, $realtimeSmoke, $apoSmoke, $mixProbe)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing package file: $path" }
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
    Write-Warning 'Rolling back Omniphony endpoint attachment.'
    try {
        & $ctl detach $PhysicalOutput 'FiiO' 'Noire' | Out-Host
    } catch {
        Write-Warning "Endpoint detach rollback failed: $($_.Exception.Message)"
    }
    try {
        Restart-Service -Name AudioSrv -Force
        Start-Sleep -Milliseconds 750
    } catch {
        Write-Warning "AudioSrv rollback restart failed: $($_.Exception.Message)"
    }
    Unregister-InstalledApoBestEffort
}

& $realtimeSmoke $packageRealtime
if ($LASTEXITCODE -ne 0) {
    throw "Omniphony realtime ABI self-test failed before installation: $LASTEXITCODE"
}

Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

# AudioDG hosts endpoint APOs out of process. Never register the development APO
# from Downloads/Desktop/a temporary extraction directory. Stage the runtime in
# Program Files so the audio service has a stable, machine-readable path and so
# future updates do not depend on the ZIP remaining in place.
Stop-Service -Name AudioSrv -Force
New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
Copy-Item -LiteralPath $packageApo -Destination $installedApo -Force
Copy-Item -LiteralPath $packageRealtime -Destination $installedRealtime -Force

$register = Start-Process -FilePath $regsvr32 -ArgumentList @('/s', $installedApo) -Wait -PassThru
if ($register.ExitCode -ne 0) {
    Start-Service -Name AudioSrv
    throw "APO COM registration failed from installed runtime path: $($register.ExitCode)"
}

Start-Service -Name AudioSrv
Start-Sleep -Milliseconds 500

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

# This is the real-world gate the earlier bootstrap was missing. It asks the
# physical endpoint for the shared-mode mix format after AudioSrv has loaded the
# installed APO. Failure rolls the EFX association back instead of leaving the
# endpoint unusable.
& $mixProbe $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    $probeExit = $LASTEXITCODE
    Rollback-EndpointAttachment
    throw "Physical endpoint failed the post-attach WASAPI GetMixFormat probe: $probeExit"
}

Write-Host ''
Write-Host 'Omniphony Current is attached to the physical endpoint.'
Write-Host "Runtime installed at: $runtimeRoot"
Write-Host 'Current includes the primary Noire X personal output-EQ and right-ear correction profile.'
Write-Host 'The native endpoint remains the Windows default; no Omniphony playback device is required.'
Write-Host 'The physical endpoint passed a post-restart WASAPI GetMixFormat probe.'
Write-Host 'This is now an audible listening candidate. Physical listening decides whether it is retained.'
