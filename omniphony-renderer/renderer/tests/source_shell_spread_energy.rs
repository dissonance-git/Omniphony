use renderer::spatial_vbap::VbapPanner;
use renderer::speaker_layout::SpeakerLayout;

const SOURCE_SHELL: &str =
    include_str!("../../../layouts/system-h-derived-22.0-upper60-grid10.yaml");

fn power(gains: &[f32]) -> f32 {
    gains.iter().map(|gain| gain * gain).sum()
}

#[test]
fn source_extent_remains_constant_power_across_the_22_direction_shell() {
    let layout = SpeakerLayout::from_yaml_str(SOURCE_SHELL).expect("embedded source shell");
    let (speaker_dirs, _) = layout.spatializable_positions();
    assert_eq!(speaker_dirs.len(), 22);

    let panner = VbapPanner::new(&speaker_dirs, 4, 4, 0.0, Default::default())
        .expect("22-direction VBAP");

    // Representative front, elevated side, rear and lower-front locations.
    // Extent is normalized 0..1 by the source-aware presentation layer.
    let positions = [
        [0.0_f32, 1.0, 0.0],
        [0.55, 0.70, 0.45],
        [-0.70, -0.55, 0.30],
        [0.35, 0.90, -0.30],
    ];
    let spreads = [0.0_f32, 0.25, 0.50, 0.75, 1.0];

    for position in positions {
        for spread in spreads {
            let gains = panner.get_gains_cartesian(
                position[0],
                position[1],
                position[2],
                spread,
            );
            assert_eq!(gains.len(), 22);
            assert!(gains.iter().all(|gain| gain.is_finite() && *gain >= 0.0));

            let shell_power = power(&gains);
            assert!(
                (shell_power - 1.0).abs() < 0.05,
                "source extent must redistribute shell energy rather than change it: pos={position:?} spread={spread} power={shell_power}"
            );
        }
    }
}
