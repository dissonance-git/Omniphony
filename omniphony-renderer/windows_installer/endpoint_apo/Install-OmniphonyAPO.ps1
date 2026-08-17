param(
    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$apo = Join-Path $here 'OmniphonyAPO.dll'
$ctl = Join-Path $here 'OmniphonyApoCtl.exe'
$endpointCtl = Join-Path $here 'OmniphonyEndpointCtl.exe'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run Install-OmniphonyAPO.ps1 from an elevated PowerShell window.'
}

foreach ($path in @($apo, $ctl, $endpointCtl)) {
    if (-not (Test-Path -LiteralPath $path)) { throw "Missing package file: $path" }
}

Get-Process -Name Omniphony -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

$register = Start-Process -FilePath $regsvr32 -ArgumentList @('/s', $apo) -Wait -PassThru
if ($register.ExitCode -ne 0) { throw "APO COM registration failed: $($register.ExitCode)" }

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
Write-Host 'Omniphony identity APO is attached to the physical endpoint.'
Write-Host 'The old Omniphony process was stopped. Do not relaunch that old build during this test.'
Write-Host 'This build intentionally sounds bit-identical; its only job is to prove native APO attachment.'
