# Real-material listening lane

`omniphony_listen.exe` is an internal diagnostic build for judging Omniphony with real source material rather than the short bundled reference fixture.

It is a development instrument, not the normal always-on Windows runtime.

## Usage

```powershell
.\omniphony_listen.exe "C:\path\to\test.wav"
```

The input must currently be uncompressed PCM/float WAV supported by `reference_bridge`. The bridge understands ordinary mono/stereo plus common 5.1, 7.1 and 7.1.4 channel counts.

The executable reads the WAV sample rate, forces channel content through Omniphony's spatial render path, renders to stereo, passes the result through the native realtime PCM seam, and plays it through the Windows default output device.

For CI/package validation without opening an audio endpoint:

```powershell
.\omniphony_listen.exe --render-only .\reference-demo\spatial-demo.wav
```

## Interpretation

This lane predates the current always-on protected-master Windows path and should not be treated as its authoritative architecture.

For stereo input, source truth remains limited:

```text
L/R mastered stereo
→ bounded inferred presentation
→ headphones
```

For real multichannel WAV input the source geometry is stronger because channel-position truth is already supplied. This makes 5.1, 7.1 and 7.1.4 material useful renderer diagnostics without implying that arbitrary stereo contains the same authored metadata.

## Current role

The normal Windows product path is `Omniphony.exe`, documented in `live-readme.txt` and the repository root `README.md`.

This file remains only to describe the older file-based diagnostic executable where it is built or used during renderer research.
