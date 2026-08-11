from pathlib import Path

p = Path('omniphony-renderer/runtime_control/src/persist.rs')
s = p.read_text()

old = '''    render.binaural = Some(renderer::config::BinauralConfig {
        output_mode: Some(live.binaural.output_mode.as_str().to_string()),'''
new = '''    // Spectral compensation is a static renderer policy rather than a live
    // control, so preserve the loaded choice when saving unrelated binaural
    // settings instead of silently dropping the music topology's opt-in.
    let spectral_compensation = render
        .binaural
        .as_ref()
        .and_then(|binaural| binaural.spectral_compensation.clone());
    render.binaural = Some(renderer::config::BinauralConfig {
        output_mode: Some(live.binaural.output_mode.as_str().to_string()),'''
if old not in s:
    raise SystemExit('BinauralConfig initializer anchor missing')
s = s.replace(old, new, 1)

old = '''        hrir_source: Some(hrir_source),
        hrtf_sofa_path,
        head_tracking: Some(renderer::config::HeadTrackingConfig {'''
new = '''        hrir_source: Some(hrir_source),
        hrtf_sofa_path,
        spectral_compensation,
        head_tracking: Some(renderer::config::HeadTrackingConfig {'''
if old not in s:
    raise SystemExit('spectral field insertion anchor missing')
s = s.replace(old, new, 1)

p.write_text(s)
