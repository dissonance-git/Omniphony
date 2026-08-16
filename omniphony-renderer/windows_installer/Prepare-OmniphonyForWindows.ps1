param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Host', 'Endpoint')]
    [string]$Mode,

    [Parameter(Mandatory = $true)]
    [string]$SourceRoot
)

$ErrorActionPreference = 'Stop'

function Require-Replace {
    param(
        [string]$Text,
        [string]$Old,
        [string]$New,
        [string]$Label
    )
    if (-not $Text.Contains($Old)) {
        throw "Omniphony for Windows productization source drift: $Label"
    }
    return $Text.Replace($Old, $New)
}

function Write-Utf8Bom {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($true))
}

function Write-Utf16LeBom {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UnicodeEncoding]::new($false, $true))
}

if ($Mode -eq 'Host') {
    $supervisorPath = Join-Path $SourceRoot 'windows_host/src/spatial_supervisor.rs'
    $appPath = Join-Path $SourceRoot 'windows_host/src/bin/omniphony_app.rs'
    $workerPath = Join-Path $SourceRoot 'windows_host/src/music_worker_evidence.rs'

    foreach ($path in @($supervisorPath, $appPath, $workerPath)) {
        if (-not (Test-Path $path)) { throw "Missing host source: $path" }
    }

    # User-facing product identity only. Keep upstream/internal Omniphony engine
    # terminology and the legacy mutex/settings location stable during migration.
    $supervisor = Get-Content -Raw -LiteralPath $supervisorPath
    $supervisor = $supervisor.Replace('Spatial', 'Omniphony')
    $supervisor = Require-Replace $supervisor `
        'const LEGACY_AUTOSTART_VALUE: &str = "Omniphony";' `
        'const LEGACY_AUTOSTART_VALUE: &str = "Spatial";' `
        'legacy autostart value after branding migration'
    $supervisor = $supervisor.Replace('spatial.log', 'omniphony.log')

    # Personal build only: make the downstream physical endpoint explicit.
    # Windows' default render endpoint is intentionally the signed development
    # transport, so the renderer child must never infer its own destination from
    # that default. Keep this in the profile/product preparation layer rather
    # than the portable renderer or generic Windows source.
    $oldWorkerLaunch = @'
        .env("OMNIPHONY_INTERNAL_ENGINE", "1")
        .env("OMNIPHONY_PROFILE", "external")
        .stdin(Stdio::piped())
'@
    $newWorkerLaunch = @'
        .env("OMNIPHONY_INTERNAL_ENGINE", "1")
        .env("OMNIPHONY_PROFILE", "external")
        .arg("--output")
        .arg("Dan Clark Noire X")
        .stdin(Stdio::piped())
'@
    $supervisor = Require-Replace $supervisor $oldWorkerLaunch $newWorkerLaunch 'personal physical-output child launch'
    Write-Utf8Bom $supervisorPath $supervisor

    $app = Get-Content -Raw -LiteralPath $appPath
    $app = $app.Replace('Spatial is the private Windows-product shell.', 'Omniphony for Windows is the Windows product shell.')
    $app = $app.Replace('Spatial is only available on Windows', 'Omniphony for Windows is only available on Windows')
    Write-Utf8Bom $appPath $app

    # Personal development build routing. This belongs to the Windows/profile
    # layer, never the portable renderer. Prefer the user's real physical
    # endpoint explicitly, then retain the historical FiiO fallback. Never let
    # a virtual transport endpoint become its own output.
    $worker = Get-Content -Raw -LiteralPath $workerPath
    $oldVirtual = @'
fn looks_like_virtual_cable(device: &cpal::Device) -> bool {
    device
        .name()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("vb-audio")
                || lower.contains("hi-fi cable")
                || lower.contains("hifi cable")
        })
        .unwrap_or(false)
}
'@
    $newVirtual = @'
fn looks_like_virtual_cable(device: &cpal::Device) -> bool {
    device
        .name()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("vb-audio")
                || lower.contains("hi-fi cable")
                || lower.contains("hifi cable")
                || lower.contains("steam streaming speakers")
                || lower.contains("omniphony")
                || lower.contains("spatial")
        })
        .unwrap_or(false)
}
'@
    $worker = Require-Replace $worker $oldVirtual $newVirtual 'virtual output rejection law'

    $oldChoice = @'
    if let Some(device) = host
        .output_devices()?
        .find(|device| name_contains(device, "fiio"))
    {
        return Ok(device);
    }
    if let Some(device) = host.default_output_device() {
        if !looks_like_virtual_cable(&device) {
            return Ok(device);
        }
    }
    bail!("no physical output was auto-detected; expected FiiO or non-cable Windows default")
'@
    $newChoice = @'
    for preferred in ["dan clark noire x", "fiio"] {
        if let Some(device) = host
            .output_devices()?
            .find(|device| name_contains(device, preferred))
        {
            return Ok(device);
        }
    }
    if let Some(device) = host.default_output_device() {
        if !looks_like_virtual_cable(&device) {
            return Ok(device);
        }
    }
    bail!("no physical output was auto-detected; expected Dan Clark Noire X, FiiO, or a non-transport Windows default")
'@
    $worker = Require-Replace $worker $oldChoice $newChoice 'personal physical-output preference'
    Write-Utf8Bom $workerPath $worker

    Write-Host 'Prepared Omniphony for Windows host: Omniphony branding, explicit Dan Clark Noire X child output, and hard virtual-transport rejection.'
    exit 0
}

$inxPath = Join-Path $SourceRoot 'Source/Main/VirtualAudioDriver.inx'
if (-not (Test-Path $inxPath)) { throw "Missing prepared endpoint INX: $inxPath" }

# This runs after Prepare-SpatialEndpoint.ps1. Keep the unique development
# hardware ID stable for clean upgrade/removal while replacing all visible
# bootstrap branding with Omniphony.
$inx = Get-Content -Raw -LiteralPath $inxPath
$replacements = @(
    @('ProviderName = "Spatial"', 'ProviderName = "Omniphony"', 'provider'),
    @('MfgName      = "Spatial"', 'MfgName      = "Omniphony"', 'manufacturer'),
    @('MsCopyRight  = "Spatial endpoint bootstrap"', 'MsCopyRight  = "Omniphony for Windows endpoint bootstrap"', 'copyright'),
    @('VIRTUALAUDIODRIVER_SA.DeviceDesc="Spatial"', 'VIRTUALAUDIODRIVER_SA.DeviceDesc="Omniphony"', 'device description'),
    @('VirtualAudioDriver.SvcDesc="Spatial Audio Endpoint"', 'VirtualAudioDriver.SvcDesc="Omniphony Audio Endpoint"', 'service description'),
    @('VIRTUALAUDIODRIVER.WaveSpeaker.szPname="Spatial"', 'VIRTUALAUDIODRIVER.WaveSpeaker.szPname="Omniphony"', 'speaker wave name'),
    @('VIRTUALAUDIODRIVER.TopologySpeaker.szPname="Spatial"', 'VIRTUALAUDIODRIVER.TopologySpeaker.szPname="Omniphony"', 'speaker topology name'),
    @('VIRTUALAUDIODRIVER.WaveMicArray1.szPname="Spatial Internal Capture (disabled)"', 'VIRTUALAUDIODRIVER.WaveMicArray1.szPname="Omniphony Internal Capture (disabled)"', 'disabled capture wave name'),
    @('VIRTUALAUDIODRIVER.TopologyMicArray1.szPname="Spatial Internal Capture (disabled)"', 'VIRTUALAUDIODRIVER.TopologyMicArray1.szPname="Omniphony Internal Capture (disabled)"', 'disabled capture topology name'),
    @('MicArray1CustomName= "Spatial Internal Capture (disabled)"', 'MicArray1CustomName= "Omniphony Internal Capture (disabled)"', 'disabled capture custom name')
)
foreach ($entry in $replacements) {
    $inx = Require-Replace $inx $entry[0] $entry[1] $entry[2]
}
Write-Utf16LeBom $inxPath $inx

$verify = Get-Content -Raw -LiteralPath $inxPath
if ($verify -notmatch 'ROOT\\SpatialAudioEndpoint') { throw 'Legacy-stable endpoint hardware ID changed unexpectedly' }
if ($verify -notmatch 'WaveSpeaker\.szPname="Omniphony"') { throw 'Omniphony endpoint branding verification failed' }
if ($verify -match 'WaveSpeaker\.szPname="Spatial"') { throw 'Legacy Spatial endpoint branding survived productization' }

Write-Host 'Prepared Omniphony for Windows endpoint branding while preserving the development hardware ID.'
