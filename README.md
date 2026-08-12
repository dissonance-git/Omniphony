# Omniphony

Omniphony is an experimental, always-on headphone spatial processor built from the upstream `mgth/Omniphony` renderer.

Its perceptual target is simple:

> **Make the headphones disappear and place the listener inside the largest coherent version of the same finished recording.**

The finished stereo master remains the musical authority. Omniphony adds a bounded external spatial world around it instead of replacing it with a full-wet reconstruction.

Windows is the first host. The renderer, inference and DSP core remain portable.

---

# Current model

The **Current model** is the single normal listening path. The old tray-profile matrix is retired.

Physical listening on 2026-08-12 first favored the measured-HRTF early-reflection path that had temporarily been labeled `Externalization`. The preference was small enough that placebo could not be excluded, but the path was not worse, used a more physically meaningful early-field mechanism, and replaced the weaker profile matrix.

A later pass added lane-local transient-aware early-room excitation. Physical listening then reported another incremental improvement, so that mechanism is now retained as part of Current model rather than treated as a provisional challenger.

The current development build carries one new listening candidate on top of that retained model: **front / center refinement**. It keeps the successful side, rear and lower presentation fixed while widening the front L/R evidence sources, widening/lifting the top-front pair, reducing only the low-level late room closure, and lowering final fixed listening level by 0.7 dB.

Normal runtime exposes only:

```text
On / clean bypass
Restart audio engine
Start with Windows
Exit
```

The historical profile experiments and promotion history remain in `docs/listening-history.md`.

## Current signal path

```text
FINISHED STEREO MASTER
        │
        ├──────────────────────────────→ protected direct master
        │
        ├→ coherent music foundation
        │      └→ additive pressure / kick / body delta
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
                         ▼
             OMNIPHONY SPEAKER STAGE
             full-sphere virtual world
                         │
                         ▼
               CASCADED BINAURAL
           measured SAF/KEMAR HRTF
           ITD / metric distance / air
                         │
                         ├→ short low-level late closure
                         │
                         └→ lane-local transient evidence
                            → first-order image timing / wall tone
                            → six directional reflection buses
                            → measured HRTF
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

## Current audible tuning

The coherent, non-spatial foundation currently uses approximately:

```text
85 Hz low shelf      +2.80 dB   pressure / mass
110 Hz punch         +1.60 dB   kick impact
240 Hz body          +1.20 dB   upper-bass / drum body
800 Hz density       +0.50 dB   lower-mid density
4.5 kHz high shelf   -0.35 dB   mild presence relaxation
```

The additive HRTF support branch carries a static, support-only SAF/KEMAR compensation. The current dense-guitar comfort trim remains:

```text
3.5 kHz
-1.20 dB
Q 0.90
```

The protected master is not darkened to solve spatial fatigue.

Current Windows final fixed makeup is now:

```text
+2.8 dB
```

This is 0.7 dB lower than the previous +3.5 dB value. Bass, foundation, support ratios and peak-safety behavior were not changed to accomplish the level reduction.

## Current front / center refinement candidate

The new candidate changes only the front-facing geometry and low-level late closure:

```text
front L/R x position       +/-1.00 -> +/-1.15
top-front x position       +/-0.96 -> +/-1.10
top-front z position          2.15 -> 2.45
late-room level              0.020 -> 0.016
late-room RT60                0.14 -> 0.12 s
final fixed makeup          +3.5 dB -> +2.8 dB
```

Side, rear and lower evidence-source poses are unchanged. The bass/foundation path is unchanged. The measured-HRTF early field is unchanged. The retained transient-aware early-room mechanism is unchanged.

The center and LFE support lanes remain silent. Centered vocals therefore remain owned by the protected stereo master; the lower late-field energy is intended to let that direct center read more clearly without shrinking the early directional world around stereo material and drums.

This is a listening candidate, not yet a demonstrated improvement.

## Current early field

The Current model replaces the original lightweight analytic first-order reflection panner with a bounded measured-HRTF field:

```text
12 derived support lanes
        ↓
lane-local transient evidence
        ↓
