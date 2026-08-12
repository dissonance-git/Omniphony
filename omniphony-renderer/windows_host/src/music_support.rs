use crate::music_early_reflections::HrtfEarlyReflectionField;
use anyhow::{Context, bail};
use bridge_api::RInputTransport;
use orender_engine::{Engine, RenderedAudio};
use renderer::music_field::MUSIC_FIELD_CHANNELS;
use std::path::{Path, PathBuf};

/// The normal Windows host has one listening model.
///
/// Historical profile experiments are preserved in git history and
/// `docs/listening-history.md`, but they are not runtime product modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpatialProfile {
    Current,
}

impl SpatialProfile {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        // Intentionally ignore historical OMNIPHONY_PROFILE values. The shipped
        // Windows host has one model so stale environment variables cannot
        // silently select a retired experiment.
        Ok(Self::Current)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
        }
    }
}

fn current_model_config(base: &str) -> String {
    let mut cfg = base.to_string();
    cfg = cfg.replace("      level: 0.32", "      level: 0.36");
    // Keep the transient-aware measured-HRTF early field intact, but reduce the
    // low-level late closure after listening found the center slightly too wet.
    // Spatial scale should come from geometry and early directional evidence,
    // leaving centered vocals anchored in the protected master.
    cfg = cfg.replace("      level: 0.028", "      level: 0.016");
    cfg = cfg.replace("      rt60_s: 0.16", "      rt60_s: 0.12");

    // The Current model owns first-order reflections in the fixed-cost six-bus
    // measured-HRTF field below, so disable the inherited analytic reflection
    // bank to prevent duplicate early energy.
    cfg.replace(
        "    reflections:\n      enabled: true",
        "    reflections:\n      enabled: false",
    )
}

pub(crate) struct MusicSupportRenderer {
    primary: Engine,
    early_reflections: HrtfEarlyReflectionField,
    primary_pcm: Vec<u8>,
}

impl MusicSupportRenderer {
    pub(crate) fn new(_profile: SpatialProfile, sample_rate_hz: u32) -> anyhow::Result<Self> {
        let bundle = Bundle::embedded()?;
        orender_engine::bridge_loader::register_linked_bridge(reference_bridge::linked_library)
            .context("failed to register linked reference PCM bridge")?;

        let mut primary = build_engine(
            &bundle.primary_config,
            &bundle.layout,
            sample_rate_hz,
            "Current model support",
        )?;
        let early_reflections = HrtfEarlyReflectionField::new(sample_rate_hz);

        let header = streaming_f32_wav_header(MUSIC_FIELD_CHANNELS as u16, sample_rate_hz);
        seed_engine(&mut primary, &header, "Current model support")?;

        Ok(Self {
            primary,
            early_reflections,
            primary_pcm: Vec::new(),
        })
    }

    /// Retained temporarily for the existing diagnostic print path. The Current
    /// model no longer has a hybrid branch.
    pub(crate) fn is_hybrid(&self) -> bool {
        false
    }

    pub(crate) fn process(&mut self, field_input: &[f32]) -> anyhow::Result<Vec<RenderedAudio>> {
        f32_as_le_bytes(field_input, &mut self.primary_pcm);
        let primary = self
            .primary
            .process(&self.primary_pcm, RInputTransport::Raw, 0)
            .context("Current model music support render failed")?;
        let early = self.early_reflections.process(field_input)?;
        add_stereo_support(primary, &early)
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

fn add_stereo_support(
    mut primary: Vec<RenderedAudio>,
    added: &[f32],
) -> anyhow::Result<Vec<RenderedAudio>> {
    let total: usize = primary.iter().map(|block| block.samples.len()).sum();
    if total != added.len() {
        bail!(
            "Current model HRTF early-reflection support length mismatch: renderer={} reflection_field={}",
            total,
            added.len()
        );
    }
    let mut cursor = 0usize;
    for block in &mut primary {
        if block.n_channels != 2 {
            bail!(
                "Current model HRTF early-reflection field expected stereo primary output, got {} channels",
                block.n_channels
            );
        }
        let end = cursor + block.samples.len();
        for (dst, src) in block.samples.iter_mut().zip(&added[cursor..end]) {
            *dst += *src;
        }
        cursor = end;
    }
    Ok(primary)
}

struct Bundle {
    primary_config: PathBuf,
    layout: PathBuf,
}

impl Bundle {
    fn embedded() -> anyhow::Result<Self> {
        const FIELD_CONFIG: &str =
            include_str!("../../assets/binaural-baselines/stereo-field-prototype.yaml");
        const GRID_LAYOUT: &str =
            include_str!("../../../layouts/system-h-derived-22.0-upper60-grid10.yaml");

        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Omniphony")
            .join("runtime");
        std::fs::create_dir_all(&root)
            .context("failed to create embedded Omniphony runtime directory")?;

        let layout = root.join("system-h-headphone-current.yaml");
        write_embedded_asset(&layout, GRID_LAYOUT)?;

        let primary_config = root.join("stereo-field-current.yaml");
        write_embedded_asset(&primary_config, &current_model_config(FIELD_CONFIG))?;

        Ok(Self {
            primary_config,
            layout,
        })
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
