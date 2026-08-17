from pathlib import Path

PATH = Path("omniphony-renderer/renderer/src/spatial_vbap/panner/native_validation.rs")
text = PATH.read_text(encoding="utf-8")

ignore = '#[ignore = "engine misses this: MDAP spread does not conserve energy — 5.1 spread=0.25 is -3.0090 dB at az=-75.1 el=67.5, target ±0.25 dB. Distinct from pole coverage, which is now fixed. Tracked deferral, see docs/dsp-validation-report.md"]\n'
if ignore not in text:
    raise SystemExit("expected MDAP deferral marker not found")
text = text.replace(ignore, "", 1)

old = "/// The wide matrix: every shipped layout at a denser lattice, plus spread.\n"
new = (
    "/// The wide matrix: every shipped layout at a denser lattice, plus spread.\n"
    "/// The former MDAP spread-energy deferral is intentionally live here: the\n"
    "/// current virtual-pole/downmix path now preserves the ±0.25 dB contract.\n"
)
text = text.replace(old, new, 1)
PATH.write_text(text, encoding="utf-8")
print("promoted wide MDAP energy sweep from deferral to live gate")
