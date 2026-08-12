# Spatial listening profiles

These are temporary listening controls for the Windows protected-master music path.
They are not product modes and they do not define general hearing laws.

The current stereo master, coherent foundation, support extraction, coherent
height transfer, output makeup and final stereo-linked peak safety remain common
unless stated otherwise.

Launch syntax:

```bat
START-OMNIPHONY.cmd <profile>
```

With no argument, `START-OMNIPHONY.cmd` selects `all`.

## Current listening observation · 2026-08-12

A physical tray-profile comparison reported **no clear audible difference among
the non-PRTF profile variants in this pass; the presentation remained good**.
`prtf` was the exception and was heard as **tinnier and worse**.

Consequences:

- `all` remains the current model;
- `hybrid` is not promoted because the direct-height branch did not produce a
  reliably audible benefit;
- the other profile-level room/routing controls remain useful engineering/A-B
  controls, but this pass provides no listening evidence that they improve the
  current model;
- `prtf` is retained as a negative control rather than a current contender;
- a head-tracking claim still requires actual live head-motion input rather than
  a static `tracked` profile.

This is one physical listening result under the current hardware/listening
conditions. It is decisive for what gets promoted in this project, but it is not
rewritten as a universal human-hearing result.

## control

Prior reference for this experiment series.

- cascaded binaural
- measured SAF/KEMAR HRTF
- current +60-degree upper shell
- reflection level 0.32
- late reverb 0.028, RT60 0.16 s
- unit scale 9.25 m
- current coherent height transfer
- current support-only spectral compensation / 3.9 kHz listening trim

## all

The current model and default listening reference.

Relative to `control`:

- upper shell diagonal directions align to the renderer's 10-degree HRTF grid
- reflection level 0.36
- late reverb level 0.020
- RT60 0.14 s

It deliberately does not stack mutually exclusive HRTF models or increase late
reverberation simply to sound larger.

## hybrid

Experimental direct-height control relative to `all`.

The twelve evidence lanes are partitioned before rendering:

```text
L R C LFE Ls Rs Lb Rb
→ current `all` cascaded virtual-speaker world

TFL TFR TBL TBR
→ measured SAF/KEMAR direct HRTF

stereo cascade + stereo direct height
→ linear support sum
```

Hard routing law:

> A height sample enters one spatial route, never both.

The direct-height engine is deliberately source-pure:

- no phantom extraction;
- no generated objects;
- no second early-reflection room;
- no late reverb;
- no air-absorption pass;
- no cascade-specific SAF spectral compensation.

The surrounding eight-lane branch keeps the current `all` room and cascade.
The protected master and coherent foundation remain outside both spatial routes.

Renderer tests establish two mechanical safety properties before listening:

- the 8+4 evidence split is exclusive and sample-wise lossless;
- an elevated off-grid source has direct and cascaded first arrivals aligned to
  within one sample frame after both native paths are settled.

Physical listening on 2026-08-12 did not reveal a clear difference from the
current model, so the extra routing complexity has **not** earned promotion.

## direct

Direct per-evidence-lane binaural rendering instead of the virtual-speaker
cascade.

Relative to `control`:

- binaural mode `direct`
- reflection level 0.24
- late reverb level 0.015

This isolates the localization cost/benefit of removing the VBAP virtual-speaker
stage before HRTF rendering for the entire support field.

## external

Early-field weighted externalization candidate.

Relative to `all`:

- reflection level 0.42
- late reverb level 0.012
- RT60 0.12 s

The intent is to test stronger binaural early-reflection structure without
buying externalization through a longer or louder diffuse tail.

## prtf

Alternative structural pinna model and current **negative listening control**.

Relative to the grid-aligned shell:

- HRTF source: Spagnol/Geronazzo/Avanzini-style structural PRTF implementation
- frequency scale 1.00
- pinna depth 0.72
- SAF/KEMAR-specific spectral compensation disabled

Physical listening on 2026-08-12 described this profile as **tinnier and worse**
than the current presentation. It therefore does not qualify for promotion.
Retaining it is useful because it is a concrete counterexample to the idea that
adding a different structural pinna model automatically improves elevation or
externalization.

## close

Distance-cue control.

- unit scale 2.25 m
- reflection room 8 x 11 x 7 m
- reflection level 0.24
- late reverb level 0.012

This is not a true near-field HRTF implementation. It tests the distance cues the
current renderer actually owns: propagation/air filtering, reflection geometry
and direct-to-environment relation while leaving programme level authoritative.

## tracked

Head-tracking-ready control.

- same conservative room balance as `all`
- SensorsOSC-compatible `/android/rotationvector`
- `rotvec` input format

Without live tracking input this remains at the static head pose and is not a
head-motion experiment. True world-lock evidence requires actual motion input.

## diffuse

Deliberate late-field comparison.

- reflection level 0.24
- late reverb level 0.055
- RT60 0.28 s
- predelay 24 ms

This exists to test whether a larger decorrelated late field merely increases
width while weakening source identity/elevation compared with coherent transfer
and binaural early-field cues.

## Not yet represented by a faithful listening switch

These remain separate engineering/research obligations rather than being faked
with a nearby-looking effect:

- full-HRTF convolution for selected early reflections;
- dedicated near-field HRTF filtering;
- explicit short-term interaural-coherence shaping;
- transient/sustained spatial routing;
- source-aware control from a validated deep-lookahead libaural analysis DSP;
- higher-order Ambisonic intermediate field;
- human head-motion comparison without a live tracker.

They should enter the listening path one at a time with matched-level controls.
