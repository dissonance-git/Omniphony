# Real-material listening lane

`omniphony_listen.exe` is the first internal build meant for judging Omniphony with real source material rather than the short bundled reference fixture.

It is intentionally still a development instrument, not the final system-wide product.

## Usage

```powershell
.\omniphony_listen.exe "C:\path\to\test.wav"
```

The input must currently be uncompressed PCM/float WAV supported by `reference_bridge`. The bridge understands ordinary mono/stereo plus common 5.1, 7.1 and 7.1.4 channel counts.

The executable reads the WAV sample rate, forces channel content through Omniphony's spatial render path, uses the protected upstream-style binaural configuration, renders to stereo, passes the result through the native realtime PCM seam, and plays it through the Windows default output device.

For CI/package validation without opening an audio endpoint:

```powershell
.\omniphony_listen.exe --render-only .\reference-demo\spatial-demo.wav
```

## Interpretation

For stereo input this milestone is deliberately conservative:

```text
L/R mastered stereo
→ known front-left / front-right bed semantics
→ protected Omniphony binaural renderer
→ headphones
```

It does **not** yet claim intelligent full-sphere reconstruction from arbitrary stereo. That later layer must earn each presentation decision.

For real multichannel WAV input the geometry is more informative because the source already supplies channel-position truth. This makes 5.1, 7.1 and 7.1.4 material useful early tests of the mature product direction.

## Current limitation

The complete file is rendered before playback, so long files may pause and consume substantial memory before sound starts. That is acceptable for this listening milestone.

The next transport/core step is incremental rendering into a bounded producer/output path so the same engine can operate continuously instead of pre-rendering the whole file.
