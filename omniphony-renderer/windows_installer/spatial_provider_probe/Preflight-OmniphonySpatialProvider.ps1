param(
    [Parameter(Mandatory = $true)]
    [string]$StageManifestPath,

    [Parameter(Mandatory = $true)]
    [string]$PhysicalEndpointId,

    [string]$ReportPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-Sha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Test-PathWithin {
    param(
        [Parameter(Mandatory = $true)][string]$Child,
        [Parameter(Mandatory = $true)][string]$Parent
    )

    $childFull = [System.IO.Path]::GetFullPath($Child).TrimEnd('\')
    $parentFull = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\')
    if ($childFull.Equals($parentFull, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $true
    }
    return $childFull.StartsWith($parentFull + '\', [System.StringComparison]::OrdinalIgnoreCase)
}

function Invoke-NativeCaptured {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory = $true)][string]$Label
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Label executable is missing: $Path"
    }

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

    return [ordered]@{
        exit_code = $code
        output = $lines
    }
}

function Assert-Marker {
    param(
        [Parameter(Mandatory = $true)][object]$Result,
        [Parameter(Mandatory = $true)][string]$Marker,
        [Parameter(Mandatory = $true)][string]$Label
    )

    if (-not (@($Result.output) -contains $Marker)) {
        throw "$Label did not emit required marker: $Marker"
    }
}

if ([Environment]::Is64BitOperatingSystem -and -not [Environment]::Is64BitProcess) {
    throw 'Spatial-provider activation preflight must run from a 64-bit PowerShell process on 64-bit Windows.'
}

if ([string]::IsNullOrWhiteSpace($PhysicalEndpointId)) {
    throw 'PhysicalEndpointId must identify the exact physical render endpoint intended to receive Omniphony stereo output.'
}

if (-not (Test-Path -LiteralPath $StageManifestPath -PathType Leaf)) {
    throw "Spatial-provider stage manifest is missing: $StageManifestPath"
}

$manifestPathResolved = (Resolve-Path -LiteralPath $StageManifestPath).Path
$manifest = Get-Content -LiteralPath $manifestPathResolved -Raw | ConvertFrom-Json

if ($manifest.schema -ne 'omniphony.windows.spatial-provider-stage.v1') {
    throw "Unsupported spatial-provider stage manifest schema: $($manifest.schema)"
}
if ($manifest.state -ne 'staged-not-registered') {
    throw "Spatial-provider stage is not inert: state=$($manifest.state)"
}
if ($manifest.registry_mutated -ne $false -or $manifest.provider_selected -ne $false) {
    throw 'Spatial-provider preflight refuses a stage manifest that records registration or selection mutation.'
}
if ($manifest.exact_file_set_verified -ne $true -or $manifest.final_path_smokes_verified -ne $true) {
    throw 'Spatial-provider stage manifest does not record final immutable verification.'
}
if (-not $manifest.app_root -or -not $manifest.generation_root) {
    throw 'Spatial-provider stage manifest is missing app_root or generation_root.'
}

$appRoot = [System.IO.Path]::GetFullPath([string]$manifest.app_root)
$generationRoot = [System.IO.Path]::GetFullPath([string]$manifest.generation_root)
if (-not (Test-PathWithin -Child $generationRoot -Parent (Join-Path $appRoot 'SpatialProvider\generations'))) {
    throw "Spatial-provider generation is outside the managed generations root: $generationRoot"
}
if (-not (Test-Path -LiteralPath $generationRoot -PathType Container)) {
    throw "Spatial-provider staged generation directory is missing: $generationRoot"
}

$expected = [ordered]@{}
foreach ($property in $manifest.file_sha256.PSObject.Properties) {
    $expected[$property.Name] = ([string]$property.Value).ToLowerInvariant()
}
if ($expected.Count -eq 0) {
    throw 'Spatial-provider stage manifest contains no file hashes.'
}

$actualItems = @(Get-ChildItem -LiteralPath $generationRoot -Force)
$directories = @($actualItems | Where-Object { $_.PSIsContainer })
if ($directories.Count -ne 0) {
    $names = ($directories | ForEach-Object { $_.Name }) -join ', '
    throw "Spatial-provider staged generation contains unexpected directories: $names"
}

$actualNames = @($actualItems | ForEach-Object { $_.Name } | Sort-Object)
$expectedNames = @($expected.Keys | Sort-Object)
$diff = @(Compare-Object -ReferenceObject $expectedNames -DifferenceObject $actualNames)
if ($diff.Count -ne 0) {
    $detail = ($diff | ForEach-Object { "$($_.SideIndicator)$($_.InputObject)" }) -join ', '
    throw "Spatial-provider staged generation file set changed after staging: [$detail]"
}

foreach ($name in $expected.Keys) {
    $path = Join-Path $generationRoot $name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Spatial-provider staged file is missing: $path"
    }
    $actualHash = Get-Sha256 $path
    if ($actualHash -ne $expected[$name]) {
        throw "Spatial-provider staged file hash changed: $path expected=$($expected[$name]) actual=$actualHash"
    }
}

$providerDll = [System.IO.Path]::GetFullPath([string]$manifest.provider_dll)
$realtimeDll = [System.IO.Path]::GetFullPath([string]$manifest.realtime_dll)
$rawOutputProbe = [System.IO.Path]::GetFullPath([string]$manifest.raw_output_probe)
foreach ($ownedPath in @($providerDll, $realtimeDll, $rawOutputProbe)) {
    if (-not (Test-PathWithin -Child $ownedPath -Parent $generationRoot)) {
        throw "Spatial-provider manifest points outside its immutable generation: $ownedPath"
    }
}

