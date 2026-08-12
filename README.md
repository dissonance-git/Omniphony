# Omniphony

Omniphony is an experimental, always-on headphone spatial processor built from the upstream `mgth/Omniphony` renderer and aimed at one perceptual goal:

> **Make the headphones disappear and place the listener inside the largest coherent version of the same finished recording.**

The target is not ordinary stereo widening and not a full-wet surround reconstruction. The finished recording remains the musical authority while Omniphony adds a bounded external spatial world around it.

Windows is the first product host. The renderer / inference / DSP core remains portable.

---

# Current listening baseline

**Baseline 2 / current reference:**

```text
89507730946ce80d767881e507d7f18937971f9f
```

The default launcher profile is:

```text
all
```

Physical listening now establishes this as the best reference in the fork so far.

Baseline 2 preserves the successful properties of the earlier cascaded-binaural build while adding the mechanisms that survived subsequent listening:

- protected finished stereo master;
- coherent low-frequency / body foundation;
- analysis-only stereo evidence extraction;
- derived 7.1.4 support field;
- coherent sample-for-sample height transfer instead of simply adding more upper wash;
- System-H-derived full-sphere virtual-speaker shell with the regular upper layer at +60 degrees;
- 10-degree-grid-aligned upper directions in the default `all` profile so important static height speakers land directly on cached HRTF nodes;
- measured SAF/KEMAR HRTF rendering;
- native ITD, metric-distance, early-reflection, short-reverb and air-absorption machinery;
- restrained support-only spectral compensation, including the small listening-derived presence trim near 3.9 kHz;
- fixed output makeup followed only by a stereo-linked look-ahead peak-safety guard;
- single-executable Windows supervisor + audio-engine architecture.

Listening feedback on this state is simply: **sounds great**.

That changes the development obligation again. New work must beat this state while preserving its bass authority, transient ownership, clarity, tonal comfort, motion, scale, reliability and continuous 360-degree presentation.

If a candidate becomes merely different, wetter, brighter, blurrier or more spectacular at the cost of musical authority, keep Baseline 2.

---

# 1. Architectural law

> **Use Omniphony itself as the spatial core. Add custom machinery only for jobs the inherited renderer does not already own.**

Current music path:

```text
FINISHED STEREO MASTER
        │
        ├──────────────────────────────→ protected direct master
        │
        ├→ coherent music foundation
        │      └→ additive body / pressure delta
        │
        └→ analysis-only stereo evidence
               │
               ├→ magnitude / phase
               ├→ M/S relation
               ├→ pan / coherence
               ├→ directness / diffuseness
               └→ temporal stability
                         │
                         ▼
                derived 7.1.4 support
                         │
                         ├→ coherent elevation transfer
                         │
                         ▼
             OMNIPHONY SPEAKER STAGE
             full-sphere virtual room
                         │
                         ▼
               CASCADED BINAURAL
       HRTF / ITD / metric distance / room
       early reflections / short FDN / air
                         │
                         ▼
                  binaural support
                         │
       protected master + foundation + support
                         │
                         ▼
          fixed makeup + peak safety only
                         │
                         ▼
                     headphones
```

The protected master does **not** pass through the virtual room.

That is the fidelity floor.

---

# 2. What changed after Baseline 1

The earlier reference proved that cascaded binaural could create a much more continuous 360-degree bubble than the direct support path, but it still had three audible weaknesses:

1. bright material could become sharp;
2. height existed but did not have enough authority;
3. some upper energy behaved more like spatial support than a real musical event occupying elevation.

The successful post-baseline changes were deliberately small and mechanism-specific.

## 2.1 Support-only spectral correction

Measured SAF/KEMAR common coloration is partially compensated in the support branch. Physical listening still found a small hard edge on guitars, so a shallow additional static trim was added around:

```text
3.9 kHz
-0.8 dB
Q 1.1
```

This never touches the protected master.

