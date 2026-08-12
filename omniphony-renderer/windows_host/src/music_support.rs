use anyhow::{Context, bail};
use bridge_api::RInputTransport;
use orender_engine::{Engine, RenderedAudio};
use renderer::music_field::MUSIC_FIELD_CHANNELS;
use renderer::music_hybrid::{split_height_routes, sum_stereo_support};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpatialProfile {
    Control,
    All,
    Hybrid,
    Direct,
    External,
    Prtf,
    Close,
    Tracked,
    Diffuse,
}

impl SpatialProfile {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let raw = std::env::var("OMNIPHONY_PROFILE").unwrap_or_else(|_| "all".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "control" | "best" => Ok(Self::Control),
            "all" | "portal" => Ok(Self::All),
            "hybrid" | "hybrid-height" | "direct-height" => Ok(Self::Hybrid),
            "direct" | "direct-hrtf" => Ok(Self::Direct),
            "external" | "reflections" => Ok(Self::External),
            "prtf" | "pinna" => Ok(Self::Prtf),
            "close" | "distance" => Ok(Self::Close),
            "tracked" | "tracking" => Ok(Self::Tracked),
            "diffuse" | "decorrelated" => Ok(Self::Diffuse),
            other => bail!(
                "unknown OMNIPHONY_PROFILE '{other}'; expected control|all|hybrid|direct|external|prtf|close|tracked|diffuse"
            ),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::All => "all",
            Self::Hybrid => "hybrid",
            Self::Direct => "direct",
            Self::External => "external",
            Self::Prtf => "prtf",
            Self::Close => "close",
            Self::Tracked => "tracked",
            Self::Diffuse => "diffuse",
        }
    }

    fn uses_grid_aligned_upper_shell(self) -> bool {
        !matches!(self, Self::Control | Self::Direct)
    }

    fn configure_single(self, base: &str) -> String {
        let mut cfg = base.to_string();
        match self {
            Self::Control => {}
            Self::All | Self::Hybrid => {
                cfg = baseline_two_room(cfg);
            }
            Self::Direct => {
                cfg = cfg.replace("    mode: cascaded", "    mode: direct");
                cfg = cfg.replace("      level: 0.32", "      level: 0.24");
                cfg = cfg.replace("      level: 0.028", "      level: 0.015");
            }
            Self::External => {
                cfg = cfg.replace("      level: 0.32", "      level: 0.42");
                cfg = cfg.replace("      level: 0.028", "      level: 0.012");
                cfg = cfg.replace("      rt60_s: 0.16", "      rt60_s: 0.12");
            }
            Self::Prtf => {
                cfg = cfg.replace("    hrir_source: saf", "    hrir_source: prtf:100:72");
                cfg = cfg.replace(
                    "    spectral_compensation: saf_partial",
                    "    spectral_compensation: off",
                );
            }
            Self::Close => {
                cfg = cfg.replace("    unit_scale_m: 9.25", "    unit_scale_m: 2.25");
                cfg = cfg.replace("      room_width_m: 23.0", "      room_width_m: 8.0");
                cfg = cfg.replace("      room_depth_m: 32.0", "      room_depth_m: 11.0");
                cfg = cfg.replace("      room_height_m: 21.0", "      room_height_m: 7.0");
                cfg = cfg.replace("      level: 0.32", "      level: 0.24");
                cfg = cfg.replace("      level: 0.028", "      level: 0.012");
            }
            Self::Tracked => {
                cfg = baseline_two_room(cfg);
                cfg = cfg.replace(
                    "    air_absorption: true\n",
                    "    air_absorption: true\n    head_tracking:\n      osc_address: \"/android/rotationvector\"\n      format: \"rotvec\"\n",
                );
            }
            Self::Diffuse => {
                cfg = cfg.replace("      level: 0.32", "      level: 0.24");
                cfg = cfg.replace("      level: 0.028", "      level: 0.055");
                cfg = cfg.replace("      rt60_s: 0.16", "      rt60_s: 0.28");
                cfg = cfg.replace("      predelay_ms: 32.0", "      predelay_ms: 24.0");
            }
        }
        cfg
    }
}

