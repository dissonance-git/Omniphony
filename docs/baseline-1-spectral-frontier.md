# Baseline 1 spectral frontier

Baseline 1 remains the canonical listening reference:

```text
03dac8bb454444b47353c39f65b58ce82617d731
```

The current post-baseline branch deliberately pushes beyond that reference while preserving a clean rollback ladder. The main unresolved perceptual defect motivating this work is intermittent **piercing / fatigue on bright transients**, especially cymbals and already-aggressive mixes.

The working hypothesis is no longer "the treble EQ is too high." The evidence supports a renderer-colour / coherence problem that manifests most audibly in the upper spectrum.

## Measured SAF/KEMAR diffuse fingerprint

`renderer/tests/hrtf_diffuse_spectrum.rs` measures the cos(elevation)-weighted direction-averaged power response of the interpolated embedded SAF/KEMAR HRTF grid.

Relative to 1 kHz, the measured profile is:

```text
500 Hz     -0.57 dB
1 kHz       0.00 dB
2 kHz      +1.74 dB
3 kHz      +4.23 dB
4 kHz      +7.27 dB
5 kHz      +7.35 dB
6 kHz      +7.02 dB
8 kHz      +5.29 dB
10 kHz     +7.53 dB
12 kHz     +4.32 dB
14 kHz     +4.06 dB
16 kHz     +3.88 dB
```

Sampled span: **8.10 dB**.

This is not evidence that the KEMAR HRTF should be flattened globally. Directional pinna structure is useful localization information. It does show that broadband HRTF energy normalization and frequency-dependent diffuse-field normalization are different jobs.

In the protected-master topology, the finished stereo recording is already present full-band. The additive spatial branch can therefore impose common HRTF colour a second time. That makes partial support-only compensation a plausible way to improve timbre without EQing or replacing the master.

## Literature and implementation convergence

### MPEG-H virtual-loudspeaker binaural rendering

Hyeong-Joo Moon and Young-Cheol Park, **Quality Enhancement of MPEG-H 3DA Binaural Rendering Using a Spectral Compensation Technique** (Electronics, 2022, DOI `10.3390/electronics11091491`) reports comb-filter spectral artifacts in virtual-loudspeaker binaural downmix caused by phase differences among binaural filters. Frequency-dependent spectral compensation improved subjective rendering quality.

The open `ittiam-systems/libmpegh` decoder is also useful structurally: its binaural filter design separates direct and diffuse BRIR/filter contributions rather than treating the entire binaural environment as one undifferentiated path.

### Diffuse-field HRTF equalization

Thomas McKenzie, Damian Murphy and Gavin Kearney, **Diffuse-Field Equalisation of Binaural Ambisonic Rendering** (Applied Sciences, 2018, DOI `10.3390/app8101956`) applies direction-independent diffuse-field equalization to improve high-frequency reproduction and timbre in binaural rendering while documenting the limits of the method.

Spatial Audio Framework implements the same core idea in `diffuseFieldEqualiseHRTFs`: compute the direction-weighted mean squared HRTF magnitude for each band/ear, take its square root, then divide each directional HRTF by that common response.

SAF's binaural examples also expose HRTF preprocessing, MagLS and diffuse matching rather than relying on raw measured filters alone.

### HRTF gain normalization

Valve Steam Audio independently treats HRTF gain management as a renderer responsibility. It exposes:

- HRTF volume gain;
- RMS normalization across HRTF directions;
- a reference-loudness calculation in its HRTF database.

This supports keeping renderer gain/colour management separate from programme EQ.

### Coherence and transient preservation

Jonathan B. Moore and Adam J. Hill, **Dynamic Diffuse Signal Processing for Sound Reinforcement and Reproduction** (JAES, 2018, DOI `10.17743/JAES.2018.0054`) documents how high inter-channel coherence can create comb-filter magnitude variation and discusses decorrelation with explicit attention to transient preservation.

For Omniphony this argues against broad decorrelation of music. If decorrelation becomes necessary later, it should target redundant diffuse/reflection residue while keeping the protected master and important transients authoritative.

## Current post-baseline mechanism stack

### 1. Cascaded renderer remains the spatial core

```text
derived 7.1.4 support
→ Omniphony virtual-speaker renderer
→ virtual room
→ SAF/KEMAR HRTF + ITD
→ binaural support
```

Direct binaural remains a generic reference path. Cascaded mode is the music architecture because physical listening found it significantly more continuous and bubble-like.

### 2. Larger frontier geometry

Current music frontier:

