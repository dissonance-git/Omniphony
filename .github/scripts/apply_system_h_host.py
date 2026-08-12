from pathlib import Path

p = Path("omniphony-renderer/windows_host/src/music_worker_evidence.rs")
s = p.read_text()

replacements = [
    (
        'println!("Omniphony for Headphones - frequency-evidence 7.1.4 shell prototype");',
        'println!("Omniphony for Headphones - protected-master full-sphere renderer");',
    ),
    (
        'println!("  field:   below 320 Hz protected; 320+ Hz derived 7.1.4 support");',
        'println!("  field:   below 320 Hz protected; 320+ Hz uses 12 evidence lanes");',
    ),
    (
        'println!(\n        "  acoustics: cascaded Omniphony virtual room / distance / HRTF / reflections / air cues"\n    );',
        'println!(\n        "  acoustics: 12 evidence lanes -> ITU System H 22-direction shell -> cascaded binaural room"\n    );',
    ),
    (
        '.context("failed to seed 7.1.4 music-field PCM bridge")?;',
        '.context("failed to seed 12-lane music-field PCM bridge")?;',
    ),
    (
        '        // Both additive branches are causal and produce one aligned stereo\n        // foundation sample / one 7.1.4 support frame per input frame. Buffer\n',
        '        // Both additive branches are causal and produce one aligned stereo\n        // foundation sample / one 12-lane support frame per input frame. Buffer\n',
    ),
    (
        '        const LAYOUT: &str = include_str!("../../../layouts/7.1.4.yaml");',
        '        const LAYOUT: &str = include_str!("../../../layouts/itu-r-bs2051-system-h-22.0.yaml");',
    ),
    (
        '        let layout = root.join("7.1.4.yaml");',
        '        let layout = root.join("itu-r-bs2051-system-h-22.0.yaml");',
    ),
]

for old, new in replacements:
    if old not in s:
        raise SystemExit(f"host handoff anchor missing:\n{old}")
    s = s.replace(old, new, 1)

p.write_text(s)
