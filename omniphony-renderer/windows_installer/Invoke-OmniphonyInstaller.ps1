param(
    [ValidateSet('Install', 'Uninstall', 'Validate')]
    [string]$Action = 'Install',

    [Parameter(Mandatory = $true)]
    [string]$AppRoot,

    [string]$PhysicalOutput = 'Dan Clark Noire X'
)

$ErrorActionPreference = 'Stop'

$installer = Join-Path $PSScriptRoot 'Install-OmniphonyForWindows.ps1'
if (-not (Test-Path -LiteralPath $installer)) {
    throw "Omniphony installer control plane is missing: $installer"
}

# The control plane deliberately remains compatible with stock Windows
# PowerShell 5.1. Windows audio COM and driver-install work now lives in the
# compiled OmniphonyEndpointCtl helper, so no runtime C# source rewriting or
# Add-Type compiler dependency is needed here.
$windowsPowerShell = Join-Path $env:WINDIR 'System32\WindowsPowerShell\v1.0\powershell.exe'
if (-not (Test-Path -LiteralPath $windowsPowerShell)) {
    throw "Windows PowerShell 5.1 was not found: $windowsPowerShell"
}

$invokeArgs = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', $installer,
    '-Action', $Action,
    '-AppRoot', $AppRoot,
    '-PhysicalOutput', $PhysicalOutput
)

& $windowsPowerShell @invokeArgs
$code = $LASTEXITCODE
if ($null -eq $code) { $code = 1 }
exit $code
