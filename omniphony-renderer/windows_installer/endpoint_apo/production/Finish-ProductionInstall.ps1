param(
    [string]$CertificateThumbprint = '',
    [switch]$MachineCertificateStore,
    [string]$TimestampUrl = '',
    [string]$AppRoot = '',
    [string]$ApoDll = '',
    [string]$RealtimeDll = '',
    [string]$ProductionProbe = '',
    [string]$WorkRoot = ''
)

$ErrorActionPreference = 'Stop'
$productionRoot = $PSScriptRoot

if ([string]::IsNullOrWhiteSpace($AppRoot)) {
    $AppRoot = Join-Path $env:ProgramFiles 'Omniphony'
}
if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    $WorkRoot = Join-Path $env:ProgramData 'Omniphony\production-handoff'
}
$WorkRoot = [IO.Path]::GetFullPath($WorkRoot)

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object System.Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run Finish-ProductionInstall.ps1 from an elevated PowerShell session.'
}

function Resolve-FirstFile([string]$Explicit, [string[]]$Candidates, [string]$Label) {
    if (-not [string]::IsNullOrWhiteSpace($Explicit)) {
        if (-not (Test-Path -LiteralPath $Explicit -PathType Leaf)) { throw "$Label was not found: $Explicit" }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    foreach ($candidate in $Candidates) {
        if (-not [string]::IsNullOrWhiteSpace($candidate) -and (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    throw "$Label was not found. Build or download the current Windows endpoint APO artifact, or pass its path explicitly."
}

function Get-AudioDgBypassValue {
    $path = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio'
    try {
        $item = Get-ItemProperty -LiteralPath $path -ErrorAction Stop
        if ($item.PSObject.Properties.Name -contains 'DisableProtectedAudioDG') {
            return [int]$item.DisableProtectedAudioDG
        }
    } catch { }
    return $null
}

$endpointCtl = Resolve-FirstFile '' @(
    (Join-Path $AppRoot 'support\OmniphonyEndpointCtl.exe'),
    (Join-Path $productionRoot '..\OmniphonyEndpointCtl.exe'),
    (Join-Path $productionRoot '..\..\build\endpoint_ctl\OmniphonyEndpointCtl.exe')
) 'OmniphonyEndpointCtl.exe'

$resolvedApo = Resolve-FirstFile $ApoDll @(
    (Join-Path $AppRoot 'APO\OmniphonyAPO.dll'),
    (Join-Path $productionRoot '..\OmniphonyAPO.dll'),
    (Join-Path $productionRoot '..\..\..\..\build\endpoint_apo\Release\OmniphonyAPO.dll')
) 'OmniphonyAPO.dll'
$resolvedRealtime = Resolve-FirstFile $RealtimeDll @(
    (Join-Path $AppRoot 'APO\omniphony_realtime.dll'),
    (Join-Path $productionRoot '..\omniphony_realtime.dll'),
    (Join-Path $productionRoot '..\..\..\target\release\omniphony_realtime.dll')
) 'omniphony_realtime.dll'
$resolvedProbe = Resolve-FirstFile $ProductionProbe @(
    (Join-Path $AppRoot 'support\OmniphonyProductionProbe.exe'),
    (Join-Path $productionRoot '..\OmniphonyProductionProbe.exe'),
    (Join-Path $productionRoot '..\..\..\..\build\endpoint_apo\Release\OmniphonyProductionProbe.exe')
) 'OmniphonyProductionProbe.exe'

New-Item -ItemType Directory -Force -Path $WorkRoot | Out-Null
$stageRoot = Join-Path $WorkRoot 'runtime-stage'
New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null
$stagedApo = Join-Path $stageRoot 'OmniphonyAPO.dll'
$stagedRealtime = Join-Path $stageRoot 'omniphony_realtime.dll'
$stagedProbe = Join-Path $stageRoot 'OmniphonyProductionProbe.exe'
Copy-Item -LiteralPath $resolvedApo -Destination $stagedApo -Force
Copy-Item -LiteralPath $resolvedRealtime -Destination $stagedRealtime -Force
Copy-Item -LiteralPath $resolvedProbe -Destination $stagedProbe -Force
Write-Host 'OMNIPHONY_PRODUCTION_RUNTIME_STAGED 1'

$devUninstaller = Join-Path $AppRoot 'support\Uninstall-OmniphonyAPO.ps1'
$devBackup = Join-Path $env:ProgramData 'Omniphony\endpoint-backup.json'
$devStatePresent = (Test-Path -LiteralPath $devBackup -PathType Leaf) -or ((Get-AudioDgBypassValue) -eq 1)
if ($devStatePresent) {
    if (-not (Test-Path -LiteralPath $devUninstaller -PathType Leaf)) {
        throw 'Legacy development APO state is present but its uninstaller is missing. Refusing manual registry cleanup.'
    }
    Write-Host 'OMNIPHONY_DEVELOPMENT_APO_REMOVAL_BEGIN 1'
    & $devUninstaller -AppRoot $AppRoot
}

if ((Get-AudioDgBypassValue) -eq 1) {
    throw 'DisableProtectedAudioDG=1 remains active after development cleanup. Production deployment is blocked.'
}
Write-Host 'OMNIPHONY_PROTECTED_AUDIODG_READY 1'

$captureScript = Join-Path $productionRoot 'Capture-ProductionTarget.ps1'
$buildScript = Join-Path $productionRoot 'Build-ProductionApoPackages.ps1'
$readinessScript = Join-Path $productionRoot 'Test-ProductionMachineReadiness.ps1'
$installScript = Join-Path $productionRoot 'Install-ProductionApoPackages.ps1'
foreach ($required in @($captureScript, $buildScript, $readinessScript, $installScript)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required production helper is missing: $required" }
}

$capturePath = Join-Path $WorkRoot 'target-capture.json'
& $captureScript -EndpointCtl $endpointCtl -OutputPath $capturePath
if (-not (Test-Path -LiteralPath $capturePath -PathType Leaf)) {
    throw 'Fresh production target capture did not produce its evidence file.'
}

if ([string]::IsNullOrWhiteSpace($CertificateThumbprint)) {
    Write-Host 'OMNIPHONY_PRODUCTION_CAPTURE_READY 1'
    Write-Host "TARGET_CAPTURE`t$capturePath"
    Write-Host 'OMNIPHONY_MICROSOFT_DRIVER_SIGNING_REQUIRED 1'
    throw 'The machine migration and target capture are complete. A Windows driver-signing identity is the remaining external gate before protected AudioDG installation; pass -CertificateThumbprint after that identity is available.'
}

$packageRoot = Join-Path $WorkRoot 'packages'
$buildArgs = @(
    '-CaptureJson', $capturePath,
    '-ApoDll', $stagedApo,
    '-RealtimeDll', $stagedRealtime,
    '-ProductionProbe', $stagedProbe,
    '-OutputRoot', $packageRoot,
    '-CertificateThumbprint', $CertificateThumbprint
)
if ($MachineCertificateStore) { $buildArgs += '-MachineCertificateStore' }
if (-not [string]::IsNullOrWhiteSpace($TimestampUrl)) {
    $buildArgs += @('-TimestampUrl', $TimestampUrl)
}
& $buildScript @buildArgs

$readinessPath = Join-Path $WorkRoot 'readiness.json'
& $readinessScript -PackageRoot $packageRoot -OutputPath $readinessPath | Out-Host
if (-not (Test-Path -LiteralPath $readinessPath -PathType Leaf)) {
    throw 'Production readiness check did not produce its report.'
}
$readiness = Get-Content -LiteralPath $readinessPath -Raw | ConvertFrom-Json
if (-not [bool]$readiness.RepositorySideReadyForPhysicalTest) {
    $why = @($readiness.Blockers) -join ' | '
    throw "Production readiness is blocked: $why"
}

& $installScript -PackageRoot $packageRoot

$tray = Join-Path $AppRoot 'support\OmniphonyTray.ps1'
if (Test-Path -LiteralPath $tray -PathType Leaf) {
    Start-Process -FilePath "$env:WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden', '-File', $tray) `
        -WindowStyle Hidden
}

Write-Host ''
Write-Host 'OMNIPHONY_PRODUCTION_HANDOFF_OK 1'
Write-Host "PACKAGE_ROOT`t$packageRoot"
Write-Host "READINESS`t$readinessPath"
Write-Host 'TRAY_ONLY_UI 1'
