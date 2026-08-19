# Windows Spatial Sound provider research

Status: research record and implementation guidance. This document records the platform, open-source, academic, and practitioner findings that shaped the Windows object-audio roadmap. It is not a claim that the full provider path is already live on a real machine.

## Product question

Omniphony should occupy the Windows-wide headphone spatial-renderer position while retaining the same renderer for ordinary stereo and conventional surround.

The desired split is therefore by **source representation**, not by product mode:

```text
ordinary stereo / 5.1 / 7.1 / height PCM
        ↓
normal Windows shared render stream
        ↓
Omniphony stream SFX
        ↓
source-authority mapping
        ↓
Omniphony renderer
        ↓
stereo endpoint

Windows Spatial Audio application
        ↓
static + dynamic spatial objects
        ↓
Omniphony Spatial Sound provider
        ↓
source-authority mapping
        ↓
SAME Omniphony renderer
        ↓
RAW stereo WASAPI egress
        ↓
physical stereo endpoint
```

The second path must not traverse the normal Omniphony stream SFX again after Omniphony has already performed the final binaural render.

---

## 1. Microsoft platform findings

### 1.1 8.1.4.4 is an object vocabulary, not the next ordinary PCM bed

Windows Spatial Audio defines 17 predefined static spatial-object roles through `AudioObjectType`:

```text
horizontal: FL FR C LFE SL SR BL BR BC
upper:      TFL TFR TBL TBR
lower:      BFL BFR BBL BBR
```

That is the complete static **8.1.4.4** vocabulary used by the Spatial Audio object API.

Conventional `WAVEFORMATEXTENSIBLE` PCM remains useful for speaker beds such as 5.1, 7.1, and height layouts such as 7.1.4, but Omniphony must not pretend that a conventional PCM negotiation is equivalent to receiving the complete 17-role Spatial Audio scene.

**Architectural consequence:**

```text
7.1 / 7.1.4 PCM
→ compatibility ingress

8.1.4.4 static roles
+ dynamic XYZ objects
→ Spatial Audio object ingress
```

### 1.2 Application-facing spatial contract

The primary Windows application contract is `ISpatialAudioClient`, with spatial rendering performed through an `ISpatialAudioObjectRenderStream` and individual `ISpatialAudioObject` instances.

The object stream supplies audio independently of the final headphone/speaker endpoint representation. This is the correct semantic boundary for a general spatial renderer because Omniphony can preserve the authored scene before performing one binaural reduction.

### 1.3 Dynamic positions are metric listener-relative XYZ

Microsoft specifies a listener-relative right-handed coordinate system for spatial-object positions:

- `+X`: right;
- `+Y`: up;
- `+Z`: behind the listener;
- one coordinate unit: one meter.

Omniphony's current scene convention therefore uses the lossless axis conversion:

```text
Windows [x, y, z]
→ Omniphony [x, -z, y]
```

The `_m` naming in the portable Windows spatial contract is intentional and should remain.

Dynamic objects must retain continuous metric XYZ rather than being snapped to the 17 static anchors.

### 1.4 Static stream topology and update lifetime

The static object mask is chosen at render-stream activation. The application/provider update loop is structured around:

```text
BeginUpdatingAudioObjects
→ obtain/write active object buffers
→ EndUpdatingAudioObjects
```

Object buffer pointers are update-quantum scoped. Omniphony must copy or consume the supplied PCM within that lifetime and must not retain raw Windows buffer pointers across update boundaries.

This maps naturally to the fixed-topology static-object realtime ABI already being developed in `realtime_ffi`.

### 1.5 RAW egress is the clean double-processing escape hatch

Microsoft documents RAW render streams as bypassing **Stream Effects (SFX)**. Endpoint effects remain a separate stage.

Omniphony's accepted Windows topology intentionally uses the native stream SFX and leaves the old endpoint EFX absent. That creates a clean coexistence design:

```text
normal app
→ ordinary shared stream
→ Omniphony SFX
→ endpoint

spatial app
→ Omniphony object provider
→ Omniphony final binaural render
→ RAW shared stereo stream
→ Omniphony SFX bypassed
→ endpoint
```

This is preferable to a virtual cable, hidden loopback device, or deliberate second spatial pass.

RAW support is endpoint-dependent. The provider must check the endpoint's RAW-processing capability before requesting `AUDCLNT_STREAMOPTIONS_RAW`. If the selected physical endpoint cannot support the required RAW path, that result is a falsifier for this egress design and a different egress boundary must be found. Do not silently fall back through the normal Omniphony SFX and double-render the signal.