```text
metric scale                 7.25 m / ADM unit
speaker effect-space width   15.5 m
front reach                  13.0 m
rear reach                   10.5 m
upper reach                  14.5 m
TFL/TFR z                    1.65
TBL/TBR z                    1.50
side x                       ±1.15
source spread floor          0.09
source spread max            0.36
phantom extraction           0.28 broadband
reflection room              17 × 27 × 15.5 m
reflection level             0.38
late field                   0.035 / 0.17 s / 28 ms
```

Scale should come from geometry, timing, HRTF/ITD, early-field structure and source extent rather than a louder late reverb tail.

### 3. Reflection spectral realism

The historical early-reflection bank used delayed, distance-scaled broadband copies. That was spatially useful but made a huge room behave too much like six spectrally perfect mirrors.

The post-baseline reflection path now adds broad frequency-dependent high-frequency loss based on:

- a generic wall HF-retention term inspired by mature material models;
- additional upper-band attenuation for reflection-only propagation distance;
- unchanged low/mid reflection timing and distance structure.

This is deliberately a physical-room mechanism rather than a programme treble shelf.

### 4. High-band coherence cleanup

The stereo evidence mapper previously injected a small correlated copy of the stereo mid directly into top-front support at every frequency. Above 5 kHz that created a particularly suspicious topology:

```text
protected dry center transient
+
correlated HRTF-rendered overhead copy
```

The direct top-front mid shortcut is now disabled above 5 kHz. Height remains available through relational / lateral / diffuse evidence and the enlarged geometry.

### 5. Partial SAF diffuse-field compensation

`renderer/src/binaural/diffuse_compensation.rs` implements a static, causal, identical-per-ear partial inverse of the measured common SAF/KEMAR colour.

First profile:

```text
4.8 kHz broad peak   -3.40 dB
10 kHz broad peak    -3.00 dB
12 kHz high shelf    -1.20 dB
```

This is intentionally **not** a full inverse of the measured +7 dB regions. The first experiment removes only part of the common colour so directional HRTF residual structure remains available for localization.

The compensation is generic-cascade **OFF by default**. The current music config explicitly opts in:

```yaml
render:
  binaural:
    mode: cascaded
    hrir_source: saf
    spectral_compensation: saf_partial
```

The renderer also checks that the active HRTF source is SAF/KEMAR before applying the SAF profile.

This preserves the generic invariant that an unconfigured cascade at an exact virtual-speaker direction can collapse numerically to the direct binaural renderer.

### 6. Reclaimed playback level

The Windows host fixed final gain is now:

```text
0.90 linear
≈ -0.9 dB
```

Baseline 1 used `0.72` (≈ -2.85 dB), so the new frontier reclaims approximately **1.94 dB** of whole-program level. ON and OFF use the same static gain.

No compressor, limiter, AGC or content-dependent loudness stage was added.

Reduced static headroom means physical peak/clipping validation is now important.

## Current rollback ladder

```text
03dac8bb  Baseline 1
0471501e  larger geometry + reflection HF realism
17ad1f20  fixed output level 0.72 → 0.90
e360c7fc  remove correlated >5 kHz top-front mid copy
cc186861  SAF diffuse-spectrum measurement
42fba777  partial SAF diffuse compensation implementation
7acde068  explicit SAF-only cascade compensation gating
```

Baseline 1 remains the reference even as later layers accumulate.

## Listening questions for the current frontier

1. Is whole-program volume now usable without nearly maxing the amplifier?
2. Is the sphere clearly larger than Baseline 1?
3. Are cymbals and the bright transients in `Jam` less needle-like?
4. Does the correction sound **calmer**, not merely darker?
5. Is height still strong after removing the correlated high-band top-front shortcut?
6. Did bass pressure, kick weight and drum body remain unchanged?
7. Are panned percussion and tom rolls still mobile through the sphere?
8. Is there any new blur, hallway colour or late-field fog?
9. Does `0.90` final gain cause real clipping on dense masters?

## If piercing remains

Do not immediately deepen the static cuts.

Next candidates, in order:

1. measure the **complete cascaded support transfer function**, not only the isolated HRTF diffuse response;
2. derive a smoother minimum-phase FIR / frequency-domain compensation from that measured cascade response;
3. test transient-preserving decorrelation only on diffuse/reflection residue;
4. investigate frequency-dependent virtual-speaker spread so upper support does not create unnecessary coherent HRTF copies;
5. only after renderer spectral stability, perform systematic HRTF/SOFA selection or personalization.

The master remains outside every one of those experiments.
