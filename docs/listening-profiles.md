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

## control

Reference for this experiment series.

- cascaded binaural
- measured SAF/KEMAR HRTF
- current +60-degree upper shell
- reflection level 0.32
- late reverb 0.028, RT60 0.16 s
- unit scale 9.25 m
- current coherent height transfer
- current support-only spectral compensation / 3.9 kHz listening trim

## all

Conservative combined candidate.

Relative to `control`:

- upper shell diagonal directions align to the renderer's 10-degree HRTF grid
- reflection level 0.36
- late reverb level 0.020
- RT60 0.14 s

It deliberately does not stack mutually exclusive HRTF models or increase late
reverberation simply to sound larger.

## direct

Direct per-evidence-lane binaural rendering instead of the virtual-speaker
cascade.

Relative to `control`:

- binaural mode `direct`
- reflection level 0.24
- late reverb level 0.015

This isolates the localization cost/benefit of removing the VBAP virtual-speaker
stage before HRTF rendering. It is not the future hybrid-height implementation.

## external

Early-field weighted externalization candidate.

Relative to `all`:

- reflection level 0.42
- late reverb level 0.012
- RT60 0.12 s

The intent is to test stronger binaural early-reflection structure without
buying externalization through a longer or louder diffuse tail.

## prtf

Alternative structural pinna model.

Relative to the grid-aligned shell:

- HRTF source: Spagnol/Geronazzo/Avanzini-style structural PRTF implementation
- frequency scale 1.00
- pinna depth 0.72
- SAF/KEMAR-specific spectral compensation disabled

This is mutually exclusive with measured KEMAR by design. Do not interpret a
preference as proof that a generic structural model is more accurate.

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

- hybrid cascaded enclosure plus direct-only height HRTF branch;
- full-HRTF convolution for selected early reflections;
- dedicated near-field HRTF filtering;
- explicit short-term interaural-coherence shaping;
- transient/sustained spatial routing;
- higher-order Ambisonic intermediate field;
- human head-motion comparison without a live tracker.

They should enter the listening path one at a time with matched-level controls.
