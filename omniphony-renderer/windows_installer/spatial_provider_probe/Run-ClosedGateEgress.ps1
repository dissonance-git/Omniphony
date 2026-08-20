[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$EndpointId,

    [ValidateRange(250, 5000)]
    [int]$DurationMs = 1500,

    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Release',

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$probeRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$rendererRoot = (Resolve-Path (Join-Path $probeRoot '..\..')).Path
$repoRoot = (Resolve-Path (Join-Path $rendererRoot '..')).Path
$buildRoot = Join-Path $repoRoot 'build\spatial-provider'
$probeExe = Join-Path $buildRoot "$Configuration\OmniphonySpatialClosedGateEgressProbe.exe"
$realtimeDll = Join-Path $rendererRoot "target\release\omniphony_realtime.dll"

function Require-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $Name"
    }
}

Require-Command cmake
Require-Command cargo

if (-not $SkipBuild) {
    Write-Host 'Building realtime renderer DLL...'
    Push-Location $rendererRoot
    try {
        cargo build --release -p realtime_ffi
        if ($LASTEXITCODE -ne 0) { throw "realtime_ffi build failed: $LASTEXITCODE" }
    }
    finally {
        Pop-Location
    }

    Write-Host 'Configuring closed-gate spatial probe...'
    cmake -S $probeRoot -B $buildRoot -A x64
    if ($LASTEXITCODE -ne 0) { throw "CMake configure failed: $LASTEXITCODE" }

    Write-Host 'Building closed-gate egress probe...'
    cmake --build $buildRoot --config $Configuration --target OmniphonySpatialClosedGateEgressProbe
    if ($LASTEXITCODE -ne 0) { throw "Probe build failed: $LASTEXITCODE" }
}

if (-not (Test-Path -LiteralPath $realtimeDll -PathType Leaf)) {
    throw "Realtime DLL not found: $realtimeDll"
}
if (-not (Test-Path -LiteralPath $probeExe -PathType Leaf)) {
    throw "Closed-gate egress probe not found: $probeExe"
}

Write-Warning 'This is a short audible low-level physical-endpoint diagnostic.'
Write-Host 'Safety boundary: no Spatial Sound provider registration, selection, or public provider gate activation is performed.'
Write-Host "Endpoint: $EndpointId"
Write-Host "Duration: $DurationMs ms"

$output = @(& $probeExe $realtimeDll $EndpointId $DurationMs 2>&1 | ForEach-Object { "$_" })
$exitCode = $LASTEXITCODE
$output | ForEach-Object { Write-Host $_ }

if ($exitCode -ne 0) {
    throw "Closed-gate egress probe failed with exit code $exitCode"
}

$required = @(
    'SPATIAL_CLOSED_GATE_EGRESS_OK 1',
    'SPATIAL_CLOSED_GATE_EGRESS_COM_TO_CURRENT 1',
    'SPATIAL_CLOSED_GATE_EGRESS_CURRENT_TO_QUEUE 1',
    'SPATIAL_CLOSED_GATE_EGRESS_ENDPOINT_EVENT_CLOCK 1',
    'SPATIAL_CLOSED_GATE_EGRESS_RAW_RENDER_CLIENT 1',
    'SPATIAL_CLOSED_GATE_EGRESS_PROVIDER_REGISTERED 0',
    'SPATIAL_CLOSED_GATE_EGRESS_PROVIDER_SELECTED 0',
    'SPATIAL_CLOSED_GATE_EGRESS_PUBLIC_PROVIDER_GATE_OPENED 0'
)
foreach ($marker in $required) {
    if (-not ($output -contains $marker)) {
        throw "Missing required success/safety marker: $marker"
    }
}

$dropLine = $output | Where-Object { $_ -match '^SPATIAL_CLOSED_GATE_EGRESS_QUEUE_DROPPED_FRAMES\s+' } | Select-Object -Last 1
$underrunLine = $output | Where-Object { $_ -match '^SPATIAL_CLOSED_GATE_EGRESS_QUEUE_UNDERRUN_FRAMES\s+' } | Select-Object -Last 1
$realLine = $output | Where-Object { $_ -match '^SPATIAL_CLOSED_GATE_EGRESS_REAL_FRAMES\s+' } | Select-Object -Last 1

if (-not $dropLine -or -not $underrunLine -or -not $realLine) {
    throw 'Probe completed but did not emit all queue/frame observability counters.'
}

$droppedFrames = [uint64](($dropLine -split '\s+')[-1])
$underrunFrames = [uint64](($underrunLine -split '\s+')[-1])
$realFrames = [uint64](($realLine -split '\s+')[-1])

if ($realFrames -eq 0) { throw 'No real rendered frames reached the endpoint pump.' }
if ($droppedFrames -ne 0) { throw "Producer dropped frames: $droppedFrames" }

Write-Host ''
Write-Host 'Closed-gate physical egress PASS.'
Write-Host "Real frames: $realFrames"
Write-Host "Producer drops: $droppedFrames"
Write-Host "Measured underrun frames: $underrunFrames"
