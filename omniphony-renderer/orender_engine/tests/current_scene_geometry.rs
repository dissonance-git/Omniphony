use orender_engine::current_music_support::CurrentMusicSupportRenderer;
use renderer::music_field::{MUSIC_FIELD_CHANNELS, MusicFieldProcessor};
use std::f32::consts::TAU;

const SAMPLE_RATE_HZ: u32 = 48_000;
const FRAMES: usize = 2_048;

const CANONICAL_8_1_4_4_ORDER: [&str; 17] = [
    "L", "R", "C", "LFE", "Ls", "Rs", "Lb", "Rb", "Cb", "Tfl", "Tfr", "Tbl", "Tbr",
    "Bfl", "Bfr", "Bbl", "Bbr",
];

const STEREO_DERIVED_EMPTY_INDICES: [usize; 7] = [2, 3, 8, 13, 14, 15, 16];
const STEREO_DERIVED_EARNED_INDICES: [usize; 10] = [0, 1, 4, 5, 6, 7, 9, 10, 11, 12];

fn stereo_probe() -> Vec<f32> {
    let mut input = Vec::with_capacity(FRAMES * 2);
    for frame in 0..FRAMES {
        let t = frame as f32 / SAMPLE_RATE_HZ as f32;
        let left = 0.12 * (TAU * 1_300.0 * t).sin() + 0.05 * (TAU * 6_400.0 * t).sin();
        let right = 0.09 * (TAU * 1_900.0 * t + 0.7).sin()
            - 0.04 * (TAU * 7_200.0 * t + 0.2).sin();
        input.push(left);
        input.push(right);
    }
    input
}

#[test]
fn current_scene_stays_canonical_8_1_4_4_before_render_expansion() {
    assert_eq!(
        MUSIC_FIELD_CHANNELS,
        CANONICAL_8_1_4_4_ORDER.len(),
        "Current product scene vocabulary must remain canonical 8.1.4.4"
    );
    assert_eq!(
        CANONICAL_8_1_4_4_ORDER,
        [
            "L", "R", "C", "LFE", "Ls", "Rs", "Lb", "Rb", "Cb", "Tfl", "Tfr", "Tbl",
            "Tbr", "Bfl", "Bfr", "Bbl", "Bbr",
        ]
    );

    let mut processor = MusicFieldProcessor::new(SAMPLE_RATE_HZ);
    let field = processor.process_interleaved_stereo(&stereo_probe());
    assert_eq!(field.len(), FRAMES * MUSIC_FIELD_CHANNELS);

    let mut earned_energy = 0.0f32;
    for frame in field.chunks_exact(MUSIC_FIELD_CHANNELS) {
        for &index in &STEREO_DERIVED_EMPTY_INDICES {
            assert_eq!(
                frame[index], 0.0,
                "stereo inference populated EMPTY canonical lane {} at index {index}",
                CANONICAL_8_1_4_4_ORDER[index]
            );
        }
        for &index in &STEREO_DERIVED_EARNED_INDICES {
            earned_energy += frame[index].abs();
        }
    }
    assert!(
        earned_energy > 1.0e-4,
        "probe failed to excite any evidence-backed 8.1.4.4 support lane"
    );
}

#[test]
fn current_render_shell_stays_22_directions_above_the_17_lane_scene() {
    let shell = include_str!("../../../layouts/system-h-derived-22.0-upper60-grid10.yaml");
    let speaker_count = shell
        .lines()
        .filter(|line| line.trim_start().starts_with("- name:"))
        .count();
    assert_eq!(
        speaker_count, 22,
        "Current expansion shell must remain the 22-direction System-H-derived lattice"
    );
}

#[test]
fn canonical_field_reaches_current_binaural_renderer_as_stereo() {
    let mut field_processor = MusicFieldProcessor::new(SAMPLE_RATE_HZ);
    let field = field_processor.process_interleaved_stereo(&stereo_probe());
    assert_eq!(field.len(), FRAMES * MUSIC_FIELD_CHANNELS);

    let mut renderer = CurrentMusicSupportRenderer::new(SAMPLE_RATE_HZ)
        .expect("Current 22-direction support renderer should construct");

    let mut rendered = Vec::new();
    for _ in 0..4 {
        let blocks = renderer
            .process(&field)
            .expect("canonical 8.1.4.4 field should render through Current shell");
        rendered.extend(blocks);
        if rendered.iter().any(|block| !block.samples.is_empty()) {
            break;
        }
    }

    assert!(
        rendered.iter().any(|block| !block.samples.is_empty()),
        "Current renderer accepted the 17-lane field but emitted no binaural PCM"
    );

    let mut audible_energy = 0.0f32;
    for block in &rendered {
        assert_eq!(
            block.n_channels, 2,
            "22-direction Current shell must terminate in stereo headphone output"
        );
        assert!(
            block.samples.iter().all(|sample| sample.is_finite()),
            "Current binaural output contained a non-finite sample"
        );
        audible_energy += block.samples.iter().map(|sample| sample.abs()).sum::<f32>();
    }
    assert!(
        audible_energy > 1.0e-5,
        "Current 8.1.4.4 -> 22-direction -> binaural path produced silent output"
    );
}