---

## 2. Open-source implementation quarry

### 2.1 MSSOAL

`ThreeDeeJay/MSSOAL` is the most relevant public implementation found for the undocumented Windows Spatial Sound provider seam.

Useful mechanisms observed there include:

- a COM class factory for the spatial provider;
- an `ISpatialAudioClient`-shaped provider surface;
- `ISpatialAudioObjectRenderStream` stream lifecycle;
- `ISpatialAudioObject` handling;
- experimental registration beneath the Windows `Spatial\\Encoder` registry surface;
- Process Monitor observations used to infer that registration boundary.

MSSOAL is **not** platform authority. Its role is implementation quarry only.

Rules for borrowing from it:

1. Use Microsoft documentation as the source of truth whenever Microsoft documents the interface or behavior.
2. Treat the `Spatial\\Encoder` registration seam as experimental until a real Omniphony machine proves Windows enumerates and activates it.
3. Do not inherit OpenAL Soft as Omniphony's renderer.
4. Do not inherit undocumented claims about Sonic, object limits, or internal Windows routing without independent evidence.
5. Transplant COM/interface lifecycle concepts, not application-specific architecture.

### 2.2 Provider architecture consequence

The Windows provider should remain thin:

```text
Windows COM / Spatial Audio types
        ↓
small native adapter
        ↓
portable Omniphony spatial semantic contract
        ↓
portable realtime object-scene ABI
        ↓
existing SourceFrameRenderer
```

Windows SDK types should not leak into the portable Rust renderer.

The existing renderer already accepts stable source identity and authored positions, so Windows object ingress should feed that renderer rather than create a second Windows-only spatial engine.

---

## 3. Source-authority consequences

The research reinforces the existing `AUTHORED / DERIVED / EMPTY` contract.

### Static objects

A Windows static object is `AUTHORED` spatial truth. Its role should map directly into the canonical scene.

### Dynamic objects

A Windows dynamic object's stable identity, PCM, and continuous XYZ are `AUTHORED` spatial truth. They remain parallel to the static scene rather than becoming synthetic channels.

### LFE

The static LFE object remains non-directional in Omniphony's renderer. It must not be treated as a point source merely because the Spatial Audio API represents it as one static object role. The existing native-bed behavior, where LFE is kept out of HRTF placement and combined coherently, remains the correct renderer law.

### Stereo and conventional surround

Stereo-derived rear/height support remains `DERIVED`. A real 5.1/7.1/height bed or object scene must suppress contradictory inference rather than be spatialized again from scratch.

---

## 4. Academic research findings

SciSpace literature review was used to place perceptual work **after** source-geometry ingress rather than use DSP to compensate for missing platform information.

### 4.1 Elevation and front/back

Elevation and front/back discrimination depend strongly on direction-dependent HRTF spectral structure, especially pinna-related cues. Horizontal localization retains stronger dependence on ITD/ILD information.

Implication:

> If Omniphony receives the same authored object geometry as a reference renderer and still loses elevation/front-back certainty, HRTF spectral behavior becomes a legitimate renderer problem. Before equivalent geometry is available, it is not.

Research starting points:

- Baumgartner, Majdak & Laback, *Modeling sound-source localization in sagittal planes for human listeners*, JASA (2014), DOI `10.1121/1.4887447`.
- Romigh & Simpson, *Do you hear where I hear?: isolating the individualized sound localization cues*, Frontiers in Neuroscience (2014), DOI `10.3389/FNINS.2014.00370`.
- Dick & Herre, *Predicting the Precision of Elevation Localization Based on Head Related Transfer Functions*, ICASSP (2019), DOI `10.1109/ICASSP.2019.8682313`.

### 4.2 HRTF personalization

Individualized HRTFs can materially improve localization, particularly sagittal/elevation behavior, but personalization should not become a prerequisite for a strong generic renderer.

Roadmap order remains:

```text
strong generic renderer
→ multiple interchangeable HRTF datasets
→ objective/perceptual selection
→ morphology-assisted personalization
→ measured individualized HRTFs when available
```

### 4.3 Head tracking

Dynamic binaural/head-tracking research supports head motion as an important localization and externalization cue. It belongs after stable object ingress and a strong static renderer.

Useful example:

- Planinec et al., *The Accuracy of Dynamic Sound Source Localization and Recognition Ability of Individual Head-Related Transfer Functions in Binaural Audio Systems with Head Tracking*, Applied Sciences (2023), DOI `10.3390/app13095254`.

