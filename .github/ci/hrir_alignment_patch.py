from pathlib import Path

MEASURED = Path("omniphony-renderer/renderer/src/binaural/measured.rs")


def main() -> None:
    text = MEASURED.read_text(encoding="utf-8")
    old = '''fn align_into(ir: &[f32], out: &mut [f32; HRIR_LEN]) {
    if ir.is_empty() {
        out.fill(0.0);
        return;
    }
    let peak = ir.iter().fold(0.0f32, |m, &x| m.max(x.abs())).max(1e-12);
    let thresh = ONSET_FRAC * peak;
    let onset = ir.iter().position(|&x| x.abs() >= thresh).unwrap_or(0);
    let start = onset.saturating_sub(PRE_SAMPLES);
    for (k, slot) in out.iter_mut().enumerate() {
        *slot = ir.get(start + k).copied().unwrap_or(0.0);
    }
}
'''
    new = '''fn align_into(ir: &[f32], out: &mut [f32; HRIR_LEN]) {
    if ir.is_empty() {
        out.fill(0.0);
        return;
    }
    let peak = ir.iter().fold(0.0f32, |m, &x| m.max(x.abs())).max(1e-12);
    let thresh = ONSET_FRAC * peak;
    let onset = ir.iter().position(|&x| x.abs() >= thresh).unwrap_or(0);

    // Put every detected direct arrival at exactly PRE_SAMPLES in the output.
    // `saturating_sub` is not sufficient here: when an onset occurs before the
    // desired pre-roll (for example L=7, R=9), clamping the earlier ear to source
    // index 0 preserves part of the original interaural bulk delay. Omniphony
    // supplies that delay analytically, so the measured filters must carry only
    // their spectral/phase structure. Negative source indices are therefore
    // represented by explicit leading zero padding rather than by shifting the
    // output onset.
    out.fill(0.0);
    for (k, slot) in out.iter_mut().enumerate() {
        let src = onset as isize + k as isize - PRE_SAMPLES as isize;
        if src >= 0 {
            *slot = ir.get(src as usize).copied().unwrap_or(0.0);
        }
    }
}
'''
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"measured HRIR align_into: expected one source fragment, found {count}")
    MEASURED.write_text(text.replace(old, new, 1), encoding="utf-8")
    print("patched measured HRIR direct-arrival alignment with left zero padding")


if __name__ == "__main__":
    main()
