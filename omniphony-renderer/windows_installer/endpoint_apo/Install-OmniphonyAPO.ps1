param(
    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$apo = Join-Path $here 'OmniphonyAPO.dll'
$ctl = Join-Path $here 'OmniphonyApoCtl.exe'
$endpointCtl = Join-Path $here 'OmniphonyEndpointCtl.exe'
$realtime = Join-Path $here 'omniphony_realtime.dll'
$realtimeSmoke = Join-Path $here 'OmniphonyRealtimeSmoke.exe'
$apoSmoke = Join-Path $here 'OmniphonyApoSmoke.exe'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run Install-OmniphonyAPO.ps1 from an elevated PowerShell window.'
}

foreach ($path in @($apo, $ctl, $endpointCtl, $realtime, $realtimeSmoke, $apoSmoke)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing package file: $path" }
}

# Prove both the identity oracle and Current worker can initialize before
# changing the machine's registered APO path.
& $realtimeSmoke $realtime
if ($LASTEXITCODE -ne 0) {
    throw "Omniphony realtime ABI self-test failed before installation: $LASTEXITCODE"
}

Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

$register = Start-Process -FilePath $regsvr32 -ArgumentList @('/s', $apo) -Wait -PassThru
if ($register.ExitCode -ne 0) { throw "APO COM registration failed: $($register.ExitCode)" }

# Exercise COM activation, Current-mode LockForProcess, the realtime bridge,
# fixed-latency safety lane, APOProcess and UnlockForProcess before endpoint association.
& $apoSmoke
if ($LASTEXITCODE -ne 0) {
    Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $apo) -Wait | Out-Null
    throw "Omniphony APO processing self-test failed before endpoint attachment: $LASTEXITCODE"
}

& $ctl attach $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $apo) -Wait | Out-Null
    throw "Physical-endpoint APO attachment failed: $LASTEXITCODE"
}

& $endpointCtl set-default-name $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) {
    & $ctl detach $PhysicalOutput 'FiiO' 'Noire' | Out-Null
    Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $apo) -Wait | Out-Null
    throw "Could not restore the physical output as Windows default: $LASTEXITCODE"
}

Restart-Service -Name AudioSrv -Force
Start-Sleep -Milliseconds 750

& $ctl status $PhysicalOutput 'FiiO' 'Noire'
if ($LASTEXITCODE -ne 0) { throw 'APO registry state did not survive Windows Audio restart.' }

Write-Host ''
Write-Host 'Omniphony Current is attached to the physical endpoint.'
Write-Host 'Current includes the primary Noire X personal output-EQ and right-ear correction profile.'
Write-Host 'The native endpoint remains the Windows default; no Omniphony playback device is required.'
Write-Host 'The realtime ABI and Current-mode APO lifecycle passed local self-tests before attachment.'
Write-Host 'This is now an audible listening candidate. Physical listening decides whether it is retained.'
