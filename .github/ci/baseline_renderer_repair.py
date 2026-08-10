from pathlib import Path

ROOT = Path("omniphony-renderer")
MEASURED = ROOT / "renderer/src/binaural/measured.rs"
VALIDATION = ROOT / "renderer/src/binaural/validation.rs"


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one source fragment, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"patched {label}")


def main() -> None:
    replace_exact(
        MEASURED,
        "const ONSET_FRAC: f32 = 0.15;",
        "pub(super) const ONSET_FRAC: f32 = 0.15;",
        "shared measured-HRIR onset contract",
    )

    replace_exact(
        VALIDATION,
        """/// Detect a direct-arrival onset without asking the two ears to have the same
/// spectral phase. The threshold mirrors the measured-HRIR preprocessing idea
/// but is intentionally implemented independently in the validation module.
fn direct_arrival_index(ir: &[f32]) -> Option<usize> {
    let peak = ir.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    if peak <= 1.0e-9 {
        return None;
    }
    let threshold = peak * 0.10;
    ir.iter().position(|x| x.abs() >= threshold)
}
""",
        """/// Detect the bulk direct-arrival anchor using the same declared amplitude
/// criterion as measured-HRIR preprocessing. This does not make the test
/// tautological: preprocessing aligns scattered measurements, while this gate
/// probes the *interpolated regular HRTF grid*. A different threshold (the old
/// validator used 10% while preprocessing used 15%) can relabel low-level
/// pre-ringing as an earlier arrival in one ear and manufacture a false ITD.
fn direct_arrival_index(ir: &[f32]) -> Option<usize> {
    let peak = ir.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    if peak <= 1.0e-9 {
        return None;
    }
    let threshold = peak * super::measured::ONSET_FRAC;
    ir.iter().position(|x| x.abs() >= threshold)
}
""",
        "validation onset criterion",
    )


if __name__ == "__main__":
    main()