$capability = Invoke-NativeCaptured `
    -Path (Join-Path $generationRoot 'OmniphonySpatialProbeSmoke.exe') `
    -Arguments @($providerDll) `
    -Label 'Final-path provider capability smoke'

$staticStream = Invoke-NativeCaptured `
    -Path (Join-Path $generationRoot 'OmniphonySpatialStaticStreamSmoke.exe') `
    -Label 'Final-path static stream lifecycle smoke'

$realtimeBridge = Invoke-NativeCaptured `
    -Path (Join-Path $generationRoot 'OmniphonySpatialRealtimeBridgeSmoke.exe') `
    -Arguments @($realtimeDll) `
    -Label 'Final-path realtime bridge smoke'
Assert-Marker -Result $realtimeBridge -Marker 'SPATIAL_COM_TO_CURRENT_OK 1' -Label 'Final-path realtime bridge smoke'
Assert-Marker -Result $realtimeBridge -Marker 'SPATIAL_FINAL_ENDPOINT_PROVEN 0' -Label 'Final-path realtime bridge smoke'

$rawOutput = Invoke-NativeCaptured `
    -Path $rawOutputProbe `
    -Arguments @($PhysicalEndpointId) `
    -Label 'Physical endpoint RAW output preflight'
Assert-Marker -Result $rawOutput -Marker 'SPATIAL_RAW_OUTPUT_PROBE_OK 1' -Label 'Physical endpoint RAW output preflight'
Assert-Marker -Result $rawOutput -Marker 'SPATIAL_RAW_OUTPUT_STREAM_INITIALIZED 0' -Label 'Physical endpoint RAW output preflight'
Assert-Marker -Result $rawOutput -Marker 'SPATIAL_RAW_OUTPUT_STREAM_STARTED 0' -Label 'Physical endpoint RAW output preflight'

$desiredSupported = @($rawOutput.output) -contains 'SPATIAL_RAW_OUTPUT_DESIRED_SUPPORTED 1'
$periodQueryOk = @($rawOutput.output) -contains 'SPATIAL_RAW_OUTPUT_PERIOD_QUERY_OK 1'
$period480Legal = @($rawOutput.output) -contains 'SPATIAL_RAW_OUTPUT_480_PERIOD_LEGAL 1'
if (-not $desiredSupported) {
    throw 'Physical endpoint does not report support for the staged stereo float32 / 48 kHz output contract.'
}
if (-not $periodQueryOk) {
    throw 'Physical endpoint did not provide the shared-engine period constraints needed for safe output planning.'
}
if (-not $period480Legal) {
    throw 'The staged 480-frame spatial quantum is not a legal shared-engine period on the selected physical endpoint.'
}

$report = [ordered]@{
    schema = 'omniphony.windows.spatial-provider-preflight.v1'
    state = 'preflight-passed-no-provider-mutation'
    generation = [string]$manifest.generation
    package_sha256 = [string]$manifest.package_sha256
    stage_manifest = $manifestPathResolved
    generation_root = $generationRoot
    physical_endpoint_id = $PhysicalEndpointId
    os_64_bit = [Environment]::Is64BitOperatingSystem
    process_64_bit = [Environment]::Is64BitProcess
    exact_file_set_verified = $true
    all_file_hashes_verified = $true
    final_path_capability_smoke_verified = $true
    final_path_static_stream_smoke_verified = $true
    final_path_realtime_bridge_smoke_verified = $true
    com_to_current_verified_registry_free = $true
    desired_stereo_output_supported = $true
    output_period_query_verified = $true
    staged_480_frame_period_legal = $true
    output_stream_initialized = $false
    output_stream_started = $false
    registry_mutated = $false
    provider_selected = $false
    preflight_utc = [DateTime]::UtcNow.ToString('o')
}

if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $ReportPath = Join-Path ([System.IO.Path]::GetDirectoryName($manifestPathResolved)) 'preflight-generation.json'
}
$reportFullPath = [System.IO.Path]::GetFullPath($ReportPath)
$reportDirectory = [System.IO.Path]::GetDirectoryName($reportFullPath)
if (-not (Test-Path -LiteralPath $reportDirectory -PathType Container)) {
    New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null
}
$tempReport = "$reportFullPath.tmp-$PID"
try {
    $report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $tempReport -Encoding UTF8
    Move-Item -LiteralPath $tempReport -Destination $reportFullPath -Force
}
finally {
    if (Test-Path -LiteralPath $tempReport -PathType Leaf) {
        Remove-Item -LiteralPath $tempReport -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "SPATIAL_PROVIDER_PREFLIGHT_OK GENERATION=$($manifest.generation)"
Write-Host "SPATIAL_PROVIDER_PREFLIGHT_ENDPOINT $PhysicalEndpointId"
Write-Host "SPATIAL_PROVIDER_PREFLIGHT_REPORT $reportFullPath"
Write-Host 'SPATIAL_PROVIDER_PREFLIGHT_COM_TO_CURRENT 1'
Write-Host 'SPATIAL_PROVIDER_PREFLIGHT_OUTPUT_CONTRACT 1'
Write-Host 'SPATIAL_PROVIDER_PREFLIGHT_OUTPUT_STREAM_INITIALIZED 0'
Write-Host 'SPATIAL_PROVIDER_PREFLIGHT_OUTPUT_STREAM_STARTED 0'
Write-Host 'SPATIAL_PROVIDER_PREFLIGHT_REGISTRY_MUTATED 0'
Write-Host 'SPATIAL_PROVIDER_PREFLIGHT_PROVIDER_SELECTED 0'
