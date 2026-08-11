# Omniphony

Omniphony is an experimental, always-on headphone spatial processor built from the upstream `mgth/Omniphony` renderer and aimed at a simple perceptual goal:

> **Make the headphones disappear and place the listener inside the largest coherent version of the same finished recording.**

This repository is not trying to make ordinary stereo merely wider. It is building a reusable spatial-audio core that can preserve source truth while presenting music, games, films and richer spatial formats as a continuous externalized world around the listener.

Windows is the first product host. The renderer itself remains platform-independent.

---

## Baseline 1

**Canonical listening baseline:**

```text
03dac8bb454444b47353c39f65b58ce82617d731
```

This is the first build that is good enough to be treated as a reference rather than a disposable prototype.

Physical listening established that Baseline 1:

- produces a substantially more continuous spherical / “bubble” presentation than the earlier direct-HRTF path;
- restores the low-end power, drum weight and physicality that earlier spatial candidates lost;
- produces useful height, front/back depth and very wide lateral extent;
- is already usable in normal listening in place of the previous HeSuVi + DTS Virtual:X chain;
- preserves unusual clarity for an aggressive spatial processor;
- runs as a minimal single-executable Windows runtime;
- still has one important audible defect: **some bright transients, especially cymbals and already-aggressive mixes, can become piercing or fatiguing.**

Post-baseline work must beat this state. If an experiment damages the baseline’s bass authority, clarity, motion, spatial scale or reliability, revert the experiment rather than redefining success.

---

# 1. Architectural law

The most important engineering rule is:

> **Use Omniphony itself as the spatial core. Add custom machinery only for jobs the inherited renderer does not already own.**

The current music architecture is:

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
               ├→ magnitude
               ├→ phase
               ├→ M/S relation
               ├→ pan / coherence
               ├→ directness / diffuseness
               └→ temporal stability
                         │
                         ▼
                derived 7.1.4 support
                         │
                         ▼
             OMNIPHONY SPEAKER STAGE
             virtual 7.1.4 loudspeaker room
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
                  fixed linear gain
                         │
                         ▼
                     headphones
```

The protected master does **not** pass through the virtual room.

That distinction is the fidelity floor of the project.

---

# 2. Why cascaded binaural became the core

The early music prototypes rendered each derived support channel directly through its own HRTF path. They could sound clear and spatial, but the result often behaved more like a set of spatial points than a continuous environment.

Upstream Omniphony’s cascaded renderer changed that topology:

```text
derived support
→ real virtual-speaker renderer
→ continuous 3D loudspeaker field
→ binauralize the resulting virtual room
```

Physical listening immediately produced a more convincing bubble.

Therefore the current headphone architecture is:

> **cascaded virtual-speaker rendering first, binaural rendering second.**

Direct binaural remains useful upstream machinery and a reference path, but it is no longer the primary music architecture for this fork.

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
→ preserve the field representation

already-binaural material
→ avoid destructive double virtualization
```

A stereo recording can justify presentation support. It cannot prove that an instrument was authored “above” or “behind” the listener.

Height is permission, not recovered metadata.

---

# 4. Hard fidelity laws

## 4.1 Dimension may not damage the music

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

The shortest formulation is:

> **OFF may collapse the world. It may not bring the rhythm section back to life.**

## 4.2 Energy may be anchored; motion may not be frozen

The low-frequency foundation exists to keep physical weight coherent. It does not mean stereo motion should become static.

Authored left/right movement must survive through the protected master and foundation. A panned tom roll should still sweep.

```text
mass        may remain anchored
trajectory  must remain alive
```

## 4.3 The master is authoritative

The finished stereo waveform remains explicitly present.

Do not replace it with a full-wet virtualized reconstruction merely to obtain spatial cues.

---

# 5. Music foundation

The music foundation is a fixed, causal, stereo-preserving additive delta. It is not a compressor, limiter, exciter or fake LFE generator.

Current baseline shaping is approximately:

```text
85 Hz      +2.30 dB   pressure / mass
240 Hz     +1.20 dB   kick / body
800 Hz     +0.50 dB   density
4.5 kHz    -0.35 dB   mild presence relaxation
```

Properties:

```text
no compressor
no limiter
no saturation
no dynamic loudness control
no mono fold
no fake LFE
no spatialized sub-bass foundation
same topology left/right
```

This is intentionally separate from any headphone-specific EQ. A personal Noire X correction profile is not Omniphony’s universal voicing.

---

# 6. Stereo evidence path

The FFT exists for **analysis only**. The mastered waveform is never STFT-resynthesized.

Current broad support policy:

```text
<320 Hz        protected master/foundation only
320–1200 Hz    restrained support body
1.2–5 kHz      spatial support with presence restraint
>5 kHz         slower-moving, reduced support
```

