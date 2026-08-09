# Binaural rendering baselines

These files are stable controls for the [headphone rendering research](../../../docs/headphone-rendering-research.md) in this fork.

They deliberately exercise established renderer behavior only. Do not put experimental DSP into these configs. New algorithms should use their own config/flag so these controls remain reproducible.

## Controls

- `baseline-room.yaml` — embedded SAF/KEMAR HRTF, 3 m scale, current demo-style first-order reflections and short FDN room tail.
- `dry-binaural.yaml` — identical HRTF/scale/air-absorption policy with early reflections and late reverb disabled.

The pair isolates the room contribution without changing source content or HRTF family.

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

Do not overwrite `baseline-room.yaml` to make a candidate look better. A candidate advances only if it beats the frozen control.

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

The desired bypass result is a collapse in perceived acoustic volume, not a revelation that the control was clearer.
