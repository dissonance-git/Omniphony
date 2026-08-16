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

    # Installed applications run unelevated from Program Files. Runtime logging
    # therefore belongs under the per-user settings root, not beside the EXE.
    # Logging is diagnostic only and must never prevent the audio-engine child
    # from starting if the log file itself cannot be opened.
    $oldAppendLog = @'
fn append_log(message: &str) {
    let Ok(root) = executable_root() else {
        return;
    };
    let path = root.join("omniphony.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[supervisor] {message}");
    }
}
'@
    $newAppendLog = @'
fn append_log(message: &str) {
    let root = settings_root();
    let _ = create_dir_all(&root);
    let path = root.join("omniphony.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[supervisor] {message}");
    }
}
'@
    $supervisor = Require-Replace $supervisor $oldAppendLog $newAppendLog 'writable runtime supervisor log root'

    $oldWorkerLog = @'
    let log_path = root.join("omniphony.log");
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open {}", log_path.display()))?;
    let log_err = log.try_clone().context("failed to clone Omniphony log handle")?;

    let mut child = Command::new(&executable)
'@
    $newWorkerLog = @'
    let log_root = settings_root();
    let _ = create_dir_all(&log_root);
    let log_path = log_root.join("omniphony.log");
    let (worker_stdout, worker_stderr) = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        Ok(log) => {
            let stderr = log
                .try_clone()
                .map(Stdio::from)
                .unwrap_or_else(|_| Stdio::null());
            (Stdio::from(log), stderr)
        }
        Err(_) => (Stdio::null(), Stdio::null()),
    };

    let mut child = Command::new(&executable)
'@
    $supervisor = Require-Replace $supervisor $oldWorkerLog $newWorkerLog 'non-fatal writable worker log setup'

    # Personal build only: make both sides of the development bridge explicit.
    # Windows applications render into Steam Streaming Speakers. The worker
    # captures that endpoint directly, while its processed output goes only to
    # the user's physical Noire X/K7 endpoint. Neither side depends on whatever
    # Windows happens to call the current default device after startup.
    $oldWorkerLaunch = @'
        .env("OMNIPHONY_INTERNAL_ENGINE", "1")
        .env("OMNIPHONY_PROFILE", "external")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
'@
    $newWorkerLaunch = @'
        .env("OMNIPHONY_INTERNAL_ENGINE", "1")
        .env("OMNIPHONY_PROFILE", "external")
        .arg("--capture")
        .arg("Steam Streaming Speakers")
        .arg("--output")
        .arg("Dan Clark Noire X")
        .stdin(Stdio::piped())
        .stdout(worker_stdout)
        .stderr(worker_stderr)