The lower spatial boundary was raised from roughly 220 Hz to 320 Hz after physical listening showed that sending too much bass and drum body through HRTF/room propagation reduced perceived punch even though the dry master still existed.

C and LFE remain silent in the inferred stereo support bed.

---

# 7. Baseline spatial world

Baseline 1 uses the inherited Omniphony renderer aggressively.

The support scene is a virtual 7.1.4 shell with:

- maximally wide sides;
- strong front distance;
- useful rear impact without rear gravity;
- dominant top-front height;
- upper-rear closure;
- metric distance scaling;
- native first-order reflections;
- a very short late room field;
- air absorption;
- cascaded speaker-to-binaural rendering.

The perceptual weighting is:

```text
FRONT DISTANCE   primary
REAR DISTANCE    primary, but not dominant
HEIGHT DISTANCE  primary
SIDE WIDTH       very important
SIDE DISTANCE    secondary
```

The goal is a **wide, depth-led sphere**, not a side-only ring and not a rear-heavy halo.

---

# 8. Known Baseline 1 defect: bright-material harshness

The remaining major audible defect is intermittent piercing / fatigue on bright transient material.

Important: this is **not assumed to be a simple EQ problem.**

The post-baseline research program treats several mechanisms as plausible contributors.

## 8.1 Virtual-loudspeaker binaural comb coloration

Research on MPEG-H binaural virtual-loudspeaker rendering reports undesirable comb-filter coloration caused by phase differences between binaural filters when virtual loudspeaker signals are downmixed to the ears.

This is directly relevant to the cascaded topology.

## 8.2 Dry-master + delayed support interference

The protected master is summed with a delayed and phase-shaped binaural support branch. Superposition of delayed and undelayed correlated material can create audible comb structure even when the delayed branch is substantially lower in level.

Therefore a narrow piercing peak may be a **coherence / phase problem**, not a broad treble imbalance.

## 8.3 HRTF spectral normalization

The current Omniphony HRIR grid performs broadband diffuse-field-style **energy normalization** so different HRTF sources do not jump wildly in overall level.

That is not the same as frequency-dependent diffuse-field equalization.

Spatial Audio Framework and published binaural-rendering work support frequency-dependent diffuse-field / coloration compensation as a way to reduce timbral errors while retaining localization and externalization.

## 8.4 Early reflections are currently too spectrally simple

Omniphony’s first-order reflection bank currently models:

```text
propagation delay
distance loss
analytic ITD
broadband ILD
```

but does not yet model a real wall’s frequency-dependent absorption on each reflected path.

This means a very large virtual room can have unrealistically shiny reflection paths, particularly obvious on cymbals and bright percussion.

The research literature on headphone externalization specifically supports combining early reflection geometry with wall absorption, air absorption and restrained late reverberation.

### Post-baseline rule

> **Do not solve the harshness merely by making Omniphony darker. Fix spectral/coherence behavior in the spatial branch first, then retain only as much static high-band trim as listening still requires.**

---

# 9. Post-baseline research priorities

The next renderer work is ordered by mechanism rather than by arbitrary parameter tweaking.

## Priority A — reflection spectral realism

Add frequency-dependent loss to early reflection paths so distant virtual walls do not return bright transient energy unrealistically intact.

Desired result:

```text
larger perceived room
less metallic glare
no loss of master attack
```

## Priority B — HRTF diffuse-field spectral compensation

Evaluate a frequency-dependent diffuse-field normalization / coloration-compensation stage for the support renderer.

Reference ideas include:

- Spatial Audio Framework diffuse-field HRTF equalization;
- MagLS / coloration-compensated binaural rendering;
- time-aligned HRTFs with diffuse-field constraints;
- MPEG-H virtual-loudspeaker spectral compensation.

This belongs in the renderer/HRTF layer, not in the protected stereo master.

## Priority C — coherence management

Investigate light, transient-preserving decorrelation only where coherent duplicate support paths create audible combing.

The master remains untouched.

Potential targets are diffuse / reflection / upper support components rather than direct musical anchors.

## Priority D — larger bubble

Continue increasing the world only after spectral stability improves.

Scale should come mainly from:

```text
geometry
propagation timing
early-field structure
source extent
HRTF / ITD
room-axis shaping
```

not from louder treble or a wetter late reverb tail.

## Priority E — HRTF selection / personalization

Omniphony already supports:

- embedded SAF KEMAR;
- SOFA loading;
- synthetic head model;
- parametric pinna model;
- PRTF model.

Once the renderer itself is spectrally stable, systematic HRTF selection can target better elevation, front/back discrimination and externalization.

Do not call a random alternate HRTF “personalization.”

---

# 10. Output level and headroom

