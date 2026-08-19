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

function Assert-Hash {
    param([string]$Path, [string]$Expected)
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) {
        throw "Spatial-provider staged file hash mismatch: $Path expected=$Expected actual=$actual"
    }
}

$packageRootResolved = (Resolve-Path -LiteralPath $PackageRoot).Path
$providerSource = Get-RequiredFile $packageRootResolved 'OmniphonySpatialProbe.dll'
$runtimeSource = Get-RequiredFile $packageRootResolved 'omniphony_realtime.dll'
$ctlSource = Get-RequiredFile $packageRootResolved 'OmniphonySpatialProbeCtl.exe'
$providerSmokeSource = Get-RequiredFile $packageRootResolved 'OmniphonySpatialProbeSmoke.exe'
$streamSmokeSource = Get-RequiredFile $packageRootResolved 'OmniphonySpatialStaticStreamSmoke.exe'
$bridgeSmokeSource = Get-RequiredFile $packageRootResolved 'OmniphonySpatialRealtimeBridgeSmoke.exe'
$captureSource = Get-RequiredFile $packageRootResolved 'CaptureSpatialProviderState.ps1'

$providerHash = (Get-FileHash -LiteralPath $providerSource -Algorithm SHA256).Hash.ToLowerInvariant()
$runtimeHash = (Get-FileHash -LiteralPath $runtimeSource -Algorithm SHA256).Hash.ToLowerInvariant()
$generation = '{0}-{1}' -f $providerHash.Substring(0, 12), $runtimeHash.Substring(0, 12)

$spatialRoot = Join-Path $AppRoot 'SpatialProvider'
$generationsRoot = Join-Path $spatialRoot 'generations'
$generationRoot = Join-Path $generationsRoot $generation
$stagingRoot = Join-Path $generationsRoot ('.{0}.staging-{1}' -f $generation, $PID)
$manifestPath = Join-Path $spatialRoot 'staged-generation.json'

New-Item -ItemType Directory -Force -Path $generationsRoot | Out-Null

$files = @(
    @{ Source = $providerSource; Name = 'OmniphonySpatialProbe.dll' },
    @{ Source = $runtimeSource; Name = 'omniphony_realtime.dll' },
    @{ Source = $ctlSource; Name = 'OmniphonySpatialProbeCtl.exe' },
    @{ Source = $providerSmokeSource; Name = 'OmniphonySpatialProbeSmoke.exe' },
    @{ Source = $streamSmokeSource; Name = 'OmniphonySpatialStaticStreamSmoke.exe' },
    @{ Source = $bridgeSmokeSource; Name = 'OmniphonySpatialRealtimeBridgeSmoke.exe' },
    @{ Source = $captureSource; Name = 'CaptureSpatialProviderState.ps1' }
)

if (Test-Path -LiteralPath $generationRoot -PathType Container) {
    Assert-Hash (Join-Path $generationRoot 'OmniphonySpatialProbe.dll') $providerHash
    Assert-Hash (Join-Path $generationRoot 'omniphony_realtime.dll') $runtimeHash
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

        Assert-Hash (Join-Path $stagingRoot 'OmniphonySpatialProbe.dll') $providerHash
        Assert-Hash (Join-Path $stagingRoot 'omniphony_realtime.dll') $runtimeHash

        Invoke-NativeChecked \
            -Path (Join-Path $stagingRoot 'OmniphonySpatialProbeSmoke.exe') \
            -Arguments @((Join-Path $stagingRoot 'OmniphonySpatialProbe.dll')) \
            -Label 'Spatial provider capability smoke'

        Invoke-NativeChecked \
            -Path (Join-Path $stagingRoot 'OmniphonySpatialStaticStreamSmoke.exe') \
            -Label 'Spatial static stream lifecycle smoke'

        Invoke-NativeChecked \
            -Path (Join-Path $stagingRoot 'OmniphonySpatialRealtimeBridgeSmoke.exe') \
            -Arguments @((Join-Path $stagingRoot 'omniphony_realtime.dll')) \
            -Label 'Spatial realtime bridge smoke'

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

# Re-run the two package-coupling smokes from the immutable final path. This
# catches path-sensitive loading mistakes before a later transaction is allowed
# to point Windows at this generation.
Invoke-NativeChecked \
    -Path (Join-Path $generationRoot 'OmniphonySpatialProbeSmoke.exe') \
    -Arguments @((Join-Path $generationRoot 'OmniphonySpatialProbe.dll')) \
    -Label 'Final-path provider capability smoke'

Invoke-NativeChecked \
    -Path (Join-Path $generationRoot 'OmniphonySpatialRealtimeBridgeSmoke.exe') \
    -Arguments @((Join-Path $generationRoot 'omniphony_realtime.dll')) \
    -Label 'Final-path realtime bridge smoke'

$manifest = [ordered]@{
    schema = 'omniphony.windows.spatial-provider-stage.v1'
    state = 'staged-not-registered'
    generation = $generation
    generation_root = $generationRoot
    provider_dll = (Join-Path $generationRoot 'OmniphonySpatialProbe.dll')
    provider_sha256 = $providerHash
    realtime_dll = (Join-Path $generationRoot 'omniphony_realtime.dll')
    realtime_sha256 = $runtimeHash
    staged_utc = [DateTime]::UtcNow.ToString('o')
    registry_mutated = $false
    provider_selected = $false
}

New-Item -ItemType Directory -Force -Path $spatialRoot | Out-Null
$tempManifest = "$manifestPath.tmp-$PID"
$manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $tempManifest -Encoding UTF8
Move-Item -LiteralPath $tempManifest -Destination $manifestPath -Force

Write-Host "SPATIAL_PROVIDER_STAGE_OK GENERATION=$generation"
Write-Host "SPATIAL_PROVIDER_STAGE_MANIFEST $manifestPath"
Write-Host 'SPATIAL_PROVIDER_REGISTRY_MUTATED 0'
Write-Host 'SPATIAL_PROVIDER_SELECTED 0'