first-order shoebox timing
+ source-distance / wall filtering
        ↓
contributions grouped by six room walls
        ↓
6 directional reflection buses
        ↓
measured SAF/KEMAR HRTF + ITD
        ↓
linear sum with the primary support render
```

This is deliberately not 22 virtual speakers multiplied by six separate full-HRTF reflection convolvers. Timing and wall filtering happen before wall-wise aggregation, so the HRTF cost stays fixed at six reflection buses.

The field is designed to change directional structure rather than win by loudness. Engineering tests cover delayed arrival, wall-direction binaural asymmetry, protected C/LFE exclusion, transient-envelope bounds and block-boundary invariance.

### Retained transient-aware early-room behavior

Each existing support lane carries a small transient detector before its early-reflection delay bank:

```text
existing support lane
        ↓
fast energy envelope
vs slow energy envelope
        ↓
positive-rise transient evidence
        ↓
bounded early-room gain only
        ↓
existing first-order wall paths
```

The detector is lane-local rather than global. A sharp rise in one support lane cannot directly turn up every other simultaneously active lane.

Current constants are:

```text
fast envelope      3 ms
slow envelope     45 ms
release           20 ms
maximum gain    +2.5 dB
```

This gain applies only to the signal entering the early-reflection bank. It does **not** modify:

- the protected stereo master;
- the coherent music foundation;
- the primary spatial-support render;
- center or LFE support.

Physical listening reported an incremental improvement after this mechanism was introduced, so it is retained in Current model. That result does not prove instrument identity or a general psychoacoustic law. It only earns this bounded behavior in the present renderer.

---

# Architectural law

> **Use Omniphony itself as the spatial core. Add custom machinery only for jobs the inherited renderer does not already own.**

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
room machinery
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
bounded directional early-field adaptation
final host summing
validation
Windows lifecycle / transport
```

If an inherited feature was designed for a final output bus, verify that it remains correct when used only on Omniphony's additive support branch.

---

# Hard fidelity laws

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
mass         may remain anchored
trajectory   must remain alive

environment  may grow
direct music must not blur
```

The stereo master is never STFT-resynthesized. FFT/STFT machinery in the music path is analysis-only.

---

# Stereo evidence and support

Current support policy is approximately:

```text
<320 Hz        protected master / coherent foundation only
320–1200 Hz    restrained support body
1.2–5 kHz      spatial support with presence restraint
>5 kHz         slower-moving, reduced support
```

The current 7.1.4 evidence order is:

```text
L R C LFE Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

C and LFE remain silent in the inferred stereo support bed.

Height is presentation permission, not recovered source metadata. A stereo recording can justify an external spatial presentation, but it cannot prove that a particular guitar, cymbal or voice was authored above or behind the listener.

---

# Source authority

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
→ preserve supplied field representation

already-binaural material
→ avoid destructive double virtualization
```

This becomes especially important as source-aware analysis arrives later. A classifier is evidence, not authorship.

---

# Realtime robustness

The Windows path is designed to stay boring and reliable even as the research layer becomes more intelligent.

Current runtime protections include:

- process-specific Windows loopback capture;
- MMCSS priority for producer and playback callback;
- bounded playback queue;
- underrun telemetry;
- short continuity concealment when the playback queue briefly starves;
- stereo-linked 5 ms look-ahead peak safety;
- amortized sliding maximum in the peak guard instead of rescanning the full look-ahead window every frame;
- automatic worker restart after output-stream failure;
- invisible supervisor/watchdog;
- per-user autostart;
- tray recovery after Explorer/taskbar restart.

The underrun concealment is not an audible effect under normal operation. It exists so a rare scheduling gap becomes a short smooth continuity event instead of a one-sample waveform cliff that can sound like a crackle.

If crackle reproduces, inspect `omniphony.log` and the underrun telemetry before blaming HRTF complexity.

---

# Windows architecture

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

The shipped runtime remains one executable with two internal roles:

```text
Omniphony.exe
├→ invisible supervisor / tray / watchdog
└→ internal audio-engine child process
```

The normal child process is pinned to the Current model. Historical listening profiles are no longer selectable from the tray or launcher.

---

# Relationship to libaural, VGM Tooling and Helix

The projects remain separate at runtime but deliberately feed research into each other at different layers.

```text
                    HELIX
          research / provenance / method
                       │
                       ▼
                    libaural
              experimental machine hearing
                       │
          heard-state / validated mechanisms
                       │
          ┌────────────┴────────────┐
          ▼                         ▼
     VGM Tooling                Omniphony