The Windows host currently combines:

```text
protected master
+ foundation delta
+ binaural support
→ fixed linear output gain
```

There is no sample-dependent host clamp, compressor, limiter or AGC.

Baseline 1 uses a fixed output gain of approximately:

```text
0.72 linear
≈ -2.85 dB
```

Physical listening now shows that this is probably too conservative for normal use, particularly when headphone correction upstream already reserves its own preamp headroom.

Post-baseline work should reclaim output level with explicit peak measurement and static gain where possible.

Do not reintroduce the earlier support-only auto-gain behavior or fast gain riding.

---

# 11. Windows product architecture

The portable core remains independent of Windows.

```text
PORTABLE OMNIPHONY CORE
renderer / inference / DSP
          │
          ▼
WINDOWS HOST
loopback capture / output / lifecycle
          │
          ▼
Omniphony.exe
```

The current Windows runtime ships as a single executable.

Internally the executable supports two process roles:

```text
Omniphony.exe
├→ invisible supervisor / tray / watchdog
└→ internal audio-engine child process
```

The second process preserves crash/device-recovery isolation without shipping a second runtime executable.

Normal product direction:

```text
install once
→ start silently at login
→ find configured output
→ run continuously
→ recover automatically after endpoint failure
→ tray only when control is needed
```

The final goal is infrastructure, not an app that must be opened before listening.

During rapid development the ZIP may include a tiny `START-OMNIPHONY.cmd` convenience launcher. That CMD is not part of the architecture.

---

# 12. Realtime robustness

Windows realtime work includes:

- MMCSS priority for the producer/audio-engine thread;
- MMCSS priority for the playback callback;
- a bounded playback queue;
- explicit underrun telemetry;
- automatic worker restart after output-stream failure;
- an invisible supervisor/watchdog;
- per-user autostart;
- tray recovery after Explorer/taskbar restart.

Audio crackle under heavy external compute load should be treated first as a realtime-host problem when it reproduces with the spatial effect disabled.

Do not blame HRTF complexity without evidence.

---

# 13. Native-first mechanism ownership

Prefer inherited Omniphony machinery for:

```text
HRTF / HRIR
ITD
head pose
metric distance
speaker geometry
VBAP / source extent
cascaded rendering
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
final host summing
validation
Windows lifecycle / transport
```

If an inherited feature was designed for a **final output bus**, verify that it remains correct when used on Omniphony’s additive support branch. Native does not automatically mean topologically appropriate.

---

# 14. Research influences

The repository’s influence documents remain the durable research ledger. The most important current families are:

- upstream Omniphony;
- Spatial Audio Framework;
- Steam Audio;
- MPEG-H binaural / virtual loudspeaker rendering research;
- binaural externalization / BRIR literature;
- 3D Tune-In;
- Google / open binaural rendering work;
- Cavern;
- Foobar Home Theater / multichannel-height references;
- FreeSurround / Real3D as evidence sources rather than final renderers;
- source-safety and reversibility ideas from mature upmix/downmix systems.

Research is an influence source, not permission to replace a working renderer with a parallel science project.

---

# 15. Relationship to libaural and Helix

```text
HELIX
research / experiment machinery
       │
       ▼
libaural
reusable machine-hearing research
       │
       ▼
Omniphony
consumer spatial processor / testbed
```

Until the Omniphony listening baseline is more mature, libaural should contribute **small validated distinctions and preservation laws**, not a large semantic runtime.

Preferred runtime hierarchy:

```text
fixed validated DSP / renderer behavior
↓
small stateful perceptual mechanisms
↓
bounded adaptive control only when clearly superior
↓
large learned / semantic runtime only exceptionally
```

---

# 16. Development method

Baseline 1 changes the development philosophy.

Before Baseline 1, the job was to discover an architecture that could become genuinely spatial without destroying music.

After Baseline 1, the job is:

> **Preserve the baseline’s winning invariants while deliberately pushing beyond it, then retreat from the first mechanism that crosses a real perceptual boundary.**

The frontier can move aggressively because the baseline is now explicit.

Failure signals include:

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
realtime crackle / underruns
```

Listening outranks parameter aesthetics.

The numbers are allowed to look unconventional if the result remains coherent.

---

# 17. Current definition of success

The near-term goal is not “accurate surround conversion.”

It is:

> **A finished stereo recording keeps its identity, weight, dynamics and clarity while gaining a stable external world with front distance, rear depth, extreme width, convincing overhead volume, continuous motion and enough radial scale that ordinary headphone playback feels dimensionally collapsed by comparison.**

Baseline 1 proves enough of that target to stop treating it as hypothetical.

The next major problem is no longer whether the bubble exists.

It is making the bubble **larger, louder and spectrally calmer at the same time.**
