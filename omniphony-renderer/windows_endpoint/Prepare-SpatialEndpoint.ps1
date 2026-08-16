param(
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
        throw "Pinned endpoint source no longer matches expected text: $Label"
    }
    return $Text.Replace($Old, $New)
}

function Write-Utf8Bom {
    param([string]$Path, [string]$Text)
    [System.IO.File]::WriteAllText(
        $Path,
        $Text,
        [System.Text.UTF8Encoding]::new($true)
    )
}

function Write-Utf16LeBom {
    param([string]$Path, [string]$Text)
    # Current WDK InfVerif accepts ANSI or UTF-16 LE for INF input. Keep the
    # generated INX explicitly UTF-16 LE with BOM so StampInf preserves a
    # verifier-supported encoding. C/C++ patches remain ordinary UTF-8 below.
    [System.IO.File]::WriteAllText(
        $Path,
        $Text,
        [System.Text.UnicodeEncoding]::new($false, $true)
    )
}

$inxPath = Join-Path $SourceRoot 'Source/Main/VirtualAudioDriver.inx'
$pairPath = Join-Path $SourceRoot 'Source/Filters/minipairs.h'
$adapterPath = Join-Path $SourceRoot 'Source/Main/adapter.cpp'

if (-not (Test-Path $inxPath)) { throw "Missing pinned endpoint INF source: $inxPath" }
if (-not (Test-Path $pairPath)) { throw "Missing pinned endpoint pair source: $pairPath" }
if (-not (Test-Path $adapterPath)) { throw "Missing pinned endpoint adapter source: $adapterPath" }

# P0 is intentionally a Windows-only transport shell. The portable renderer is
# not copied into the driver and no DSP is performed here. The virtual speaker
# merely gives Windows a real, silent render destination so the existing
# endpoint-independent process-loopback host can hear every external app while
# its own binaural K7 output remains excluded.
$inx = Get-Content -Raw -LiteralPath $inxPath
$inx = Require-Replace $inx '%VIRTUALAUDIODRIVER_SA.DeviceDesc%=VIRTUALAUDIODRIVER_SA, ROOT\VirtualAudioDriver' '%VIRTUALAUDIODRIVER_SA.DeviceDesc%=VIRTUALAUDIODRIVER_SA, ROOT\SpatialAudioEndpoint' 'root hardware id'
$inx = Require-Replace $inx 'ProviderName = "MikeTheTech"' 'ProviderName = "Spatial"' 'provider name'
$inx = Require-Replace $inx 'MfgName      = "MikeTheTech"' 'MfgName      = "Spatial"' 'manufacturer name'
$inx = Require-Replace $inx 'MsCopyRight  = "MikeTheTech"' 'MsCopyRight  = "Spatial endpoint bootstrap"' 'copyright label'
$inx = Require-Replace $inx 'VIRTUALAUDIODRIVER_SA.DeviceDesc="Virtual Audio Driver by MTT"' 'VIRTUALAUDIODRIVER_SA.DeviceDesc="Spatial"' 'device description'
$inx = Require-Replace $inx 'VirtualAudioDriver.SvcDesc="Virtual Audio Driver by MTT"' 'VirtualAudioDriver.SvcDesc="Spatial Audio Endpoint"' 'service description'
$inx = Require-Replace $inx 'VIRTUALAUDIODRIVER.WaveSpeaker.szPname="Virtual Audio Driver by MTT"' 'VIRTUALAUDIODRIVER.WaveSpeaker.szPname="Spatial"' 'speaker wave friendly name'
$inx = Require-Replace $inx 'VIRTUALAUDIODRIVER.TopologySpeaker.szPname="Virtual Audio Driver by MTT"' 'VIRTUALAUDIODRIVER.TopologySpeaker.szPname="Spatial"' 'speaker topology friendly name'
$inx = Require-Replace $inx 'VIRTUALAUDIODRIVER.WaveMicArray1.szPname="Virtual Mic Driver by MTT"' 'VIRTUALAUDIODRIVER.WaveMicArray1.szPname="Spatial Internal Capture (disabled)"' 'capture wave friendly name'
$inx = Require-Replace $inx 'VIRTUALAUDIODRIVER.TopologyMicArray1.szPname="Virtual Mic Driver by MTT"' 'VIRTUALAUDIODRIVER.TopologyMicArray1.szPname="Spatial Internal Capture (disabled)"' 'capture topology friendly name'
$inx = Require-Replace $inx 'MicArray1CustomName= "Virtual Mic Driver by MTT"' 'MicArray1CustomName= "Spatial Internal Capture (disabled)"' 'capture custom name'