The rule remains:

> **Do not solve spatial harshness by darkening the recording. Correct the spatial branch first.**

## 2.2 Coherent height transfer

Height is no longer increased only by constructing more upper-field energy.

A controlled fraction of an already-existing horizontal support waveform is moved sample-for-sample into its corresponding elevated lane:

```text
horizontal event
      │
      ├→ horizontal remainder
      └→ same waveform → elevated lane → HRTF
```

The transfer does not create another wet copy. Before binaural rendering, horizontal + elevated lane amplitude remains algebraically conserved by the transfer itself.

This is the preferred direction for strong height:

> **move structured evidence upward rather than smear additional energy upward.**

## 2.3 Steeper upper shell

The active headphone shell moves the eight regular System-H-derived upper directions from +30 degrees to +60 degrees while retaining:

- the horizontal layer;
- zenith;
- the lower hemisphere;
- no synthetic LFE speakers.

The default `all` profile additionally moves the diagonal upper azimuths onto exact 10-degree HRTF grid nodes.

This is a headphone experiment, not a claim that the modified shell is normative ITU-R BS.2051 System H. A canonical System H reference remains stored separately.

---

# 3. Source authority

The richer the source truth, the less Omniphony should infer.

```text
stereo
→ preserve master + infer bounded support

5.1 / 7.1
→ preserve real directional channels

5.1.2 / 7.1.4
→ preserve authored height

object audio
→ preserve supplied object positions

Ambisonics / HOA
→ preserve the supplied field representation

already-binaural material
→ avoid destructive double virtualization
```

A stereo recording can justify presentation support. It cannot prove that an instrument was authored above or behind the listener.

Height is presentation permission, not recovered metadata.

---

# 4. Hard fidelity laws

> **Dimension may not be purchased by damaging the music.**

Turning Omniphony off may collapse:

- width;
- front/back depth;
- height;
- radial distance;
- source extent;
- ambient continuity;
- listener envelopment.

Turning Omniphony off must **not** restore:

- clarity;
- kick impact;
- bass pressure;
- drum body;
- transient snap;
- tonal identity;
- rhythmic precision;
- center stability;
- microdetail;
- dynamics;
- comfortable spectral balance.

Shortest form:

> **OFF may collapse the world. It may not bring the rhythm section back to life.**

Additional invariants:

```text
mass        may remain anchored
trajectory  must remain alive

environment may grow
direct musical structure must not blur
```

---

# 5. Music foundation

The music foundation is a fixed, causal, stereo-preserving additive delta. It is not a compressor, limiter, exciter or fake LFE generator.

Approximate shaping:

```text
85 Hz      +2.30 dB   pressure / mass
240 Hz     +1.20 dB   kick / body
800 Hz     +0.50 dB   density
4.5 kHz    -0.35 dB   mild presence relaxation
```

Properties:

```text
no compressor
no saturation
no mono fold
no fake LFE
no spatialized sub-bass foundation
same topology left/right
```

A personal headphone correction profile is not Omniphony’s universal voicing.

---

# 6. Stereo evidence path

The FFT is **analysis only**. The finished waveform is never STFT-resynthesized.

Current support policy:

```text
<320 Hz        protected master / foundation only
320–1200 Hz    restrained support body
1.2–5 kHz      spatial support with presence restraint
>5 kHz         slower-moving, reduced support
```

C and LFE remain silent in the inferred stereo support bed.

The current 7.1.4 evidence order is:

