param(
    [string]$CertificateThumbprint = '',
    [switch]$MachineCertificateStore,
    [string]$TimestampUrl = '',
    [string]$SignedPackageRoot = '',
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

function Start-OmniphonyTray {
    $tray = Join-Path $AppRoot 'support\OmniphonyTray.ps1'
    if (Test-Path -LiteralPath $tray -PathType Leaf) {
        Start-Process -FilePath "$env:WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" `
            -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden', '-File', $tray) `
            -WindowStyle Hidden
    }
}

$captureScript = Join-Path $productionRoot 'Capture-ProductionTarget.ps1'
$buildScript = Join-Path $productionRoot 'Build-ProductionApoPackages.ps1'
$finalizeSignedScript = Join-Path $productionRoot 'Finalize-SignedProductionPackages.ps1'
$readinessScript = Join-Path $productionRoot 'Test-ProductionMachineReadiness.ps1'
$installScript = Join-Path $productionRoot 'Install-ProductionApoPackages.ps1'
foreach ($required in @($captureScript, $buildScript, $finalizeSignedScript, $readinessScript, $installScript)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required production helper is missing: $required" }
}

New-Item -ItemType Directory -Force -Path $WorkRoot | Out-Null

# If this is the preparation phase, preserve the currently audible Current
# binaries before removing the development endpoint attachment and APO folder.
$stagedApo = ''
$stagedRealtime = ''
$stagedProbe = ''
$endpointCtl = ''
if ([string]::IsNullOrWhiteSpace($SignedPackageRoot)) {
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

    $stageRoot = Join-Path $WorkRoot 'runtime-stage'
    New-Item -ItemType Directory -Force -Path $stageRoot | Out-Null
    $stagedApo = Join-Path $stageRoot 'OmniphonyAPO.dll'
    $stagedRealtime = Join-Path $stageRoot 'omniphony_realtime.dll'
    $stagedProbe = Join-Path $stageRoot 'OmniphonyProductionProbe.exe'
    Copy-Item -LiteralPath $resolvedApo -Destination $stagedApo -Force
    Copy-Item -LiteralPath $resolvedRealtime -Destination $stagedRealtime -Force
    Copy-Item -LiteralPath $resolvedProbe -Destination $stagedProbe -Force
    Write-Host 'OMNIPHONY_PRODUCTION_RUNTIME_STAGED 1'
}

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

# Installation phase: only an externally signed package is eligible. Local
# Authenticode signing is useful for preparing/submitting a candidate, but is
# not silently promoted to Microsoft production trust here.
if (-not [string]::IsNullOrWhiteSpace($SignedPackageRoot)) {
    $signedRoot = (Resolve-Path -LiteralPath $SignedPackageRoot -ErrorAction Stop).Path
    & $finalizeSignedScript -PackageRoot $signedRoot -RequireMicrosoftCatalogSigner

    $readinessPath = Join-Path $WorkRoot 'readiness.json'
    & $readinessScript -PackageRoot $signedRoot -OutputPath $readinessPath | Out-Host
    if (-not (Test-Path -LiteralPath $readinessPath -PathType Leaf)) {
        throw 'Production readiness check did not produce its report.'
    }
    $readiness = Get-Content -LiteralPath $readinessPath -Raw | ConvertFrom-Json
    if (-not [bool]$readiness.RepositorySideReadyForPhysicalTest) {
        $why = @($readiness.Blockers) -join ' | '
        throw "Production readiness is blocked: $why"
    }

    & $installScript -PackageRoot $signedRoot
    Start-OmniphonyTray

    Write-Host ''
    Write-Host 'OMNIPHONY_PRODUCTION_HANDOFF_OK 1'
    Write-Host "PACKAGE_ROOT`t$signedRoot"
    Write-Host "READINESS`t$readinessPath"
    Write-Host 'TRAY_ONLY_UI 1'
    return
}

# Preparation phase: clean the dev path once, capture the exact physical target,
# and optionally build a locally signed submission candidate. No more manual
# endpoint/INF archaeology is required.
$capturePath = Join-Path $WorkRoot 'target-capture.json'
& $captureScript -EndpointCtl $endpointCtl -OutputPath $capturePath
if (-not (Test-Path -LiteralPath $capturePath -PathType Leaf)) {
    throw 'Fresh production target capture did not produce its evidence file.'
}
Write-Host 'OMNIPHONY_PRODUCTION_CAPTURE_READY 1'
Write-Host "TARGET_CAPTURE`t$capturePath"

if ([string]::IsNullOrWhiteSpace($CertificateThumbprint)) {
    Write-Host 'OMNIPHONY_DRIVER_SIGNING_IDENTITY_REQUIRED 1'
    throw 'The machine migration and clean target capture are complete. A Hardware Dev Center-compatible signing identity is the remaining external gate for preparing the driver submission.'
}

$packageRoot = Join-Path $WorkRoot 'submission-candidate'
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

Write-Host ''
Write-Host 'OMNIPHONY_PRODUCTION_SUBMISSION_CANDIDATE_READY 1'
Write-Host "PACKAGE_ROOT`t$packageRoot"
Write-Host "COMPONENT_PACKAGE`t$(Join-Path $packageRoot 'component')"
Write-Host "EXTENSION_PACKAGE`t$(Join-Path $packageRoot 'extension')"
Write-Host 'OMNIPHONY_PARTNER_CENTER_SIGNING_REQUIRED 1'
Write-Warning 'Do not install this locally signed candidate as the production result. Submit the driver packages through the appropriate Microsoft Hardware Dev Center signing/certification path, then rerun this script with -SignedPackageRoot pointing at the reassembled signed package.'