# Do not expose the sample microphone as a Windows endpoint. Keeping the
# upstream capture implementation compiled minimizes the source delta, while
# removing its AddInterface registrations plus the capture miniport list keeps
# P0 user-facing behavior to one render endpoint only.
$micInterfaces = @(
    'AddInterface=%KSCATEGORY_AUDIO%,    %KSNAME_WaveMicArray1%, VIRTUALAUDIODRIVER.I.WaveMicArray1',
    'AddInterface=%KSCATEGORY_REALTIME%, %KSNAME_WaveMicArray1%, VIRTUALAUDIODRIVER.I.WaveMicArray1',
    'AddInterface=%KSCATEGORY_CAPTURE%,  %KSNAME_WaveMicArray1%, VIRTUALAUDIODRIVER.I.WaveMicArray1',
    'AddInterface=%KSCATEGORY_AUDIO%,    %KSNAME_TopologyMicArray1%, VIRTUALAUDIODRIVER.I.TopologyMicArray1',
    'AddInterface=%KSCATEGORY_TOPOLOGY%, %KSNAME_TopologyMicArray1%, VIRTUALAUDIODRIVER.I.TopologyMicArray1'
)
foreach ($line in $micInterfaces) {
    $inx = Require-Replace $inx $line ('; Spatial P0 disabled capture: ' + $line) ('capture interface ' + $line)
}
Write-Utf16LeBom $inxPath $inx

$pairs = Get-Content -Raw -LiteralPath $pairPath
$oldCaptureArray = @'
static
PENDPOINT_MINIPAIR  g_CaptureEndpoints[] =
{
    &MicArray1Miniports,
};

#define g_cCaptureEndpoints (SIZEOF_ARRAY(g_CaptureEndpoints))
'@
$newCaptureArray = @'
// Spatial P0 exposes only the silent render endpoint. The sample microphone
// remains compiled as upstream provenance but is not instantiated.
static PENDPOINT_MINIPAIR* g_CaptureEndpoints = NULL;
#define g_cCaptureEndpoints 0
'@
$pairs = Require-Replace $pairs $oldCaptureArray $newCaptureArray 'capture endpoint list'
Write-Utf8Bom $pairPath $pairs

# With capture count intentionally compiled to zero, current WDK /W4 /WX
# diagnoses `i < 0` as an always-false unsigned comparison. Keep upstream's
# loop structurally intact while using the equivalent count-termination form;
# for every valid endpoint count it visits exactly the same 0..count-1 range,
# and for Spatial P0 the zero-count loop remains a no-op.
$adapter = Get-Content -Raw -LiteralPath $adapterPath
$adapter = Require-Replace `
    $adapter `
    'for (ULONG i = 0; i < g_cCaptureEndpoints; ++i, ++ppAeMiniports)' `
    'for (ULONG i = 0; i != g_cCaptureEndpoints; ++i, ++ppAeMiniports)' `
    'zero-count capture loop warning'
Write-Utf8Bom $adapterPath $adapter

# Guardrails: fail the build rather than silently shipping a partly branded or
# unexpectedly expanded endpoint if the pinned upstream source drifts.
$verifyInx = Get-Content -Raw -LiteralPath $inxPath
$verifyPairs = Get-Content -Raw -LiteralPath $pairPath
$verifyAdapter = Get-Content -Raw -LiteralPath $adapterPath
if ($verifyInx -notmatch 'ROOT\\SpatialAudioEndpoint') {
    throw 'Spatial root hardware ID verification failed'
}
if ($verifyInx -notmatch 'WaveSpeaker\.szPname="Spatial"') {
    throw 'Spatial render endpoint branding verification failed'
}
if ($verifyInx -match '(?m)^AddInterface=.*WaveMicArray1') {
    throw 'Spatial P0 still exposes the sample microphone wave interface'
}
if ($verifyPairs -notmatch '#define g_cCaptureEndpoints 0') {
    throw 'Spatial P0 capture miniport disable verification failed'
}
if ($verifyAdapter -notmatch 'i != g_cCaptureEndpoints') {
    throw 'Spatial P0 capture loop compile guard verification failed'
}

Write-Host 'Prepared Spatial P0 endpoint source: one silent Windows render endpoint, unique root hardware ID, no DSP, no sample microphone endpoint.'
