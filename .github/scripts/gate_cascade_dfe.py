from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    p = Path(path)
    s = p.read_text()
    if old not in s:
        raise SystemExit(f"{label}: expected source fragment missing")
    p.write_text(s.replace(old, new, 1))


replace_once(
    "omniphony-renderer/renderer/src/config.rs",
    '''    /// Path to a SOFA HRTF file, used when `hrir_source = "sofa"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrtf_sofa_path: Option<PathBuf>,''',
    '''    /// Path to a SOFA HRTF file, used when `hrir_source = "sofa"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hrtf_sofa_path: Option<PathBuf>,
    /// Cascade-only common HRTF colour compensation. Absent / "off" keeps
    /// generic direct↔cascade parity; "saf_partial" opts into the measured,
    /// bounded SAF/KEMAR diffuse-field correction used by the music host.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectral_compensation: Option<String>,''',
    "BinauralConfig",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/mod.rs",
    '''    /// Scratch per-channel world positions for the binaural path (reused).
    binaural_pos_buf: Vec<[f64; 3]>,''',
    '''    /// Explicit host/config opt-in for cascade-only spectral compensation.
    /// Defaults false so generic cascade remains numerically comparable to the
    /// direct binaural path; the music topology opts in for SAF/KEMAR only.
    cascade_spectral_compensation: bool,

    /// Scratch per-channel world positions for the binaural path (reused).
    binaural_pos_buf: Vec<[f64; 3]>,''',
    "SpatialRenderer field",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/mod.rs",
    '''    pub fn auto_gain_triggered(&self) -> bool {
        self.auto_gain_triggered
            .load(std::sync::atomic::Ordering::Relaxed)
    }
''',
    '''    pub fn auto_gain_triggered(&self) -> bool {
        self.auto_gain_triggered
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Enable/disable the cascade-only common HRTF spectral compensation.
    /// This is intentionally a construction/host policy rather than a generic
    /// binaural default: direct mode and unconfigured cascades remain untouched.
    pub fn set_cascade_spectral_compensation(&mut self, enabled: bool) {
        self.cascade_spectral_compensation = enabled;
    }
''',
    "SpatialRenderer setter",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/mod.rs",
    '''            let (binaural_params, ears) = {
                let g = self.control.live.read();''',
    '''            let (binaural_params, ears, saf_cascade_compensation) = {
                let g = self.control.live.read();''',
    "binaural tuple start",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/mod.rs",
    '''                    },
                    g.binaural.ears,
                )
            };''',
    '''                    },
                    g.binaural.ears,
                    self.cascade_spectral_compensation
                        && matches!(g.binaural.hrir_source, crate::binaural::HrirSource::SafKemar),
                )
            };''',
    "binaural tuple close",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/mod.rs",
    '''                    live.speaker_params,
                    &binaural_params,
                    &mut output,
                );''',
    '''                    live.speaker_params,
                    &binaural_params,
                    saf_cascade_compensation,
                    &mut output,
                );''',
    "cascade call",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/construction.rs",
    '''            binaural: crate::binaural::BinauralRenderer::new(sample_rate),
            cascade: None,
            last_mix_num_speakers: 0,
            binaural_pos_buf: Vec::new(),''',
    '''            binaural: crate::binaural::BinauralRenderer::new(sample_rate),
            cascade: None,
            last_mix_num_speakers: 0,
            cascade_spectral_compensation: false,
            binaural_pos_buf: Vec::new(),''',
    "construction default",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/cascade.rs",
    '''    speaker_params: &[crate::live_params::SpeakerLiveParams],
    binaural_params: &crate::binaural::BinauralFrameParams,
    output: &mut [f32],''',
    '''    speaker_params: &[crate::live_params::SpeakerLiveParams],
    binaural_params: &crate::binaural::BinauralFrameParams,
    spectral_compensation: bool,
    output: &mut [f32],''',
    "cascade signature",
)

replace_once(
    "omniphony-renderer/renderer/src/spatial_renderer/cascade.rs",
    '''    let compensation = cascade
        .diffuse_compensation
        .get_or_insert_with(|| DiffuseFieldCompensator::saf_kemar_partial(stage.sample_rate));
    compensation.process_interleaved_stereo_in_place(output);

    diag''',
    '''    if spectral_compensation {
        let compensation = cascade
            .diffuse_compensation
            .get_or_insert_with(|| DiffuseFieldCompensator::saf_kemar_partial(stage.sample_rate));
        compensation.process_interleaved_stereo_in_place(output);
    } else if let Some(compensation) = cascade.diffuse_compensation.as_mut() {
        compensation.reset_runtime_state();
    }

    diag''',
    "cascade compensation gate",
)

replace_once(
    "omniphony-renderer/orender_engine/src/engine.rs",
    '''        let renderer = build_spatial_renderer(
            &params,
            layout,
            sample_rate,
            vbap_defaults,
            preferred,
            render_cfg.as_ref(),
        )?;

        // Seed monitoring cadences from config''',
    '''        let mut renderer = build_spatial_renderer(
            &params,
            layout,
            sample_rate,
            vbap_defaults,
            preferred,
            render_cfg.as_ref(),
        )?;
        let cascade_spectral_compensation = render_cfg
            .as_ref()
            .and_then(|render| render.binaural.as_ref())
            .and_then(|binaural| binaural.spectral_compensation.as_deref())
            .is_some_and(|mode| mode.eq_ignore_ascii_case("saf_partial"));
        if let Some(mode) = render_cfg
            .as_ref()
            .and_then(|render| render.binaural.as_ref())
            .and_then(|binaural| binaural.spectral_compensation.as_deref())
        {
            if !mode.eq_ignore_ascii_case("off") && !mode.eq_ignore_ascii_case("saf_partial") {
                log::warn!(
                    "unknown binaural spectral_compensation '{}'; leaving cascade compensation off",
                    mode
                );
            }
        }
        renderer.set_cascade_spectral_compensation(cascade_spectral_compensation);

        // Seed monitoring cadences from config''',
    "engine config gate",
)

replace_once(
    "omniphony-renderer/assets/binaural-baselines/stereo-field-prototype.yaml",
    '''    mode: cascaded
    hrir_source: saf

    unit_scale_m: 7.25''',
    '''    mode: cascaded
    hrir_source: saf
    spectral_compensation: saf_partial

    unit_scale_m: 7.25''',
    "music config opt-in",
)
