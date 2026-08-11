# Frequency-evidence stereo music path

Status: **active protected stereo sound foundation**.

The first listening pass has now validated this direction strongly enough that future stereo sound work should build from it rather than from the earlier full-wet or side-only experiments.

## Why this exists

Early live tests established three important facts:

1. sending a finished stereo master wholesale through the generic virtual-speaker HRTF path damaged timbre, bass and useful stereo definition;
2. preserving the original stereo master as the direct path removed those failures;
3. a support field derived only from `(L-R)/2` remained effectively inaudible even when nominal support gain was made very large.

The frequency-evidence path therefore uses richer, frequency-dependent stereo evidence while keeping the master authoritative.

## Current architecture

```text
FINISHED STEREO MASTER
        |
        +---------------------------------------> protected direct path
        |
        +-> 1024-sample FFT analysis
              |
              +-> L/R magnitude
              +-> L/R phase
              +-> pan / phase coherence
              +-> directness / diffuseness
              +-> true complex M/S
              +-> persistence / stability
                        |
                        v
                 scene inference
                        |
          +-------------+-------------+
          |             |             |
       broad         lateral       diffuse
       support        support       support
          |             |             |
          +------ causal multiband extraction
                        |
                        v
            derived logical 7.1 support bed

            L/R   = broad extent
            C     = silence
            LFE   = silence
            Ls/Rs = lateral/object-like evidence
            Lb/Rb = diffuse/field-like evidence
                        |
                        v
               upstream Omniphony
             HRTF / ITD / binaural
                        |
                        v
                 stereo support
                        |
      protected direct master + support
                        |
                        v
                    headphones
```

Physical output remains ordinary stereo headphones. The 7.1 structure is an internal logical support field used to give the inherited Omniphony binaural renderer differentiated material.

## Portable-core ownership

The Windows host must not own the stereo reasoning.

The active implementation lives in:

```text
renderer/src/stereo_inference.rs
renderer/src/scene_inference.rs
renderer/src/music_field.rs
```

The Windows worker only owns:

```text
WASAPI capture
-> call portable MusicFieldProcessor
-> feed derived field into Omniphony engine
-> align protected master to renderer latency
-> combine
-> physical playback
```

## Evidence laws

### Center authority

Coherent frontal anchors should remain in the direct stereo master. The support processor suppresses field promotion when scene evidence classifies energy as a frontal/foundation anchor.

### Bass foundation

The support field begins above approximately 220 Hz. C and LFE are intentionally silent. Low-frequency weight, timing and pressure stay in the mastered direct path unless later evidence proves a safer mechanism.

### Frequency-dependent evidence

A 1024-sample FFT is analysis-only. It measures real magnitude and phase relationships per bin. It does not synthesize the audible field and therefore does not add STFT reconstruction latency.

Scene controls are aggregated into three support bands:

```text
220-1200 Hz
1200-5000 Hz
5000 Hz-Nyquist
```

The audible field is extracted with a causal parallel low-pass/difference bank whose bands sum algebraically back to the original signal before weighting.

### Analysis is not authored truth

Stereo evidence can justify broad/lateral/diffuse support. It does not prove that a source was authored behind or above the listener.

Rearward or vertical field placement remains a presentation decision, not a claim about hidden source metadata.

## Current logical field geometry

```text
Broad L/R
-> front-side extent

Lateral Ls/Rs
-> side / slightly rearward

Diffuse Lb/Rb
-> strongest rearward extent
```

No early reflections, late reverb or air absorption are enabled in this experiment. The current spatial win therefore comes from evidence, geometry, HRTF and ITD rather than an audible room tail.

## Protected-master mix law

The direct master gets first claim on headroom.

```text
wanted = rendered_support * support_gain
available positive/negative headroom = distance from direct sample to +/-1
actual support = clamp(wanted to remaining headroom)
output = direct + actual support
```

The master is not attenuated merely to make room for the spatial effect.

## Instrumentation

The Windows worker reports roughly every five seconds:

```text
direct RMS / peak
derived field RMS / peak
Omniphony-rendered support RMS / peak
actually-added RMS / peak
added/direct dB ratio

anchor score
broad score
lateral score
diffuse score
lateral pan
side fraction
```

Use these meters before treating an inaudible result as a taste problem.

## Current listening baseline

For stereo-music development, the temporary Hi-Fi Cable endpoint should be configured as **Stereo / 2.0**.

A 7.1 Windows endpoint feeding the stereo process-loopback prototype reduced playback level. Returning the endpoint to Stereo restored normal level. Treat this as a prototype transport/gain finding.

The first clean frequency-evidence listening result is now:

```text
raw stereo clarity remains
music stays intact
sound is clearly enhanced
apparent scene is bigger than the incumbent HeSuVi/DTS chain
behind-head placement is convincing
percussion can feel genuinely behind the listener rather than merely lateral
```

This validates the architecture strongly enough to protect it.

The current weakness is **rear-heavy field geometry**:

```text
rear support is too dominant
front externalization is weaker than rear externalization
side/front-side wrap is incomplete
height is largely absent
radial near/mid/far layering is underdeveloped
continuous 360° shell is incomplete
```

Therefore the next sound work is **not more rear gain**.

It is:

```text
rebalance rear evidence
-> strengthen front externalization
-> increase side/front-side continuity
-> form a continuous 360° shell
-> add conservative upper/lower field support
-> add radial depth
-> improve source extent / ambient continuity
-> preserve the current clarity floor
```

## Listening success criterion from here

The current build is the reference foundation.

A successor wins only if it preserves:

```text
bass/body
vocal solidity
center authority
transient definition
stereo identity
rhythmic precision
raw clarity
comfort
```

while improving:

```text
front/side/rear balance
360° wrap
height
below-listener plausibility
radial depth
source extent
ambient continuity
listener envelopment
```

The desired mature bypass reaction remains:

> **The original music is still fully present, but the acoustic world collapses when Omniphony is turned off.**