'@
    $supervisor = Require-Replace $supervisor $oldWorkerLaunch $newWorkerLaunch 'explicit transport capture and physical output child launch'
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

    $worker = Require-Replace $worker `
        '    AudioCaptureClient, AudioClient, Direction, SampleType, StreamMode, WaveFormat,' `
        '    AudioCaptureClient, AudioClient, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat,' `
        'WASAPI endpoint enumerator import'

    $worker = Require-Replace $worker @'
struct Args {
    output: Option<String>,
    start_off: bool,
}
'@ @'
struct Args {
    capture: Option<String>,
    output: Option<String>,
    start_off: bool,
}
'@ 'capture endpoint argument storage'

    $worker = Require-Replace $worker @'
        match arg.as_str() {
            "--output" => {
'@ @'
        match arg.as_str() {
            "--capture" => {
                parsed.capture = Some(
                    args.next()
                        .context("--capture requires a render-endpoint name substring")?,
                );
            }
            "--output" => {
'@ 'capture endpoint argument parsing'

    $worker = Require-Replace $worker `
        '    let mut loopback = LoopbackCapture::open_stereo(SAMPLE_RATE_HZ)?;' `
        '    let mut loopback = LoopbackCapture::open_stereo(SAMPLE_RATE_HZ, args.capture.as_deref())?;' `
        'explicit endpoint loopback construction'

    $worker = Require-Replace $worker `
        '    println!("  capture: {SAMPLE_RATE_HZ} Hz / stereo / f32 process loopback");' `
        '    println!("  capture: {SAMPLE_RATE_HZ} Hz / stereo / f32 {}", loopback.source());' `
        'capture source banner'

    $worker = Require-Replace $worker @'
        let Some(input) = loopback.next_block()? else {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
'@ @'
        let Some(input) = loopback.next_block()? else {
            if last_meter_report.elapsed() >= Duration::from_secs(METER_INTERVAL_SECS) {
                println!("  capture heartbeat: no endpoint-loopback packets in the last {METER_INTERVAL_SECS} s (source may be idle)");
                report_playback_transport(&playback_telemetry);
                report_output_peak_guard(&mut output_peak_guard);
                last_meter_report = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
'@ 'capture no-packet heartbeat'

    $oldLoopback = @'
struct LoopbackCapture {
    client: AudioClient,
    capture: AudioCaptureClient,
    scratch: Vec<u8>,
}

impl LoopbackCapture {
    fn open_stereo(sample_rate_hz: u32) -> anyhow::Result<Self> {
        const BUFFER_DURATION_HNS: i64 = 200_000;
        let mode = StreamMode::PollingShared {
            autoconvert: true,
            buffer_duration_hns: BUFFER_DURATION_HNS,
        };
        let mask = make_channelmasks(2).into_iter().next().unwrap_or(0);
        let format = WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            sample_rate_hz as usize,
            2,
            Some(mask),
        );
        let mut client = AudioClient::new_application_loopback_client(std::process::id(), false)
            .context("failed to activate self-excluding Windows process loopback")?;
        client
            .initialize_client(&format, &Direction::Capture, &mode)
            .context("Windows process loopback rejected required stereo 48 kHz float format")?;
        let capture = client
            .get_audiocaptureclient()
            .context("process loopback initialized but exposed no capture client")?;
        Ok(Self {
            client,
            capture,
            scratch: Vec::new(),
        })
    }
'@
    $newLoopback = @'
struct LoopbackCapture {
    client: AudioClient,
    capture: AudioCaptureClient,
    scratch: Vec<u8>,
    source: String,
}

impl LoopbackCapture {
    fn open_stereo(sample_rate_hz: u32, endpoint_name: Option<&str>) -> anyhow::Result<Self> {
        const BUFFER_DURATION_HNS: i64 = 200_000;
        let mode = StreamMode::PollingShared {
            autoconvert: true,
            buffer_duration_hns: BUFFER_DURATION_HNS,
        };
        let mask = make_channelmasks(2).into_iter().next().unwrap_or(0);
        let format = WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            sample_rate_hz as usize,
            2,
            Some(mask),
        );

        if let Some(needle) = endpoint_name {
            let enumerator = DeviceEnumerator::new()
                .context("failed to create WASAPI render-endpoint enumerator")?;
            let devices = enumerator
                .get_device_collection(&Direction::Render)
                .context("failed to enumerate active WASAPI render endpoints")?;
            let count = devices
                .get_nbr_devices()
                .context("failed to count active WASAPI render endpoints")?;
            let needle_lower = needle.to_ascii_lowercase();
            let mut selected = None;
            for index in 0..count {
                let device = devices
                    .get_device_at_index(index)
                    .with_context(|| format!("failed to inspect WASAPI render endpoint {index}"))?;
                let name = device
                    .get_friendlyname()
                    .unwrap_or_else(|_| "<unnamed render endpoint>".to_string());
                if name.to_ascii_lowercase().contains(&needle_lower) {
                    selected = Some((device, name));
                    break;
                }
            }
            let (device, name) = selected.with_context(|| {
                format!("no active WASAPI render endpoint contains '{needle}'")
            })?;
            let mut client = device
                .get_iaudioclient()
                .with_context(|| format!("failed to open endpoint loopback client for {name}"))?;
            client
                .initialize_client(&format, &Direction::Capture, &mode)
                .with_context(|| {
                    format!("endpoint loopback for {name} rejected stereo 48 kHz float format")
                })?;
            let capture = client
                .get_audiocaptureclient()
                .with_context(|| format!("endpoint loopback for {name} exposed no capture client"))?;
            return Ok(Self {
                client,
                capture,
                scratch: Vec::new(),
                source: format!("endpoint loopback <- {name}"),
            });
        }

        let mut client = AudioClient::new_application_loopback_client(std::process::id(), false)
            .context("failed to activate self-excluding Windows process loopback")?;
        client
            .initialize_client(&format, &Direction::Capture, &mode)
            .context("Windows process loopback rejected required stereo 48 kHz float format")?;
        let capture = client
            .get_audiocaptureclient()
            .context("process loopback initialized but exposed no capture client")?;
        Ok(Self {
            client,
            capture,
            scratch: Vec::new(),
            source: "self-excluding process loopback".to_string(),
        })
    }

    fn source(&self) -> &str {
        &self.source
    }
'@
    $worker = Require-Replace $worker $oldLoopback $newLoopback 'direct transport endpoint loopback implementation'

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

    Write-Host 'Prepared Omniphony for Windows host: direct Steam endpoint loopback capture, explicit Dan Clark Noire X output, writable non-fatal logging, and hard virtual-transport rejection.'
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
