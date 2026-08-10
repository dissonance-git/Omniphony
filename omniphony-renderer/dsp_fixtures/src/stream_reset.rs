//! Always-on stream-boundary fidelity regressions.
//!
//! A decoder seek, track change or explicit stream restart is a discontinuity in
//! the canonical audio timeline. Old FIR/delay/reflection/reverb energy must not
//! leak into the first samples of the next stream, but immutable renderer/HRTF
//! configuration should stay warm.

use crate::scene::{SAMPLE_RATE, build_renderer_binaural, pseudo};
use renderer::live_params::{OutputMode, RampMode};
use renderer::spatial_renderer::SpatialChannelEvent;
use renderer::speaker_layout::SpeakerLayout;

fn object_event() -> SpatialChannelEvent {
    SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(0),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.55, 0.82, 0.18]),
        sample_pos: Some(0),
    }
}

#[test]
fn stream_reset_clears_previous_binaural_audio_history() {
    let mut renderer = build_renderer_binaural(
        SpeakerLayout::preset("7.1.4").expect("known preset"),
        true,
        false,
    );
    {
        let control = renderer.renderer_control();
        control.set_requested_ramp_mode(RampMode::Sample);
        let mut live = control.live.write();
        live.ramp_mode = RampMode::Sample;
        live.binaural.output_mode = OutputMode::Binaural;
        live.binaural.reflections.enabled = true;
        live.binaural.reflections.level = 0.7;
        live.binaural.reverb.enabled = true;
        live.binaural.reverb.level = 0.5;
        live.binaural.reverb.rt60_s = 0.6;
        live.binaural.reverb.predelay_ms = 0.0;
    }

    let event = object_event();
    let block = 128usize;
    let total = 2_560usize;
    let mut reuse = Vec::new();
    for start in (0..total).step_by(block) {
        let pcm: Vec<f32> = (0..block)
            .map(|s| pseudo((50_000 + start + s) as u64) * 0.35)
            .collect();
        let events: &[SpatialChannelEvent] = if start == 0 {
            std::slice::from_ref(&event)
        } else {
            &[]
        };
        let frame = renderer
            .render_frame(&pcm, 1, events, reuse, false)
            .expect("fill direct and room history");
        reuse = frame.samples;
        reuse.clear();
    }

    renderer.reset_runtime_state();

    // Keep the new stream's channel logically active while feeding exact silence.
    // Without a whole-pipeline reset, old FIR/ITD/reflection/FDN history can leak
    // into this buffer even though the new PCM contains no energy.
    let silence = vec![0.0f32; 1_024];
    let frame = renderer
        .render_frame(&silence, 1, std::slice::from_ref(&event), reuse, false)
        .expect("render first silent block of new stream");

    let peak = frame.samples.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
    eprintln!("post-reset stale-history peak={peak:.9}");
    assert!(
        peak <= 1.0e-8,
        "old audio history leaked across stream reset: peak={peak:.9}"
    );
    assert_eq!(SAMPLE_RATE, 48_000);
}