Head tracking should remain optional rather than becoming a condition for normal music playback.

### 4.4 Externalization and room support

Externalization is not equivalent to simply adding more reverb. Research points to interactions between HRTF correctness, interaural coherence, early room information, source direction, and listener motion.

Useful starting points:

- Leclère, Lavandier & Perrin, *On the externalization of sound sources with headphones without reference to a real source*, JASA (2019), DOI `10.1121/1.5128325`.
- Landschoot & Jot, *Binaural externalization processing method for object-based audio rendering*, JASA (2023), DOI `10.1121/10.0018389`.

Therefore room/externalization work must not be used to conceal weak direct-source localization.

---

## 5. QuadraphonicQuad practitioner findings

QuadraphonicQuad was used as practitioner evidence rather than technical authority.

Recurring useful observations:

- a binaural headphone render should not be assumed to preserve the perceptual intent of a speaker-array mix automatically;
- different binaural renderers can produce meaningfully different elevation, directionality, envelopment, and tonal results from related multichannel/object material;
- head tracking can greatly improve the impression of a fixed external scene for some listening contexts while being undesirable for others;
- personalization can matter strongly to some listeners, but preference and usefulness are context-dependent.

Product implication:

> Omniphony should expose head tracking and personalization as optional renderer capabilities, while the baseline must remain convincing without either.

Practitioner evidence is useful for choosing listening tests and detecting failure modes, but it does not override primary platform documentation or controlled research.

---

## 6. Experimental provider ladder

The current evidence ladder remains deliberately granular:

```text
P0 endpoint capability
→ P1 provider registration/enumeration
→ P2 COM provider activation
→ P3 one real static object
→ full 17-role static scene
→ P4 one real moving dynamic object
→ P5 real spatial application/game
```

Proof at one rung does not imply the next.

### Current source state

Implemented or scaffolded in source:

- read-only endpoint spatial-capability probing;
- inert provider registration experiment;
- an `ISpatialAudioClient` capability scaffold;
- portable Windows static/dynamic semantic contract;
- compiled static-object scene adapter feeding the existing source-aware renderer;
- fixed-topology static-object realtime ABI work;
- static COM stream lifecycle smoke work;
- inaudible RAW endpoint probe work.

These are source/CI states until a real Windows machine proves the corresponding platform boundary.

---

## 7. Most important architectural result

The strongest result of this research pass is that Omniphony does not need two products or a user-visible cable topology.

If Windows accepts the provider seam and the endpoint supports RAW egress, the full architecture can remain:

```text
                         ┌─ stereo PCM ───────────┐
                         │                        │
Windows applications ───┼─ 5.1 / 7.1 / height ─┼─ normal stream → Omniphony SFX ─┐
                         │                        │                                │
                         └─ Spatial Audio objects ─ Omniphony provider ────────────┤
                                                                                   ↓
                                                                      one Omniphony renderer
                                                                                   ↓
                                                                      one binaural stereo result
                                                                                   ↓
                                                              normal SFX path OR RAW provider egress
                                                                                   ↓
                                                                            headphones
```

The renderer remains the same. Only the amount and authority of source information change.

---

## References

### Microsoft

- Spatial Sound: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Render Spatial Sound using spatial audio objects: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `ISpatialAudioObjectRenderStream`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioobjectrenderstream
- `ISpatialAudioObject`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioobject
- `AudioObjectType`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ne-spatialaudioclient-audioobjecttype
- `ISpatialAudioObject::SetPosition`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioobject-setposition
- RAW audio streams / `AUDCLNT_STREAMOPTIONS_RAW`: https://learn.microsoft.com/windows-hardware/drivers/audio/raw-audio-format

### GitHub implementation quarry

- MSSOAL: https://github.com/ThreeDeeJay/MSSOAL
- OpenAL Soft: https://github.com/kcat/openal-soft
- Microsoft Windows Spatial Sound sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound
- Microsoft Xbox ATG Advanced Spatial Sounds sample: https://github.com/microsoft/Xbox-ATG-Samples/tree/main/UWPSamples/Audio/AdvancedSpatialSoundsUWP

### Academic research

See the DOI references in sections 4.1–4.4 and `ROADMAP.md`.

### Practitioner evidence

- QuadraphonicQuad Object-based Surround forum: https://quadraphonicquad.com/forums/object-based-surround.178/

Use forum observations as listening/practitioner evidence only.