fn baseline_two_room(mut cfg: String) -> String {
    cfg = cfg.replace("      level: 0.32", "      level: 0.36");
    cfg = cfg.replace("      level: 0.028", "      level: 0.020");
    cfg.replace("      rt60_s: 0.16", "      rt60_s: 0.14")
}

fn configure_direct_height(base: &str) -> String {
    let mut cfg = base.to_string();
    cfg = cfg.replace("    mode: cascaded", "    mode: direct");
    // The cascade remains the environmental/world branch. The direct-height
    // engine is intentionally localization-only so the same height event does
    // not acquire a second room, early field or late tail.
    cfg = cfg.replace("    spectral_compensation: saf_partial", "    spectral_compensation: off");
    cfg = cfg.replace(
        "    reflections:\n      enabled: true",
        "    reflections:\n      enabled: false",
    );
    cfg = cfg.replace(
        "    reverb:\n      enabled: true",
        "    reverb:\n      enabled: false",
    );
    // Preserve the measured HRTF's directional high-frequency structure on the
    // direct height branch. The cascade still owns distance/air cues for the
    // surrounding world.
    cfg.replace("    air_absorption: true", "    air_absorption: false")
}

pub(crate) struct MusicSupportRenderer {
    primary: Engine,
    height: Option<Engine>,
    primary_pcm: Vec<u8>,
    height_pcm: Vec<u8>,
}

impl MusicSupportRenderer {
    pub(crate) fn new(profile: SpatialProfile, sample_rate_hz: u32) -> anyhow::Result<Self> {
        let bundle = Bundle::embedded(profile)?;
        orender_engine::bridge_loader::register_linked_bridge(reference_bridge::linked_library)
            .context("failed to register linked reference PCM bridge")?;

        let mut primary = build_engine(
            &bundle.primary_config,
            &bundle.layout,
            sample_rate_hz,
            "primary support",
        )?;
        let mut height = if let Some(height_config) = bundle.height_config.as_ref() {
            Some(build_engine(
                height_config,
                &bundle.layout,
                sample_rate_hz,
                "direct height",
            )?)
        } else {
            None
        };

        let header = streaming_f32_wav_header(MUSIC_FIELD_CHANNELS as u16, sample_rate_hz);
        seed_engine(&mut primary, &header, "primary support")?;
        if let Some(engine) = height.as_mut() {
            seed_engine(engine, &header, "direct height")?;
        }

        Ok(Self {
            primary,
            height,
            primary_pcm: Vec::new(),
            height_pcm: Vec::new(),
        })
    }

    pub(crate) fn is_hybrid(&self) -> bool {
        self.height.is_some()
    }

    pub(crate) fn process(&mut self, field_input: &[f32]) -> anyhow::Result<Vec<RenderedAudio>> {
        if let Some(height_engine) = self.height.as_mut() {
            let (cascade_input, height_input) = split_height_routes(field_input)?;
            f32_as_le_bytes(&cascade_input, &mut self.primary_pcm);
            f32_as_le_bytes(&height_input, &mut self.height_pcm);

            let primary = self
                .primary
                .process(&self.primary_pcm, RInputTransport::Raw, 0)
                .context("hybrid cascade support render failed")?;
            let height = height_engine
                .process(&self.height_pcm, RInputTransport::Raw, 0)
                .context("hybrid direct-height render failed")?;
            combine_rendered_blocks(primary, height)
        } else {
            f32_as_le_bytes(field_input, &mut self.primary_pcm);
            self.primary
                .process(&self.primary_pcm, RInputTransport::Raw, 0)
                .context("music support render failed")
        }
    }
}

fn build_engine(
    config: &Path,
    layout: &Path,
    sample_rate_hz: u32,
    label: &str,
) -> anyhow::Result<Engine> {
    let mut engine = Engine::from_paths(
        Some(config),
        Some(layout),
        None,
        None,
        sample_rate_hz,
    )
    .with_context(|| format!("failed to construct Omniphony {label} engine"))?;
    engine.set_channel_render_mode_code(1);
    if engine.channel_count() != 2 {
        bail!(
            "{label} configuration expected 2 output channels but engine reports {}",
            engine.channel_count()
        );
    }
    Ok(engine)
}