```text
L R C LFE Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

The useful support lanes are derived from actual stereo relations rather than from semantic source separation.

---

# 7. Current spatial world

Baseline 2 is a wide, depth-led full sphere.

Current priorities are approximately:

```text
FRONT DISTANCE   primary
REAR DISTANCE    primary but not dominant
HEIGHT           primary and still the main frontier
SIDE WIDTH       very important
SIDE DISTANCE    secondary
LOWER HEMISPHERE present but restrained
```

Scale should come mainly from:

```text
geometry
HRTF / ITD
propagation timing
early-field structure
source extent
metric distance
room-axis shaping
```

not from excessive treble, chorus, Haas widening, giant decorrelation networks or a wetter late-reverb tail.

The perceptual north star remains:

> **The listener should feel as if their head occupies a portal into the recording’s acoustic world, not as if a clever effect is attached to the headphones.**

---

# 8. Listening-profile matrix

`START-OMNIPHONY.cmd` accepts an optional temporary research profile.

With no argument it selects:

```text
all
```

Current profiles:

```text
control   previous current-best topology before the latest matrix
all       conservative combined Baseline-2 candidate
direct    direct per-evidence-lane binaural instead of cascade
external  stronger early-field / smaller late-field candidate
prtf      structural PRTF HRTF model instead of measured KEMAR
close     shorter distance / room-geometry control
tracked   head-tracking-ready configuration
diffuse   deliberately more diffuse late-field comparison
```

These are experiment switches, not product modes.

Mutually exclusive HRTF models are compared rather than stacked. `all` does not convolve the same source through measured KEMAR and PRTF simultaneously.

See `docs/listening-profiles.md` for the exact current differences.

---

# 9. Output level and headroom

The host currently sums:

```text
protected master
+ foundation delta
+ binaural support
→ 0.90 fixed linear base gain
→ +3.5 dB fixed makeup
→ stereo-linked look-ahead peak guard
```

The peak guard is endpoint safety only:

```text
ceiling          -1.0 dBFS
look-ahead        5 ms
release          160 ms
```

It applies one linked gain to both ears only when needed to prevent a future peak crossing the ceiling.

It is not a loudness leveller, support AGC or spatial compressor.

---

# 10. Windows architecture

```text
PORTABLE OMNIPHONY CORE
renderer / inference / DSP
          │
          ▼
WINDOWS HOST
process loopback / output / lifecycle
          │
          ▼
Omniphony.exe
```

The Windows runtime remains a single executable with two internal roles:

```text
Omniphony.exe
├→ invisible supervisor / tray / watchdog
└→ internal audio-engine child process
```

This preserves crash / endpoint recovery without shipping a separate engine executable.

Normal direction:

```text
install once
→ start silently at login
→ find configured physical output
→ process continuously
→ recover automatically
→ tray only when control is needed
```

---

# 11. Realtime robustness

Current Windows work includes:

- process-specific Windows loopback capture;
- MMCSS priority for the producer / audio-engine thread;
- MMCSS priority for the playback callback;
- bounded playback queue;
- underrun telemetry;
- automatic worker restart after output-stream failure;
- invisible supervisor/watchdog;
- per-user autostart;
- tray recovery after Explorer/taskbar restart.

If crackle reproduces with the spatial effect disabled, treat it first as a realtime-host problem rather than blaming HRTF complexity.

---

# 12. Native mechanism ownership

Prefer inherited Omniphony machinery for:

```text
HRTF / HRIR
ITD
head pose / tracking
metric distance
speaker geometry
VBAP / source extent
cascaded rendering
direct binaural rendering
first-order reflections
late room field
SOFA
object / bed handling
```

Custom fork ownership is strongest for:

```text
source preservation
stereo evidence
confidence / permission laws
music foundation
support-field construction
coherent elevation transfer
final host summing
validation
Windows lifecycle / transport
```

If an inherited feature was designed for a final output bus, verify that it remains correct on Omniphony’s additive support branch.

---

# 13. Relationship to libaural and Helix

```text
HELIX
research / experiment machinery
       │
       ▼
libaural
reusable machine-hearing / human-hearing research
       │
       ▼
