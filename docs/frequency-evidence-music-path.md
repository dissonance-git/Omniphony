# Frequency-evidence stereo music path

Status: **protected stereo fidelity foundation with an active stronger 7.1.4-shell successor experiment**.

Longer listening has validated this direction strongly enough that future stereo sound work should build from it rather than from the earlier full-wet or side-only experiments. It has **not** yet validated a large 360° win over HeSuVi.

## Why this exists

Early live tests established three important facts:

1. sending a finished stereo master wholesale through the generic virtual-speaker HRTF path damaged timbre, bass and useful stereo definition;
2. preserving the original stereo master as the direct path removed those failures;
3. a support field derived only from `(L-R)/2` remained effectively inaudible even when nominal support gain was made very large.

The frequency-evidence path therefore uses richer, frequency-dependent stereo evidence while keeping the master authoritative.

## Protected architecture

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
              derived support field
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

Physical output remains ordinary stereo headphones. Internal multichannel lanes are only differentiated support material for the inherited Omniphony renderer.

## Active 7.1.4 shell experiment

The first frequency-evidence build used a sparse logical 7.1 mapping:

```text
L/R   = broad extent
C     = silence
LFE   = silence
Ls/Rs = lateral/object-like evidence
Lb/Rb = diffuse/field-like evidence
```

That mapping preserved fidelity but longer listening found the enhancement only mild. Diffuse evidence was also structurally concentrated in the rear pair.

The active successor expands the field to canonical logical **7.1.4**:

```text
L/R         broad front/front-side extent
C           silence
LFE         silence
Ls/Rs       strongest lateral wrap
Lb/Rb       restrained rear continuation
Tfl/Tfr     front-height extent
Tbl/Tbr     rear-height / upper diffuse continuation
```

Evidence overlaps neighbouring regions rather than assigning each class to exactly one speaker pair. The purpose is a shell rather than isolated virtual-speaker islands.

The host support coefficient is also raised to full derived-field strength for this experiment. That means **100% of the derived support**, not 100% wet replacement: the protected stereo master remains explicitly present and still owns headroom.

No early reflections, late reverb or air absorption are enabled. This pass tests geometry/evidence/HRTF/ITD before adding room effects.

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

The support field begins above approximately 220 Hz. C and LFE remain intentionally silent. Low-frequency weight, timing and pressure stay in the mastered direct path unless later evidence proves a safer mechanism.

### Frequency-dependent evidence

A 1024-sample FFT is analysis-only. It measures real magnitude and phase relationships per bin. It does not synthesize the audible field and therefore does not add STFT reconstruction latency.

Scene controls are aggregated into three support bands:

```text
220-1200 Hz
1200-5000 Hz
5000 Hz-Nyquist
```

The audible field is extracted with a causal parallel low-pass/difference bank whose bands sum algebraically back to the original signal before weighting.

### Height is permission, not recovered truth

The active shell gives already-spatial broad/lateral/diffuse evidence some vertical extent. Frequency changes the *permission prior* but never becomes a height command by itself.

Forbidden shortcut:

```text
high frequency = above
low frequency  = below
```

A coherent direct anchor remains protected regardless of register.

### Analysis is not authored truth

Stereo evidence can justify broad/lateral/diffuse support. It does not prove that a source was authored behind or above the listener.

Rearward or vertical field placement remains a presentation decision, not a claim about hidden source metadata.

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
height score
lateral pan
side fraction
```

Use these meters before treating an inaudible result as a taste problem.

## Current listening baseline

For stereo-music development, the temporary Hi-Fi Cable endpoint should be configured as **Stereo / 2.0**.

A 7.1 Windows endpoint feeding the stereo process-loopback prototype reduced playback level. Returning the endpoint to Stereo restored normal level. Treat this as a prototype transport/gain finding.

The repeated-listening result for the protected frequency-evidence build is:

```text
raw stereo clarity remains
music stays intact
no obvious tinny / hallway regression
ON is mildly enhanced versus OFF
```

The first few minutes produced stronger impressions of rear placement and apparent size, but those were partly confounded by reacclimating to raw stereo after disabling HeSuVi. They are **not frozen comparison claims**.

What is protected is therefore the architecture's fidelity behavior, not a claim that it already beats HeSuVi spatially.

## Sound frontier

The next sound work is:

```text
make ON unmistakably larger than OFF
-> preserve raw clarity / bass / center / transients
-> distribute support across front-side / side / rear instead of rear-only concentration
-> add conservative upper-shell participation
-> improve front externalization
-> add radial depth
-> improve source extent / ambient continuity
-> later restore / exceed incumbent energy and punch
```

The active 7.1.4-shell build is the first major push on the first three items.

## Listening success criterion from here

The protected P0.4 fidelity floor remains the reference.

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
immediate ON/OFF audibility
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