source-native synthesis     spatial presentation
 / reconstruction               testbed
```

VGM Tooling can expose source-native structure before final stereo collapse. libaural can use that as unusually strong calibration evidence for machine hearing. Omniphony tests which distinctions actually matter when a human experiences the final headphone presentation.

No project should become a runtime dependency of another merely because it provided a useful experiment.

---

# Listening evidence so far

The listening program has produced useful compression and a sequence of retained improvements:

- `hybrid` direct-height routing was not clearly distinguishable from the prior current model and did not earn its added complexity;
- `prtf` was clearly worse in the tested system, described as tinnier, and remains a negative result;
- several room/routing variants were not clearly distinguishable enough to justify carrying them as user-facing modes;
- the measured-HRTF early-reflection path was heard as **slightly better**, though the first comparison could not exclude placebo, and became Current model;
- the later transient-aware early-room pass was reported as **incrementally better again** and is therefore retained;
- the current front / center refinement has not yet been physically adjudicated.

See `docs/listening-history.md` for the retained experiment record.

---

# Current frontier

The large 360-degree world already exists. The current work should refine **directness, front scale and musical intelligence**, not add more generic spatial variants.

## 1. Front / center refinement

The current isolated listening question is:

> **Can the direct center become slightly clearer while front stereo width and upper-front scale grow, without changing the already-successful side/rear/lower world or weakening bass and drums?**

Acceptance signals:

- centered vocals feel a little cleaner and less washed;
- stereo-panned/front material occupies a larger front face;
- height feels taller or more open rather than merely brighter;
- side, rear and lower wrap remain as good as before;
- bass pressure and drum authority remain unchanged;
- the 0.7 dB level reduction feels more comfortable rather than less alive.

Rejection signals:

- vocals become unnaturally isolated or thin;
- the front image develops a hole;
- widened front geometry weakens center stability;
- height becomes detached from the mix;
- the drier late field makes the scene noticeably smaller;
- any successful side/rear/lower quality regresses.

## 2. Source-aware control from libaural

Once the basic geometry/directness balance is satisfactory, libaural can tell Omniphony more about **what** generated the evidence.

The intended relationship is:

```text
original stereo master
        │
        ├→ audible truth
        │
        └→ libaural machine-hearing analysis DSP
                ↓
         time-varying source/activity evidence
         masks / continuity / confidence
                ↓
         bounded Omniphony control projection
```

Separated stem waveforms do not need to enter the audible master. Modern separators, semantic activity models and open-vocabulary extractors are sensors for libaural, not replacement audio sources.

This can eventually let the retained transient mechanism behave differently for drums, vocals, guitars, bass, strings and pads without hard-coded genre presets.

## 3. Head motion and personalization

Later controlled frontiers remain:

- actual world-lock testing with live head motion;
- listener-specific or selected HRTFs;
- dedicated near-field HRTF behavior;
- short-term interaural-coherence shaping where it solves a measured obligation.

These should not interrupt the directness/source-awareness path unless listening exposes a more urgent failure.

---

# Failure signals

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
spatial pumping / twitching from analysis
```

Listening outranks parameter aesthetics.

A mechanism stays only if it improves the experience or solves an isolated failure without damaging the protected music underneath it.

---

# Definition of success

The goal is not "accurate surround conversion."

It is:

> **A finished stereo recording keeps its identity, weight, dynamics and clarity while gaining a stable external world with front distance, rear depth, extreme width, convincing overhead volume, continuous motion and enough radial scale that ordinary headphone playback feels dimensionally collapsed by comparison.**

The next narrower question is:

> **Can the front grow outward and upward while the protected center becomes clearer, without shrinking the successful rest of the world?**
