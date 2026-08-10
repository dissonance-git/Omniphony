# Binaural rendering baselines

These files are stable controls for the headphone-rendering work in this fork.

The most important rule is simple:

> **Do not let an experiment silently redefine what the upstream Omniphony percept was.**

New algorithms, stereo-presentation mechanisms, calibration, libaural-derived ideas, and room changes use separate controls until listening proves they should graduate.

## Upstream perceptual ancestor

`upstream-demo-reference.yaml` now tracks the **actual bundled headphone-demo configuration in `mgth/Omniphony`**, rather than the earlier inferred approximation.

The upstream `omniphony-renderer/assets/demo/demo.yaml` uses:

```text
binaural output
SAF / KEMAR measured HRTF
3.0 m per layout unit
first-order reflections enabled at 0.4
short late reverb enabled at 0.2
RT60 = 0.3 s
```

That matters. The earlier local approximation used a 1 m scale, enabled reflections without the demo's explicit level, and disabled late reverb. It was therefore too dry and too small to be treated as the protected upstream demo sound.

The corrected local control is:

```yaml
render:
  binaural:
    output_mode: binaural
    hrir_source: saf
    unit_scale_m: 3.0
    reflections:
      enabled: true
      level: 0.4
    reverb:
      enabled: true
      level: 0.2
      rt60_s: 0.3
```

This is still a perceptual/reference control, not a claim that every future upstream commit or hosted recording is byte-identical. The source of truth is the upstream bundled demo configuration at the inspected commit plus direct listening.

## Controls

- `upstream-demo-reference.yaml` — protected upstream-demo-style perceptual ancestor.
- `baseline-room.yaml` — fork room-assisted comparison. It may overlap the upstream demo's room ingredients, but remains a separate fork comparison surface.
- `dry-binaural.yaml` — room-disabled isolation control.

Do not normalize candidates independently. Loudness-match comparisons deliberately.

## What the first live Windows listen taught us

The first arbitrary-music path proved that Windows audio can reach the renderer and the physical headphones, but it was **not** a trustworthy renderer-quality verdict until routing was cleaned up.

Observed sequence:

```text
initial live listen
→ tinny / hallway-like / weak bubble
→ residual wet sound after OFF

then
HeSuVi disabled
ASIO Bridge / old forwarding stopped
clean route-only bypass added
→ OFF became clean
→ ON remained severely thin / bass-light / weakly spatial
```

Therefore:

```text
transport viability        = proven
old duplicate-path leakage = substantially isolated
wet-path fidelity problem  = real and still open
```

The remaining wet-path problem must be debugged before adding artistic complexity.

## Stereo-music priority

Ordinary stereo music is the main product use case. A successful surround renderer is not enough.

The next controlled listening sequence is:

```text
clean stereo source
→ no legacy foobar surround upmix
→ Omniphony
→ binaural stereo output
```

This isolates the stereo presentation/fidelity problem from the old HeSuVi-era 5.1 upmix.

Native 5.1/7.1/height/object sources remain first-class richer inputs later. They should retain their source truth rather than being collapsed and rediscovered.

## LFE versus stereo foundation

Upstream Omniphony's binaural renderer gives an authored LFE channel a special dry/full-range equal-ear route rather than HRTF-convolving it. Ordinary stereo has no authored LFE channel, so its low-frequency foundation currently receives no equivalent protection in the generic channel-bed path.

That is a useful clue, not permission to invent a fake LFE blindly.

Any stereo foundation mechanism must prove that it:

- restores/retains bass body and timing;
- does not collapse the spatial benefit;
- does not create phase/crossover artifacts;
- preserves stereo musical identity;
- remains a portable Omniphony mechanism rather than Windows-only DSP.

## Offline A/B

The bundled `assets/demo/spatial-demo.wav` remains useful for known-scene renderer validation because it separates renderer quality from stereo inference.

Example from `omniphony-renderer/`:

```bash
cargo build -r -p reference_bridge
cargo build -r -p omniphony-renderer

./target/release/orender assets/demo/spatial-demo.wav \
  --bridge-path target/release/libreference_bridge.so \
  --enable-vbap --speaker-layout ../layouts/7.1.4.yaml \
  --config assets/binaural-baselines/upstream-demo-reference.yaml \
  --output-backend file --output-file upstream-demo-reference.f32 --output-file-format raw-f32
```

On Windows use the corresponding `.exe` and `reference_bridge.dll` artifacts.

## Listening dimensions

Score dimensions independently:

```text
front externalization
rear discrimination
side precision
elevation
radial distance
listener envelopment
source extent
source stability
room presence / scale
ambient continuity
transient clarity
vocal/direct solidity
timbral fidelity
bass body / timing
groove
microdetail
dynamics
fatigue
bypass-collapse strength
```

The desired bypass result is a collapse in perceived acoustic volume, not the discovery that bypass restores bass, clarity, punch, tonal correctness, or musical coherence.