Omniphony
listening product / perceptual testbed
```

The projects help each other but remain separate.

Omniphony asks:

> **Does this mechanism make the listening experience better?**

libaural asks:

> **What auditory variable changed, can it be represented cleanly, and what claim does the evidence actually support?**

Current spatial research sequence:

```text
AUD-SPACE-001  typed binaural evidence gate
AUD-SPACE-002  elevation spectral-template stress
AUD-SPACE-003  externalization-cue decomposition
```

Do not create a runtime dependency from Omniphony onto libaural merely because an experiment originated there.

---

# 14. Research influences

Durable influence families include:

- upstream Omniphony;
- Spatial Audio Framework;
- Steam Audio;
- OpenAL Soft;
- MPEG-H binaural / virtual-loudspeaker rendering work;
- binaural externalization / BRIR literature;
- human sagittal-plane / elevation-localization literature;
- 3D Tune-In;
- open binaural rendering systems;
- mature source-safe upmix / downmix systems.

Research is an influence source, not permission to replace a successful renderer with a parallel science project.

Every substantive audible change follows the repository research gate in `AGENTS.md`:

```text
listening observation
→ literature pass
→ mature implementation pass
→ smallest relevant mechanism
→ adapt to Omniphony topology
→ CI / measurement
→ physical listening
→ keep, revise, or revert
```

---

# 15. Current frontier

The bubble exists. Width, depth, bass authority and overall fidelity are no longer the primary unsolved problems.

The next work is ordered by expected perceptual leverage.

## A. Hybrid direct-height rendering

Keep the cascaded binaural renderer for the continuous 360-degree environment, but remove the four height support lanes from the cascade and binauralize those lanes directly at their intended upper directions.

Desired topology:

```text
horizontal / side / rear support
→ cascaded virtual-speaker world
                     ┐
                     ├→ sum support binaural
                     │
TFL / TFR / TBL / TBR│
→ direct HRTF height ┘
```

Hard requirement:

> **A height sample may take one path or the other, never both.**

This is the next major implementation target.

## B. Directional HRTF early reflections

The current reflection bank supplies useful binaural timing / level structure, but selected strongest reflection paths can eventually carry their own full directional HRTF filtering.

The goal is stronger externalization without a larger late tail.

## C. HRTF selection / personalization

Generic KEMAR may become the ceiling for elevation on a particular listener.

Omniphony already owns measured, SOFA and structural HRTF families. Future personalization should be a controlled selection/calibration problem, not random HRTF swapping.

## D. World locking under real head motion

The renderer already has head-pose / SensorsOSC plumbing. A real comparison requires actual motion input.

The purpose is not a visual gimmick. It is to make the acoustic world remain stable while the listener’s head moves.

## E. Additional controlled candidates

After the major spatial coordinates are stable:

- dedicated near-field HRTF filtering;
- transient versus sustained spatial routing;
- short-term interaural-coherence shaping;
- higher-order Ambisonic intermediate-field experiments.

Do not add these simply because they exist. Each must beat Baseline 2 or solve a clearly isolated limitation.

---

# 16. Failure signals

```text
piercing / fatigue
moving spectral coloration
comb-like timbral edges
clarity loss
source blur
transient softening
bass/drum power loss
center instability
rear gravity
hallway coloration
late-reverb fog
stereo motion collapse
height that is only brighter, not higher
realtime crackle / underruns
```

Listening outranks parameter aesthetics.

The numbers are allowed to look unconventional if the result remains coherent.

---

# 17. Definition of success

The goal is not “accurate surround conversion.”

It is:

> **A finished stereo recording keeps its identity, weight, dynamics and clarity while gaining a stable external world with front distance, rear depth, extreme width, convincing overhead volume, continuous motion and enough radial scale that ordinary headphone playback feels dimensionally collapsed by comparison.**

Baseline 2 makes the central question much narrower:

> **Can Omniphony turn an already excellent giant headphone sphere into a genuinely external, vertically occupied acoustic world without sacrificing the finished master underneath it?**

That is the current frontier.
