param(
    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$apo = Join-Path $here 'OmniphonyAPO.dll'
$ctl = Join-Path $here 'OmniphonyApoCtl.exe'
$regsvr32 = "$env:WINDIR\System32\regsvr32.exe"

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run Uninstall-OmniphonyAPO.ps1 from an elevated PowerShell window.'
}

if (Test-Path -LiteralPath $ctl) {
    & $ctl detach $PhysicalOutput 'FiiO' 'Noire'
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 3) {
        throw "Physical-endpoint APO detach failed: $LASTEXITCODE"
    }
}

Restart-Service -Name AudioSrv -Force
Start-Sleep -Milliseconds 500

if (Test-Path -LiteralPath $apo) {
    $unregister = Start-Process -FilePath $regsvr32 -ArgumentList @('/u', '/s', $apo) -Wait -PassThru
    if ($unregister.ExitCode -ne 0) { throw "APO COM unregistration failed: $($unregister.ExitCode)" }
}

Write-Host 'Omniphony endpoint APO removed. No other endpoint effect was changed.'
