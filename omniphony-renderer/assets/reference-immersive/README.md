# Reference Immersive fixtures

These files are stable controls for the experimental [Reference Immersive](../../../docs/reference-immersive.md) research track.

They deliberately use the existing renderer only. Do not put experimental DSP into these configs: new algorithms should get their own config/flag so the controls remain reproducible.

## Controls

- `baseline-room.yaml` — embedded SAF/KEMAR HRTF, 3 m scale, current demo-style first-order reflections and short FDN room tail.
- `dry-binaural.yaml` — identical HRTF/scale/air-absorption policy with early reflections and late reverb disabled.

The pair isolates the room contribution without changing the source asset or HRTF family.

## Offline A/B with the bundled fixture

The bundled `assets/demo/spatial-demo.wav` is 7.1.4 test content and is intentionally used before ordinary stereo inference exists. The first job is to make the binaural renderer better, not to confound renderer changes with a new upmixer.

### Linux / macOS-style shell

From `omniphony-renderer/`:

```bash
cargo build -r -p reference_bridge
cargo build -r -p omniphony-renderer

./target/release/orender assets/demo/spatial-demo.wav \
  --bridge-path target/release/libreference_bridge.so \
  --enable-vbap --speaker-layout ../layouts/7.1.4.yaml \
  --config assets/reference-immersive/baseline-room.yaml \
  --output-backend file --output-file baseline-room.f32 --output-file-format raw-f32

./target/release/orender assets/demo/spatial-demo.wav \
  --bridge-path target/release/libreference_bridge.so \
  --enable-vbap --speaker-layout ../layouts/7.1.4.yaml \
  --config assets/reference-immersive/dry-binaural.yaml \
  --output-backend file --output-file dry-binaural.f32 --output-file-format raw-f32
```

On macOS the dynamic-library extension is normally `.dylib`; use the bridge artifact produced by the local build.

### Windows PowerShell

From `omniphony-renderer\`:

```powershell
cargo build -r -p reference_bridge
cargo build -r -p omniphony-renderer

.\target\release\orender.exe assets\demo\spatial-demo.wav `
  --bridge-path target\release\reference_bridge.dll `
  --enable-vbap --speaker-layout ..\layouts\7.1.4.yaml `
  --config assets\reference-immersive\baseline-room.yaml `
  --output-backend file --output-file baseline-room.f32 --output-file-format raw-f32

.\target\release\orender.exe assets\demo\spatial-demo.wav `
  --bridge-path target\release\reference_bridge.dll `
  --enable-vbap --speaker-layout ..\layouts\7.1.4.yaml `
  --config assets\reference-immersive\dry-binaural.yaml `
  --output-backend file --output-file dry-binaural.f32 --output-file-format raw-f32
```

The offline file backend does not require ASIO, PipeWire, or an audio device.

## Optional WAV wrapping

The fixture is 48 kHz and binaural output is two-channel float. If `ffmpeg` is installed, wrap the raw files without adding DSP:

```bash
ffmpeg -f f32le -ar 48000 -ac 2 -i baseline-room.f32 -c:a pcm_f32le baseline-room.wav
ffmpeg -f f32le -ar 48000 -ac 2 -i dry-binaural.f32 -c:a pcm_f32le dry-binaural.wav
```

Do not normalize one candidate independently from another. For subjective comparison, match playback loudness deliberately and retain enough headroom to avoid clipping.

## Experimental naming

When adding a renderer experiment, prefer an explicit name that identifies the changed variable, for example:

```text
directional-early-hrir.yaml
hrir-grid-5deg.yaml
hoa-ambient-order3.yaml
nearfield-proximity.yaml
```

Do not overwrite `baseline-room.yaml` to make a candidate look better. A candidate advances only if it beats the frozen control.

## Initial listening notes

Score changes separately rather than using one generic "immersive" rating:

```text
front externalization
rear discrimination
side precision
elevation
radial distance
source extent
source separation
source stability
room scale
ambient continuity
transient clarity
vocal/direct clarity
timbral fidelity
fatigue
bypass-collapse strength
```

The desired bypass result is a collapse in perceived acoustic volume, not a revelation that the unprocessed signal was clearer.
