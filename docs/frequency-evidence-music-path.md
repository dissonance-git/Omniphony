# Frequency-evidence stereo music path

Status: **protected stereo fidelity foundation with an active P0.6 anterior/height 7.1.4-shell experiment**.

Longer listening has validated this direction strongly enough that future stereo sound work should build from it rather than from the earlier full-wet or side-only experiments. P0.5 moved the sound close to the incumbent HeSuVi experience while preserving the striking clarity of the protected master, but its acoustic volume is still too small and rear-biased to count as the desired full bubble.

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

## P0.5 listening result

P0.5 expanded the first frequency-evidence build from sparse logical 7.1 into an overlapping canonical logical **7.1.4** shell:

```text
L/R         broad front/front-side extent
C           silence
LFE         silence
Ls/Rs       strongest lateral wrap
Lb/Rb       restrained rear continuation
Tfl/Tfr     front-height extent
Tbl/Tbr     rear-height / upper diffuse continuation
```

The host support coefficient was raised to full derived-field strength. That means **100% of the derived support**, not 100% wet replacement: the protected stereo master remains explicitly present and still owns headroom.

Physical listening established the strongest checkpoint so far:

```text
clear as hell
clearly spatial
getting close to full replacement of HeSuVi
```

But it also exposed a much more precise geometry failure:

```text
bubble smaller than preferred
field behaves more like a band than a sphere
much of that band still resolves inside the head and behind
rear remains somewhat too strong
front volume is underdeveloped
height is underdeveloped
OFF still does not collapse the world as dramatically as desired
```

A later P0.5 check on `Cosmic Cove Galaxy` exposed a **tiny ON-only grain** that was not audible with Omniphony OFF. The rest of the presentation remained very clean. This is a narrow fidelity regression to remove, not a reason to weaken the spatial target.

The first repair candidate is transport continuity rather than DSP strength. The Windows worker previously used a bounded playback queue that silently discarded a complete processed block whenever the queue was full. P0.6 now applies backpressure instead of dropping processed audio. This changes no shell coefficient, support gain, HRTF behavior or protected-master mix law. The same passage must be physically rechecked before treating the grain as solved.

This means the next spatial problem is **shape**, not generic support loudness.

## Active P0.6: anterior + vertical expansion

P0.6 keeps the same protected-master architecture, evidence analysis, bass veto and **full 100% derived-field host coefficient**. Its spatial experiment remains the originally intended geometry weighting of the derived shell.

The intended balance is:

```text
rear support        down
rear height         down
side wrap           preserve
front/front-side    up
front height        strongly up
upper-front volume  strongly up
```

The portable music-field processor now:

- raises the height-permission prior in all three support bands;
- gives broad and lateral evidence more weight in L/R front support;
- trims lateral/diffuse continuation into Lb/Rb;
- makes Tfl/Tfr the dominant height contribution;
- retains Tbl/Tbr only strongly enough to close the upper shell behind;
- allows a very small coherent-mid contribution into front height, but only when broad-scene evidence already permits it;
- keeps coherent frontal anchors protected from becoming synthetic overhead vocals;
- keeps C and LFE silent.

The implementation also carries regression assertions that hard-panned spatial material must produce:

```text
front energy > rear energy
top-front energy > top-rear energy
height energy > 0
LFE energy = 0
```

The bass-foundation regression is now measured relative to direct-master energy instead of using a brittle accumulated-energy constant. This keeps the invariant focused on actual bass leakage rather than failing merely because the same tiny filter residue is distributed across a larger shell.

These are structural tests, not claims about final perceptual balance.

## Foobar-for-Home-Theater influence

Repository studied for P0.6:

`https://github.com/ArtifexEt/Foobar-for-Home-Theater`

Two mechanisms were useful as research inputs rather than code to transplant:

1. its `Add Ceiling Speakers` DSP preserves all existing channels and synthesizes only missing height channels;
2. its height synthesis is difference/spatial-evidence dominant, adds surround/rear feed, and uses only a small coherent-mid feed; its top-back path is weaker than its top-front path.

Its integrated `Reference` stereo upmix also structurally restrains rear more than side and makes top-front stronger than top-back.

The important lesson is architectural:

```text
preserve authoritative bed/master
+ synthesize only the missing dimension
+ make rear continuation weaker than the useful side/front field
+ make upper-front the primary height cue
```

Its actual AVR speaker-bed coefficients are **not** treated as Omniphony headphone targets. P0.6 adapts only the topology to the existing frequency-evidence + inherited binaural renderer architecture.

## Why room effects are still off in P0.6

P0.5 proved that a very clean spatial field already exists. P0.6 therefore isolates shell shape before adding environmental cues.

Still disabled:

```text
early reflections
late reverb
air absorption
artificial LFE
```

If P0.6 successfully inflates the front and upper bubble while preserving clarity, the next isolated experiment can test tiny directional early reflections / radial-distance cues for stronger front externalization. If P0.6 remains inside-head despite the new geometry, that is evidence that geometry alone is insufficient and makes the reflection/depth experiment much easier to interpret.

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
-> queue continuously without silently dropping processed blocks
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

The active shell gives already-spatial broad/lateral/diffuse evidence vertical extent. Frequency changes the *permission prior* but never becomes a height command by itself.

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

The master is not attenuated merely to make room for the spatial effect. P0.6 intentionally keeps this same mix law while the tiny P0.5 grain is first tested against the independent transport-continuity repair.

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

## Current listening setup

For stereo-music development, the temporary Hi-Fi Cable endpoint should be configured as **Stereo / 2.0**.

A 7.1 Windows endpoint feeding the stereo process-loopback prototype reduced playback level. Returning the endpoint to Stereo restored normal level. Treat this as a prototype transport/gain finding.

The clean route remains:

```text
Hi-Fi Cable speaker config = Stereo / 2.0
foobar upmix = OFF
HeSuVi = OFF
ASIO Bridge forwarding = OFF
Omniphony = only audible path to the FiiO/headphones
```

## Sound frontier

The immediate listening question is now:

```text
P0.6 anterior + vertical shell
-> does the front move outward?
-> does the field arch above the listener?
-> is rear dominance reduced without losing wrap?
-> does the bubble become meaningfully larger?
-> does the protected master remain just as clear and punchy?
-> is the tiny P0.5 grain on Cosmic Cove Galaxy gone?
```

If yes:

```text
next: radial depth / tiny directional early reflections
-> source extent
-> ambient continuity
-> lower-shell plausibility
-> later, only if earned, restrained late-room support / distance-dependent air behavior
```

If no, do not simply raise global support gain. Determine whether the failure is front-distance perception, HRTF geometry, insufficient upper continuity, evidence ownership, or transport/mix behavior.

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

The desired bypass reaction remains:

```text
The world collapsed.
```

not:

```text
The music came back.
```
