use renderer::speaker_layout::SpeakerLayout;
use renderer::spatial_vbap::VbapPanner;

// Historical runtime compatibility path: currently the controlled headphone
// height A/B with the upper ring moved to +60 degrees.
const ACTIVE_HEADPHONE_22: &str = include_str!("../../../layouts/itu-r-bs2051-system-h-22.0.yaml");
// Normative/reference System H geometry remains frozen separately.
const SYSTEM_H_REFERENCE_22: &str =
    include_str!("../../../layouts/reference/itu-r-bs2051-system-h-22.0.yaml");

fn elevation_deg(speaker: &renderer::speaker_layout::Speaker) -> f32 {
    let horizontal = (speaker.x * speaker.x + speaker.y * speaker.y).sqrt();
    speaker.z.atan2(horizontal).to_degrees()
}

fn active_panner() -> VbapPanner {
    let layout = SpeakerLayout::from_yaml_str(ACTIVE_HEADPHONE_22)
        .expect("active headphone layout must parse");
    VbapPanner::new(&layout.positions(), 5, 5, 0.0, Default::default())
        .expect("active headphone directions must triangulate")
        .with_negative_z(true)
}

fn gain_energy(gains: &[f32]) -> f32 {
    gains.iter().map(|gain| gain * gain).sum()
}

#[test]
fn canonical_system_h_reference_keeps_30_degree_upper_ring() {
    let layout = SpeakerLayout::from_yaml_str(SYSTEM_H_REFERENCE_22)
        .expect("canonical System H reference layout must parse");
    assert_eq!(layout.num_speakers(), 22);

    for name in [
        "TpFC", "TpFL", "TpFR", "TpSiL", "TpSiR", "TpBL", "TpBR", "TpBC",
    ] {
        let speaker = layout
            .speakers
            .iter()
            .find(|speaker| speaker.name == name)
            .unwrap_or_else(|| panic!("missing canonical upper speaker {name}"));
        let elevation = elevation_deg(speaker);
        assert!(
            (elevation - 30.0).abs() < 0.05,
            "canonical {name} moved away from +30 degrees: {elevation}"
        );
    }
}

#[test]
fn active_headphone_shell_is_a_real_three_layer_unit_sphere() {
    let layout = SpeakerLayout::from_yaml_str(ACTIVE_HEADPHONE_22)
        .expect("active headphone layout must parse");

    assert_eq!(layout.num_speakers(), 22);
    assert!(layout.speakers.iter().all(|speaker| speaker.spatialize));
    assert!(
        layout
            .speakers
            .iter()
            .all(|speaker| !speaker.name.to_ascii_uppercase().contains("LFE"))
    );

    for speaker in &layout.speakers {
        assert!(speaker.x.is_finite());
        assert!(speaker.y.is_finite());
        assert!(speaker.z.is_finite());
        let radius = (speaker.x * speaker.x
            + speaker.y * speaker.y
            + speaker.z * speaker.z)
            .sqrt();
        assert!(
            (radius - 1.0).abs() < 2.0e-4,
            "{} is off the unit sphere: radius={radius}",
            speaker.name
        );
    }

    let lower = layout
        .speakers
        .iter()
        .filter(|speaker| speaker.z < -0.45)
        .count();
    assert_eq!(lower, 3, "headphone shell must retain the bottom layer");

    let zenith = layout
        .speakers
        .iter()
        .find(|speaker| speaker.name == "TpC")
        .expect("headphone shell must retain top centre");
    assert!(zenith.x.abs() < 1.0e-6);
    assert!(zenith.y.abs() < 1.0e-6);
    assert!((zenith.z - 1.0).abs() < 1.0e-6);
}

#[test]
fn active_headphone_upper_ring_is_60_degrees() {
    let layout = SpeakerLayout::from_yaml_str(ACTIVE_HEADPHONE_22)
        .expect("active headphone layout must parse");

    for name in [
        "TpFC", "TpFL", "TpFR", "TpSiL", "TpSiR", "TpBL", "TpBR", "TpBC",
    ] {
        let speaker = layout
            .speakers
            .iter()
            .find(|speaker| speaker.name == name)
            .unwrap_or_else(|| panic!("missing active upper speaker {name}"));
        let elevation = elevation_deg(speaker);
        assert!(
            (elevation - 60.0).abs() < 0.05,
            "active {name} is not near +60 degrees: {elevation}"
        );
    }
}

#[test]
fn active_headphone_shell_can_pan_into_the_lower_hemisphere() {
    let panner = active_panner();
    let gains = panner.get_gains_cartesian(0.0, 0.866025, -0.5, 0.025);
    assert_eq!(gains.len(), 22);
    assert!(gains.iter().all(|gain| gain.is_finite()));
    let energy = gain_energy(&gains);
    assert!(energy > 0.5, "lower-hemisphere pan lost energy: {energy}");
}

#[test]
fn current_shell_virtual_poles_cover_lower_front_and_rear_quadrants() {
    let panner = active_panner();
    // Canonical 30-degree-down probes corresponding to lower-front and
    // lower-rear quadrants. The active 22-speaker shell has a sparse physical
    // bottom layer, so these are exactly the directions where an open convex
    // hull used to collapse toward silence before the virtual-pole path.
    for az_deg in [-135.0_f32, -45.0, 45.0, 135.0] {
        let az = az_deg.to_radians();
        let el = (-30.0_f32).to_radians();
        let cos_el = el.cos();
        let x = cos_el * az.sin();
        let y = cos_el * az.cos();
        let z = el.sin();
        let gains = panner.get_gains_cartesian(x, y, z, 0.0);
        assert!(gains.iter().all(|gain| gain.is_finite()));
        let energy = gain_energy(&gains);
        assert!(
            energy > 0.80,
            "Current virtual-pole coverage lost too much energy at az={az_deg} el=-30: {energy}"
        );
    }
}

#[test]
fn current_shell_has_no_full_sphere_energy_holes() {
    let panner = active_panner();
    // Fibonacci sampling avoids over-weighting the poles and makes this a real
    // whole-sphere coverage gate rather than a handful of friendly meridians.
    const N: usize = 512;
    const GOLDEN_ANGLE: f32 = 2.399_963_1;
    let mut worst = (f32::INFINITY, 0usize, [0.0_f32; 3]);

    for i in 0..N {
        let z = 1.0 - 2.0 * (i as f32 + 0.5) / N as f32;
        let radius = (1.0 - z * z).max(0.0).sqrt();
        let phi = GOLDEN_ANGLE * i as f32;
        let x = radius * phi.cos();
        let y = radius * phi.sin();
        let gains = panner.get_gains_cartesian(x, y, z, 0.0);
        assert!(gains.iter().all(|gain| gain.is_finite()));
        let energy = gain_energy(&gains);
        if energy < worst.0 {
            worst = (energy, i, [x, y, z]);
        }
    }

    assert!(
        worst.0 > 0.75,
        "Current full-sphere VBAP has an energy hole: energy={} sample={} xyz={:?}",
        worst.0,
        worst.1,
        worst.2
    );
}
