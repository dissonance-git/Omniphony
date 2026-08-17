param(
    [string]$PhysicalOutput = 'Dan Clark Noire X',
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
$ctl = Join-Path $supportRoot 'OmniphonyApoCtl.exe'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run the Omniphony uninstaller from an elevated context.'
}

Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

if (Test-Path -LiteralPath $ctl) {
    & $ctl detach $PhysicalOutput 'FiiO' 'Noire'
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 3) {
        throw "Physical-endpoint APO detach failed: $LASTEXITCODE"
    }
}

$service = Get-Service -Name AudioSrv -ErrorAction Stop
if ($service.Status -ne 'Stopped') {
    Stop-Service -Name AudioSrv -Force
}
try {
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
}
finally {
    $service = Get-Service -Name AudioSrv -ErrorAction Stop
    if ($service.Status -ne 'Running') {
        Start-Service -Name AudioSrv
    }
}
Start-Sleep -Milliseconds 500

Write-Host 'Omniphony APO removed. No other endpoint effect was changed.'
