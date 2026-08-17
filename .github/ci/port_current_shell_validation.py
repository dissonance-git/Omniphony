from pathlib import Path

PATH = Path("omniphony-renderer/renderer/src/spatial_vbap/panner/native_validation.rs")
text = PATH.read_text(encoding="utf-8")

if "current_headphone_shell_conserves_energy_over_the_sphere" in text:
    raise SystemExit("Current-shell gates already present")

panner_anchor = '''fn panner_for(preset: &str) -> (NativeVbapLayout, usize) {
    let layout = SpeakerLayout::preset(preset).expect("known preset");
    let dirs: Vec<[f32; 2]> = layout
        .speakers
        .iter()
        .filter(|s| s.spatialize)
        .map(|s| [s.azimuth, s.elevation])
        .collect();
    let n = dirs.len();
    (
        NativeVbapLayout::from_speaker_dirs(&dirs, Default::default()).expect("triplet search"),
        n,
    )
}
'''
if panner_anchor not in text:
    raise SystemExit("panner_for anchor not found")

helper = r'''

/// Build the exact virtual-speaker shell embedded by the retained Current
/// headphone support renderer. This is the product geometry, not a generic
/// surround preset, so full-sphere validation must exercise it explicitly.
fn current_headphone_shell_panner() -> (NativeVbapLayout, usize) {
    const CURRENT_SHELL: &str = include_str!(
        "../../../../../layouts/system-h-derived-22.0-upper60-grid10.yaml"
    );
    let layout = SpeakerLayout::from_yaml_str(CURRENT_SHELL).expect("embedded Current shell");
    let dirs: Vec<[f32; 2]> = layout
        .speakers
        .iter()
        .filter(|speaker| speaker.spatialize)
        .map(|speaker| [speaker.azimuth, speaker.elevation])
        .collect();
    let n = dirs.len();
    assert_eq!(n, 22, "Current headphone shell geometry changed unexpectedly");
    (
        NativeVbapLayout::from_speaker_dirs(&dirs, Default::default())
            .expect("Current-shell triplet search"),
        n,
    )
}
'''
text = text.replace(panner_anchor, panner_anchor + helper, 1)

wide_anchor = '''/// The wide matrix: every shipped layout at a denser lattice, plus spread.
'''
if wide_anchor not in text:
    raise SystemExit("wide-matrix anchor not found")

current_tests = r'''
/// Product-specific full-sphere sweep for the 22-direction shell used by
/// `CurrentMusicSupportRenderer`. A generic 7.1.4 pass cannot prove that this
/// denser, asymmetric-height shell has no silent hole or gain discontinuity.
const CURRENT_SHELL_POINTS: usize = 2048;

#[test]
fn current_headphone_shell_conserves_energy_over_the_sphere() {
    let (panner, n_spk) = current_headphone_shell_panner();
    let mut worst = (0.0f32, 0.0f32, 0.0f32);
    for (az, el) in fibonacci_sphere(CURRENT_SHELL_POINTS) {
        let dev = energy_db(&panner, az, el);
        assert!(
            dev.is_finite(),
            "Current shell has a silent direction at az={az:.1} el={el:.1}"
        );
        if dev.abs() > worst.2.abs() {
            worst = (az, el, dev);
        }
    }
    println!(
        "[measure] Current-shell VBAP energy ({n_spk} speakers, {CURRENT_SHELL_POINTS} dirs): \
         worst {:+.4} dB at az={:.1} el={:.1}",
        worst.2, worst.0, worst.1
    );
    assert!(
        worst.2.abs() <= ENERGY_TOLERANCE_DB,
        "Current shell energy off by {:+.4} dB at az={:.1} el={:.1}, tolerance ±{ENERGY_TOLERANCE_DB} dB",
        worst.2,
        worst.0,
        worst.1
    );
}

#[test]
fn current_headphone_shell_is_continuous_across_triplet_boundaries() {
    let (panner, _) = current_headphone_shell_panner();
    let mut worst = (0.0f32, 0.0f32, 0.0f32);
    for (az, el) in fibonacci_sphere(CURRENT_SHELL_POINTS) {
        let (jump, at_az) = residual_jump(&panner, az, el);
        if jump > worst.2 {
            worst = (at_az, el, jump);
        }
    }
    println!(
        "[measure] Current-shell VBAP seams ({CURRENT_SHELL_POINTS} dirs, bisected to {:.0e}°): \
         worst surviving jump {:.6} at az={:.2} el={:.1}",
        SEAM_SPAN_DEG / (1u32 << SEAM_BISECT_STEPS) as f32,
        worst.2,
        worst.0,
        worst.1
    );
    assert!(
        worst.2 <= MAX_SEAM_JUMP,
        "Current shell gain vector jumps {:.6} across only {:.0e}° at az={:.2} el={:.1} (max {MAX_SEAM_JUMP})",
        worst.2,
        SEAM_SPAN_DEG / (1u32 << SEAM_BISECT_STEPS) as f32,
        worst.0,
        worst.1
    );
}

'''
text = text.replace(wide_anchor, current_tests + wide_anchor, 1)
PATH.write_text(text, encoding="utf-8")
print("added Current 22-direction full-sphere VBAP gates")
