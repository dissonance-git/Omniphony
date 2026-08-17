from pathlib import Path
import re

PATH = Path("omniphony-renderer/renderer/src/spatial_renderer/speaker_stage.rs")
text = PATH.read_text(encoding="utf-8")

helper = '''
/// Scale-accumulate one band's speaker gains onto a single output frame.
///
/// Kept as a slice-to-slice loop rather than index arithmetic: it is the one
/// shape in this loop nest that carries no bounds check and no reduction, so a
/// compiler is free to widen it. Every ramp arm funnels through here.
#[inline(always)]
fn accumulate_band(out_frame: &mut [f32], gains: &[f32], sample: f32) {
    for (out, &gain) in out_frame.iter_mut().zip(gains.iter()) {
        *out += sample * gain;
    }
}

'''

if "fn accumulate_band(" not in text:
    marker = "impl SpeakerRenderStage {\n"
    if marker not in text:
        raise SystemExit("SpeakerRenderStage impl marker not found")
    text = text.replace(marker, helper + marker, 1)

pattern = re.compile(
    r"(?P<indent>[ \t]*)let out_base = sample_idx \* self\.num_speakers;\n"
    r"(?P=indent)for \(b, gains\) in band_gains\.iter\(\)\.enumerate\(\) \{\n"
    r"(?P=indent)    let s = (?P<sample>self\.crossover_band_scratch\[b\]\[sample_idx\]|split\.get\(b\));\n"
    r"(?P=indent)    for \(spk, &g\) in gains\.iter\(\)\.enumerate\(\) \{\n"
    r"(?P=indent)        output\[out_base \+ spk\] \+= s \* g;\n"
    r"(?P=indent)    \}\n"
    r"(?P=indent)\}"
)


def replace(m: re.Match[str]) -> str:
    i = m.group("indent")
    sample = m.group("sample")
    return (
        f"{i}let out_base = sample_idx * self.num_speakers;\n"
        f"{i}let out_frame = &mut output[out_base..out_base + self.num_speakers];\n"
        f"{i}for (b, gains) in band_gains.iter().enumerate() {{\n"
        f"{i}    accumulate_band(out_frame, gains, {sample});\n"
        f"{i}}}"
    )

text, replaced = pattern.subn(replace, text)
if replaced != 8:
    raise SystemExit(f"expected 8 hot accumulation loops, replaced {replaced}")

PATH.write_text(text, encoding="utf-8")
print(f"ported accumulate_band helper across {replaced} hot loops")