fn seed_engine(engine: &mut Engine, header: &[u8], label: &str) -> anyhow::Result<()> {
    let output = engine
        .process(header, RInputTransport::Raw, 0)
        .with_context(|| format!("failed to seed {label} 12-lane PCM bridge"))?;
    if !output.is_empty() {
        bail!("{label} streaming WAV header unexpectedly produced audio");
    }
    Ok(())
}

fn combine_rendered_blocks(
    primary: Vec<RenderedAudio>,
    height: Vec<RenderedAudio>,
) -> anyhow::Result<Vec<RenderedAudio>> {
    if primary.len() != height.len() {
        bail!(
            "hybrid engines produced different block counts: cascade={} height={}",
            primary.len(),
            height.len()
        );
    }

    let mut out = Vec::with_capacity(primary.len());
    for (mut a, b) in primary.into_iter().zip(height.into_iter()) {
        if a.n_channels != 2 || b.n_channels != 2 {
            bail!(
                "hybrid engine output width changed: cascade={} height={}",
                a.n_channels,
                b.n_channels
            );
        }
        if a.samples.len() != b.samples.len() {
            bail!(
                "hybrid engines produced different sample counts: cascade={} height={}",
                a.samples.len(),
                b.samples.len()
            );
        }
        a.samples = sum_stereo_support(&a.samples, &b.samples)?;
        out.push(a);
    }
    Ok(out)
}

struct Bundle {
    primary_config: PathBuf,
    height_config: Option<PathBuf>,
    layout: PathBuf,
}

impl Bundle {
    fn embedded(profile: SpatialProfile) -> anyhow::Result<Self> {
        const FIELD_CONFIG: &str =
            include_str!("../../assets/binaural-baselines/stereo-field-prototype.yaml");
        const CONTROL_LAYOUT: &str =
            include_str!("../../../layouts/itu-r-bs2051-system-h-22.0.yaml");
        const GRID_LAYOUT: &str =
            include_str!("../../../layouts/system-h-derived-22.0-upper60-grid10.yaml");

        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Omniphony")
            .join("runtime");
        std::fs::create_dir_all(&root)
            .context("failed to create embedded Omniphony runtime directory")?;

        let layout = root.join(format!("system-h-headphone-{}.yaml", profile.as_str()));
        let layout_content = if profile.uses_grid_aligned_upper_shell() {
            GRID_LAYOUT
        } else {
            CONTROL_LAYOUT
        };
        write_embedded_asset(&layout, layout_content)?;

        if profile == SpatialProfile::Hybrid {
            let primary_config = root.join("stereo-field-hybrid-cascade.yaml");
            let height_config = root.join("stereo-field-hybrid-height.yaml");
            write_embedded_asset(
                &primary_config,
                &SpatialProfile::All.configure_single(FIELD_CONFIG),
            )?;
            write_embedded_asset(&height_config, &configure_direct_height(FIELD_CONFIG))?;
            Ok(Self {
                primary_config,
                height_config: Some(height_config),
                layout,
            })
        } else {
            let primary_config = root.join(format!("stereo-field-{}.yaml", profile.as_str()));
            write_embedded_asset(&primary_config, &profile.configure_single(FIELD_CONFIG))?;
            Ok(Self {
                primary_config,
                height_config: None,
                layout,
            })
        }
    }
}

fn write_embedded_asset(path: &Path, content: &str) -> anyhow::Result<()> {
    let current = std::fs::read_to_string(path).ok();
    if current.as_deref() != Some(content) {
        std::fs::write(path, content)
            .with_context(|| format!("failed to materialize {}", path.display()))?;
    }
    Ok(())
}

fn streaming_f32_wav_header(channels: u16, sample_rate_hz: u32) -> Vec<u8> {
    let block_align = channels.saturating_mul(4);
    let byte_rate = sample_rate_hz.saturating_mul(u32::from(block_align));
    let mut wav = Vec::with_capacity(44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&u32::MAX.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate_hz.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&32u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&u32::MAX.to_le_bytes());
    wav
}

fn f32_as_le_bytes(samples: &[f32], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(samples.len() * 4);
    for &sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
}
