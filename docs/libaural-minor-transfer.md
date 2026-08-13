# libaural minor transfer into Omniphony

This note deliberately keeps the boundary narrow. Omniphony is stabilizing its own protected-master music path; libaural remains a broader artificial-hearing research project and is **not** a required runtime dependency.

Current transfer law:

```text
libaural research distinction
→ smallest stable mechanism or validation law
→ Omniphony only when it improves listening
```

Do **not** import the full artificial-hearing stack before the current Omniphony sound is stable.

## Small influences promoted now

The libaural Omniphony boundary document reinforces several already-useful invariants:

- preserve groove and microtiming;
- preserve transient ownership;
- preserve bass foundation / pressure / weight;
- preserve important stereo relationships;
- preserve source motion rather than flattening it into a static foundation;
- prefer finished, authored, stable behavior over obvious runtime fader-riding;
- use rich research only to discover a smaller realtime mechanism.

These are validation and architecture laws, not a request for semantic source separation or an AI mix engineer in the playback loop.

## Foundation does not mean static

The current coherent foundation is stereo, not mono. Left and right are filtered independently with the same topology, so authored panning remains authored panning.

For drums and other low/body-rich sources:

```text
physical mass / impact
→ protected master + coherent foundation

stereo position / sweep
→ remains in the protected L/R waveform

attack + upper resonances + directional evidence
→ may additionally enter the Omniphony support field
```

Therefore a tom roll that pans across the recording should still sweep across the listener while each hit retains direct physical authority.

Frozen shorthand:

> **Energy may be anchored. Motion may not be frozen.**

Do not add a special low-frequency motion renderer unless physical listening shows the preserved stereo pan plus existing 320 Hz+ support is insufficient.

## Hearing loss and prosthetic-hearing research

libaural now treats hearing loss, deafness, hearing aids, cochlear implants, hybrid electric-acoustic stimulation, auditory brainstem implants and biological hearing restoration as **controlled perturbations of auditory representation**.

That does not change Omniphony's current listening model.

The useful transfer is initially a validation route:

```text
current Omniphony render
+
altered-hearing simulation / listener model
↓
which spatial and musical cues remain available?
which cues become redundant?
which protected invariants fail first?
```

This can later help answer questions such as:

- whether the current spatial field remains useful under reduced frequency selectivity;
- whether height or externalization depends too strongly on a narrow spectral cue range;
- whether binaural asymmetry makes one support mechanism unstable;
- whether added support energy falls into a region that is poorly usable for a particular listener;
- whether a small compensation can improve accessibility without touching the protected master more than necessary.

The most useful open-source precedent is 3D Tune-In, which deliberately combined binaural spatialization with hearing-loss and hearing-aid simulation. Treat it as a research quarry, not as a replacement renderer.

The first libaural loss discriminator now adds a more concrete rule. `AUD-LOSS-001` matched two synthetic loss mechanisms exactly on an isolated-tone loss and restored both with the same gain, yet the broadened-frequency-selectivity condition still differed by about `9.03 dB` on its local masking relation. The transfer to Omniphony is not the specific number. It is this validation law:

> **Matching level or an audiogram-like gain target is not enough evidence that the spatially relevant auditory representation has been restored.**

So any future listener-specific accessibility layer must test the obligations Omniphony actually needs, such as usable interaural relations, externalization cues, spectral-directional structure, transient ownership and music fidelity. Do not call a compensation successful merely because a level target has been matched.

`AUD-LOSS-002` now prepares the next site-of-lesion challenge with a Verhulst2018 auditory-periphery teacher: the same modeled BM/IHC drive is retained while HSR/MSR/LSR neural population weights change. That remains research-only and externally unscored. Its immediate value to Omniphony is to prevent future personalization from treating "hearing loss" as one EQ curve.

Hard boundary:

```text
hearing-loss model
≠ default audible processing

hearing-aid / implant research
≠ permission to alter the normal master path
```

If listener-specific compensation is ever added, it should be an explicit accessibility/personalization layer with its own validation and bypass, not a silent change to the Current model.

## Deferred until Omniphony is stable

Keep these in libaural research for now:

- semantic source identity;
- long-context auditory memory;
- prediction-driven organization;
- learned-model scene interpretation;
- expensive biological hearing front ends;
- multi-hypothesis auditory-world state as a realtime requirement;
- hearing-loss / hearing-aid / implant simulation as normal runtime processing;
- listener-specific impairment compensation without controlled validation.

The product should first earn a stable everyday stereo sound using the inherited Omniphony renderer, protected source truth, small evidence mechanisms, and listening-driven correction.
