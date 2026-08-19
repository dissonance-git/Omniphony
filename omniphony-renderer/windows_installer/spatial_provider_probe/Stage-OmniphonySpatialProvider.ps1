param(
    [Parameter(Mandatory = $true)]
    [string]$PackageRoot,

    [string]$AppRoot = (Join-Path $env:ProgramFiles 'Omniphony')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory = $true)][string]$Label
    )

    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $lines = @(& $Path @Arguments 2>&1 | ForEach-Object { "$_" })
        $code = $LASTEXITCODE
        if ($null -eq $code) { $code = 0 }
    }
    finally {
        $ErrorActionPreference = $previous
    }

    $lines | ForEach-Object { Write-Host $_ }
    if ($code -ne 0) {
        throw "$Label failed with exit code $code."
    }
}

function Get-RequiredFile {
    param([string]$Root, [string]$Name)
    $path = Join-Path $Root $Name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing spatial-provider package file: $path"
    }
    return (Resolve-Path -LiteralPath $path).Path
}

function Get-Sha256 {
    param([string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-Hash {
    param([string]$Path, [string]$Expected)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Spatial-provider immutable generation is incomplete: $Path"
    }
    $actual = Get-Sha256 $Path
    if ($actual -ne $Expected) {
        throw "Spatial-provider staged file hash mismatch: $Path expected=$Expected actual=$actual"
    }
}

$packageRootResolved = (Resolve-Path -LiteralPath $PackageRoot).Path
$AppRoot = [System.IO.Path]::GetFullPath($AppRoot)

$files = @(
    @{ Source = (Get-RequiredFile $packageRootResolved 'OmniphonySpatialProbe.dll'); Name = 'OmniphonySpatialProbe.dll' },
    @{ Source = (Get-RequiredFile $packageRootResolved 'omniphony_realtime.dll'); Name = 'omniphony_realtime.dll' },
    @{ Source = (Get-RequiredFile $packageRootResolved 'OmniphonySpatialProbeCtl.exe'); Name = 'OmniphonySpatialProbeCtl.exe' },
    @{ Source = (Get-RequiredFile $packageRootResolved 'OmniphonySpatialProbeSmoke.exe'); Name = 'OmniphonySpatialProbeSmoke.exe' },
    @{ Source = (Get-RequiredFile $packageRootResolved 'OmniphonySpatialStaticStreamSmoke.exe'); Name = 'OmniphonySpatialStaticStreamSmoke.exe' },
    @{ Source = (Get-RequiredFile $packageRootResolved 'OmniphonySpatialRealtimeBridgeSmoke.exe'); Name = 'OmniphonySpatialRealtimeBridgeSmoke.exe' },
    @{ Source = (Get-RequiredFile $packageRootResolved 'CaptureSpatialProviderState.ps1'); Name = 'CaptureSpatialProviderState.ps1' }
)

foreach ($file in $files) {
    $file.Hash = Get-Sha256 $file.Source
}

$identity = ($files | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Hash)" }) -join "`n"
$identityBytes = [System.Text.Encoding]::UTF8.GetBytes($identity)
$sha256 = [System.Security.Cryptography.SHA256]::Create()
try {
    $packageDigestBytes = $sha256.ComputeHash($identityBytes)
}
finally {
    $sha256.Dispose()
}
$packageDigest = [System.BitConverter]::ToString($packageDigestBytes).Replace('-', '').ToLowerInvariant()
$generation = $packageDigest.Substring(0, 24)

$providerEntry = $files | Where-Object { $_.Name -eq 'OmniphonySpatialProbe.dll' } | Select-Object -First 1
$runtimeEntry = $files | Where-Object { $_.Name -eq 'omniphony_realtime.dll' } | Select-Object -First 1
$providerHash = $providerEntry.Hash
$runtimeHash = $runtimeEntry.Hash

$spatialRoot = Join-Path $AppRoot 'SpatialProvider'
$generationsRoot = Join-Path $spatialRoot 'generations'
$generationRoot = Join-Path $generationsRoot $generation
$stagingRoot = Join-Path $generationsRoot ('.{0}.staging-{1}' -f $generation, $PID)
$manifestPath = Join-Path $spatialRoot 'staged-generation.json'

New-Item -ItemType Directory -Force -Path $generationsRoot | Out-Null

if (Test-Path -LiteralPath $generationRoot -PathType Container) {
    foreach ($file in $files) {
        Assert-Hash (Join-Path $generationRoot $file.Name) $file.Hash
    }
    Write-Host "SPATIAL_PROVIDER_GENERATION_REUSED $generationRoot"
}
else {
    if (Test-Path -LiteralPath $stagingRoot) {
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stagingRoot | Out-Null

    try {
        foreach ($file in $files) {
            Copy-Item -LiteralPath $file.Source -Destination (Join-Path $stagingRoot $file.Name) -Force
        }
        foreach ($file in $files) {
            Assert-Hash (Join-Path $stagingRoot $file.Name) $file.Hash
        }

        Invoke-NativeChecked -Path (Join-Path $stagingRoot 'OmniphonySpatialProbeSmoke.exe') -Arguments @((Join-Path $stagingRoot 'OmniphonySpatialProbe.dll')) -Label 'Spatial provider capability smoke'
        Invoke-NativeChecked -Path (Join-Path $stagingRoot 'OmniphonySpatialStaticStreamSmoke.exe') -Label 'Spatial static stream lifecycle smoke'
        Invoke-NativeChecked -Path (Join-Path $stagingRoot 'OmniphonySpatialRealtimeBridgeSmoke.exe') -Arguments @((Join-Path $stagingRoot 'omniphony_realtime.dll')) -Label 'Spatial realtime bridge smoke'

        Move-Item -LiteralPath $stagingRoot -Destination $generationRoot
        Write-Host "SPATIAL_PROVIDER_GENERATION_STAGED $generationRoot"
    }
    catch {
        if (Test-Path -LiteralPath $stagingRoot) {
            Remove-Item -LiteralPath $stagingRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
        throw
    }
}

# Re-run package-coupling smokes from the immutable final path. This catches
# path-sensitive loading mistakes before a later transaction is allowed to
# point Windows at this generation.
Invoke-NativeChecked -Path (Join-Path $generationRoot 'OmniphonySpatialProbeSmoke.exe') -Arguments @((Join-Path $generationRoot 'OmniphonySpatialProbe.dll')) -Label 'Final-path provider capability smoke'
Invoke-NativeChecked -Path (Join-Path $generationRoot 'OmniphonySpatialRealtimeBridgeSmoke.exe') -Arguments @((Join-Path $generationRoot 'omniphony_realtime.dll')) -Label 'Final-path realtime bridge smoke'

$fileHashes = [ordered]@{}
foreach ($file in ($files | Sort-Object Name)) {
    $fileHashes[$file.Name] = $file.Hash
}

$manifest = [ordered]@{
    schema = 'omniphony.windows.spatial-provider-stage.v1'
    state = 'staged-not-registered'
    generation = $generation
    package_sha256 = $packageDigest
    generation_root = $generationRoot
    provider_dll = (Join-Path $generationRoot 'OmniphonySpatialProbe.dll')
    provider_sha256 = $providerHash
    realtime_dll = (Join-Path $generationRoot 'omniphony_realtime.dll')
    realtime_sha256 = $runtimeHash
    file_sha256 = $fileHashes
    staged_utc = [DateTime]::UtcNow.ToString('o')
    registry_mutated = $false
    provider_selected = $false
}

New-Item -ItemType Directory -Force -Path $spatialRoot | Out-Null
$tempManifest = "$manifestPath.tmp-$PID"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $tempManifest -Encoding UTF8
Move-Item -LiteralPath $tempManifest -Destination $manifestPath -Force

Write-Host "SPATIAL_PROVIDER_STAGE_OK GENERATION=$generation PACKAGE_SHA256=$packageDigest"
Write-Host "SPATIAL_PROVIDER_STAGE_MANIFEST $manifestPath"
Write-Host 'SPATIAL_PROVIDER_REGISTRY_MUTATED 0'
Write-Host 'SPATIAL_PROVIDER_SELECTED 0'
