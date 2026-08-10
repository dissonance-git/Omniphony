# Binaural rendering baselines

These files are stable controls for the [headphone rendering research](../../../docs/headphone-rendering-research.md) in this fork.

They deliberately keep experimental DSP out of the reference path. New algorithms should use their own config/flag so a research change cannot silently redefine what "good Omniphony" means.

## Perceptual ancestor

The upstream Omniphony website describes its headphone demo as the **stock engine with KEMAR HRTF and early reflections on**. That hosted demo is the perceptual ancestor for this fork: if a fork candidate is technically more elaborate but sounds worse than that reference at matched loudness, the candidate does not earn the default.

`upstream-demo-reference.yaml` is the smallest local configuration that expresses only the published ingredients:

- binaural output;
- embedded SAF/KEMAR HRTF;
- early reflections enabled;
- late reverb explicitly disabled;
- all other values inherited from stock renderer defaults.

The website does not pin the exact commit or rendering command used for its hosted audio, so this config is a **reproducible local approximation of the published stock-demo contract**, not a claim of byte identity with the hosted file. The hosted demo remains a listening oracle when available.

## Controls

- `upstream-demo-reference.yaml` — closest local expression of the published upstream demo contract: stock defaults, SAF/KEMAR, early reflections on, late reverb off.
- `baseline-room.yaml` — fork room-assisted comparison: 3 m scale, first-order reflections and a short FDN late tail. This is useful, but it is **not** allowed to redefine the upstream perceptual ancestor merely because it is richer.
- `dry-binaural.yaml` — same fork HRTF/scale/air-absorption policy as `baseline-room.yaml` with early reflections and late reverb disabled.

`baseline-room.yaml` versus `dry-binaural.yaml` isolates the fork's room contribution without changing source content or HRTF family. `upstream-demo-reference.yaml` asks the more important product question: did the fork actually improve on the already-good stock effect?

## Offline A/B with the bundled fixture

The bundled `assets/demo/spatial-demo.wav` is 7.1.4 test content. It is intentionally used before ordinary stereo scene inference exists so renderer changes are not confounded with a new analysis/upmix stage.

### Linux / macOS-style shell

From `omniphony-renderer/`:

```bash
cargo build -r -p reference_bridge
cargo build -r -p omniphony-renderer

./target/release/orender assets/demo/spatial-demo.wav \
  --bridge-path target/release/libreference_bridge.so \
  --enable-vbap --speaker-layout ../layouts/7.1.4.yaml \
  --config assets/binaural-baselines/upstream-demo-reference.yaml \
  --output-backend file --output-file upstream-demo-reference.f32 --output-file-format raw-f32

./target/release/orender assets/demo/spatial-demo.wav \
  --bridge-path target/release/libreference_bridge.so \
  --enable-vbap --speaker-layout ../layouts/7.1.4.yaml \
  --config assets/binaural-baselines/baseline-room.yaml \
  --output-backend file --output-file baseline-room.f32 --output-file-format raw-f32

./target/release/orender assets/demo/spatial-demo.wav \
  --bridge-path target/release/libreference_bridge.so \
  --enable-vbap --speaker-layout ../layouts/7.1.4.yaml \
  --config assets/binaural-baselines/dry-binaural.yaml \
  --output-backend file --output-file dry-binaural.f32 --output-file-format raw-f32
```

On macOS use the bridge dynamic-library artifact produced by the local build.

### Windows PowerShell

From `omniphony-renderer\`:

```powershell
cargo build -r -p reference_bridge
cargo build -r -p omniphony-renderer

.\target\release\orender.exe assets\demo\spatial-demo.wav `
  --bridge-path target\release\reference_bridge.dll `
  --enable-vbap --speaker-layout ..\layouts\7.1.4.yaml `
  --config assets\binaural-baselines\upstream-demo-reference.yaml `
  --output-backend file --output-file upstream-demo-reference.f32 --output-file-format raw-f32

.\target\release\orender.exe assets\demo\spatial-demo.wav `
  --bridge-path target\release\reference_bridge.dll `
  --enable-vbap --speaker-layout ..\layouts\7.1.4.yaml `
  --config assets\binaural-baselines\baseline-room.yaml `
  --output-backend file --output-file baseline-room.f32 --output-file-format raw-f32

.\target\release\orender.exe assets\demo\spatial-demo.wav `
  --bridge-path target\release\reference_bridge.dll `
  --enable-vbap --speaker-layout ..\layouts\7.1.4.yaml `
  --config assets\binaural-baselines\dry-binaural.yaml `
  --output-backend file --output-file dry-binaural.f32 --output-file-format raw-f32
```

The offline file backend does not require ASIO, PipeWire, or an audio device.

## Optional WAV wrapping

The fixture is 48 kHz and binaural output is two-channel float. If `ffmpeg` is installed, wrap the raw files without adding DSP:

```bash
ffmpeg -f f32le -ar 48000 -ac 2 -i upstream-demo-reference.f32 -c:a pcm_f32le upstream-demo-reference.wav
ffmpeg -f f32le -ar 48000 -ac 2 -i baseline-room.f32 -c:a pcm_f32le baseline-room.wav
ffmpeg -f f32le -ar 48000 -ac 2 -i dry-binaural.f32 -c:a pcm_f32le dry-binaural.wav
```

Do not normalize one candidate independently from another. Match playback loudness deliberately and retain enough headroom to avoid clipping.

## Experimental naming

Prefer explicit technical names that identify the variable being changed, for example:

```text
directional-early-hrir.yaml
hrir-grid-5deg.yaml
hoa-ambient-order3.yaml
nearfield-proximity.yaml
```

Do not overwrite `upstream-demo-reference.yaml` or an accepted control to make a candidate look better. A candidate advances only if it preserves the musical identity of the control and earns its audible differences.

## Listening dimensions

Score dimensions separately rather than collapsing them into a single generic spatial rating:

```text
front externalization
rear discrimination
side precision
elevation
radial distance
apparent source width
listener envelopment
source extent
source separation
source stability
room presence / scale
ambient continuity
transient clarity
vocal/direct clarity
timbral fidelity
bass stability
fatigue
bypass-collapse strength
```

The desired bypass result is a collapse in perceived acoustic volume, not a revelation that the control was clearer, punchier, more tonally convincing or more musically coherent.
