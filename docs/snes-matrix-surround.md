# SNES matrix-surround source evidence

This note records a source-specific exception to the generic game-music spatial rule. SNES S-DSP signed stereo routing can carry authored matrix-surround phase information. Omniphony may use that phase relationship as presentation evidence, but it must not relabel it as an authored 3-D point.

## Evidence

### SNES hardware

SNESdev documents `VxVOLL` and `VxVOLR` as signed per-voice left/right volume registers. It also documents `OUTX` as the signed current voice sample after envelope and before volume. Global `MVOLL/MVOLR` and `EVOLL/EVOLR` are signed as well.

Sources:

- https://snes.nesdev.org/wiki/APU_register_table/DSP_voice
- https://snes.nesdev.org/wiki/S-DSP_registers

This matches the causal source tap used by the SPC path:

```text
BRR/interpolation/envelope -> OUTX-equivalent dry voice
                           -> signed VxVOLL/VxVOLR route
                           -> dry stereo mix

shared echo FIR -> signed EVOLL/EVOLR -> wet stereo contribution
```

### Historical SNES use

ConsoleMods documents matrixed surround on SNES/SFC and describes compatible software carrying inverted-phase information in the ordinary stereo pair for decoding by Dolby Surround / Pro Logic style receivers. It names examples including *Super Turrican*, *Jurassic Park*, *Fatal Fury Special*, and *Final Fantasy VI*.

Source:

- https://consolemods.org/wiki/SNES:Audio_Information

A community technical discussion independently describes the same SNES technique and points to titles that deliberately use opposite-polarity stereo routing:

- https://www.reddit.com/r/miniSNESmods/comments/hyajts/snes_dolby_surround/

These secondary sources are useful for title discovery. The renderer law itself is grounded in the signed S-DSP registers plus the documented matrix-encoding relationship below.

### Dolby matrix relationship

Dolby's Lt/Rt documentation describes the surround component of a matrixed stereo downmix as being added in phase to one leg and out of phase to the other so a Pro Logic decoder can recover the surround channel. Center information is added in phase to both legs.

Source:

- https://professionalsupport.dolby.com/s/article/A-Guide-to-Dolby-Metadata?language=en_US

Therefore, a sufficiently balanced opposite-polarity native route is not merely a pan value. It is evidence compatible with an authored matrix-surround channel.

## Renderer law

Keep three concepts separate:

1. **left/right pan** is derived from route magnitudes;
2. **phase-opposition / matrix-surround evidence** is derived from route signs plus magnitude balance;
3. **3-D position** remains an Omniphony presentation unless explicit source geometry exists.

Consequences:

- changing only the sign of one route must not swap left/right side;
- balanced opposite-polarity routing may establish strong rear/surround presentation evidence;
- matrix evidence may outrank an inferred musical-role label because the route is lower-level source truth;
- matrix evidence alone must not manufacture height;
- a strongly one-sided inverted route must not automatically become a rear channel;
- Surround off must preserve the original matrixed stereo exactly so external Dolby/Pro Logic decoding remains possible;
- Surround on may decode the phase relationship at the separated-source stage before the stereo matrix is destroyed.

## Current implementation

`renderer/src/source_scene.rs` derives a conservative `matrix_surround_phase_cue` from `NativeStereoRoute`:

- same-sign routes -> no matrix cue;
- equal-magnitude opposite-sign routes -> maximal cue;
- increasingly unbalanced opposite-sign routes -> progressively weaker cue.

The cue can strengthen rear placement and object extent in the full-sphere presentation while leaving source-position authority as `InferredPresentation` and leaving elevation dependent on independent evidence.

## Important next test

Do not validate this only with synthetic `L=+1, R=-1` controls. Add real SPC captures from known matrix-surround titles and compare:

```text
A. historical stereo -> actual Pro Logic / matrix decoder
B. historical stereo -> Omniphony ordinary stereo inference
C. separated SPC sources -> Omniphony matrix-aware source renderer
```

Measure at least:

- front/back discrimination;
- stability of intended rear events;
- front-stage collapse or leakage;
- timbral coloration;
- shared-echo integrity;
- mono/reference compatibility;
- listener preference.

The target is not to imitate a Pro Logic receiver. The decoder is a historical control. The target is to recover the authored surround relationship from source truth and then render it more cleanly through the full-sphere binaural engine.