# Windows spatial input contract

This note defines how Omniphony for Windows should interpret Windows audio layouts before the portable binaural renderer. It complements `docs/omniphony-for-windows.md` and exists to keep four different concepts separate:

1. conventional shared-mode PCM beds;
2. Windows Spatial Audio static/dynamic objects;
3. Omniphony's canonical static scene frame;
4. Omniphony's denser internal support/rendering lattice.

## Governing rule

**Preserve the richest source representation Windows actually supplies. Represent it inside one canonical 8.1.4.4-capable scene without pretending missing channels were authored.**

The physical headphone endpoint may remain stereo while the upstream Windows graph supplies richer spatial information.

```text
SOURCE TRUTH                         CANONICAL OMNIPHONY SCENE                         OUTPUT
stereo ---------------------------> 8.1.4.4-capable scene, sparse/inferred --------> binaural stereo
5.1 ------------------------------> 8.1.4.4-capable scene, 5.1 authored -----------> binaural stereo
7.1 ------------------------------> 8.1.4.4-capable scene, 7.1 authored -----------> binaural stereo
7.1.4 ----------------------------> 8.1.4.4-capable scene, 7.1.4 authored ---------> binaural stereo
7.1.4.4 --------------------------> 8.1.4.4-capable scene, 7.1.4.4 authored -------> binaural stereo
8.1.4.4 --------------------------> 8.1.4.4-capable scene, fully authored static --> binaural stereo
dynamic spatial objects ----------> continuous object layer + static scene --------> binaural stereo
```

The FiiO/DAC side remains two physical channels throughout.

## Canonical static scene: 8.1.4.4

Omniphony's **base static scene contract is 8.1.4.4**: seventeen semantic spatial anchors matching the complete predefined static-channel vocabulary exposed by Microsoft Spatial Sound.

```text
horizontal
FL FR C LFE SL SR BL BR BC

upper
TFL TFR TBL TBR

lower
BFL BFR BBL BBR
```

Common source subsets map naturally into this one frame:

```text
7.1       = FL FR C LFE SL SR BL BR
7.1.4     = 7.1 + TFL TFR TBL TBR
7.1.4.4   = 7.1.4 + BFL BFR BBL BBR
8.1.4.4   = 7.1.4.4 + BC
```

The frame is always available as Omniphony's static coordinate vocabulary. The signals occupying it are not always authored.

Every static anchor must therefore carry source authority explicitly. At minimum the scene contract must distinguish:

```text
AUTHORED   source or host supplied this signal/position
DERIVED    Omniphony inferred bounded support for this anchor
EMPTY      no trustworthy signal is assigned here
```

Additional confidence, directness/diffuseness, extent, distance, activity, and source-identity metadata may be attached where the portable renderer can use it without confusing inference with authorship.

The central law is:

> **8.1.4.4 is the base coordinate frame, not the default claim about what the source contained.**

## Conventional game and media path

For ordinary Windows shared-mode applications and games, the primary compatibility target remains conventional PCM:

```text
stereo
5.1
7.1
```

On Windows 11 23H2+, `IAudioProcessingObjectPreferredFormatSupport::GetPreferredInputFormat` is specifically documented for headphone virtualization. Microsoft gives the example of a stereo-rendering endpoint whose APO requests 7.1 input.

Therefore the Omniphony endpoint effect should negotiate and preserve authored 5.1/7.1 PCM before reducing it to binaural stereo.

A game that authors 7.1 must still be **authored 7.1** at the Omniphony boundary. Omniphony then places those eight authoritative signals into its 8.1.4.4 scene frame and leaves the remaining anchors `EMPTY` unless a separately validated inference mechanism earns bounded `DERIVED` support.

```text
authored 7.1
        ↓
FL FR C LFE SL SR BL BR = AUTHORED
BC TFL TFR TBL TBR BFL BFR BBL BBR = EMPTY or bounded DERIVED
        ↓
Omniphony rendering geometry
        ↓
binaural stereo
```

Do not label derived height, bottom, or back-center content as authored merely because the internal frame contains those positions.

This remains the important compatibility route for games that expose a generic Home Theater / 7.1 output mode.

## Windows Spatial Audio path

Windows Spatial Audio is a richer source path than conventional shared-mode PCM. `ISpatialAudioClient` supports static spatial objects assigned to predefined speaker positions plus dynamic objects with arbitrary 3D positions.

Microsoft defines up to **8.1.4.4 / 17 static objects** for headphone spatial formats and allows dynamic objects in parallel. On current Windows, Microsoft documents Dolby Atmos for Headphones, DTS Headphone:X, and Windows Sonic for Headphones with the same maximum 17-static-object bed.

That makes 8.1.4.4 the correct interoperability frame for Omniphony's static scene contract rather than merely a maximum format to tolerate.

When a Spatial Audio-aware application supplies 7.1.4, 7.1.4.4, 8.1.4.4, or dynamic object positions, preserve that supplied geometry and authority. Do not collapse it to 7.1 merely because 7.1 is the conventional APO compatibility path.

The endpoint-effect APO and Spatial Audio ingestion may require different Windows host seams. That is acceptable. They must converge on the same portable Omniphony scene semantics rather than duplicating the renderer.

## Dynamic objects are parallel to the static frame

Dynamic spatial objects are not eighteenth, nineteenth, or later fixed channels. They carry continuous 3D position and may move over time.

```text
canonical static frame: 17 semantic anchors
        +
dynamic object layer: arbitrary x/y/z objects
        ↓
one portable Omniphony scene
```

When real object coordinates are supplied, preserve them continuously as far into rendering as possible. Do not snap a moving object prematurely to the nearest `TFL`, `SL`, `BBR`, or other static anchor merely to fit the 8.1.4.4 bed.

Static anchors may still provide room, fallback, diffusion, acceleration, or bed semantics around an object. They are not a reason to throw away more precise object metadata.

Source authority therefore increases approximately as:

```text
stereo evidence
    < authored horizontal bed
    < authored height/lower bed
    < supplied continuous object position / scene field
```

This is an authority ordering, not a statement that every object mix necessarily sounds better than every channel mix.

## Dolby-native interoperability target

Omniphony should work with Dolby content **natively through supported Windows/Dolby interfaces wherever the platform exposes a trustworthy seam**. It should not require a Dolby-specific internal scene model when Microsoft's Spatial Audio layer already presents compatible static/object semantics.

Dolby's own Windows implementation guidance directs applications to Microsoft's Spatial Audio APIs and `ISpatialAudioClient`. Microsoft documents Dolby Atmos for Headphones on current Windows as supporting the full 17-static-object 8.1.4.4 bed plus dynamic objects. This is a strong reason for Omniphony's canonical scene to use the same static vocabulary.

There are three distinct interoperability cases:

### 1. Raw static/object scene reaches Omniphony

If a supported Windows host seam exposes the application's spatial bed and/or dynamic objects before another renderer destroys that information:

```text
Dolby / Windows spatial application
        ↓
8.1.4.4 static objects + dynamic x/y/z objects
        ↓
Omniphony canonical scene
        ↓
Omniphony HRTF / distance / room / binaural renderer
        ↓
stereo headphones
```

This is the preferred long-term Dolby-native path because it preserves the richest source truth and lets Omniphony perform the final headphone rendering.

### 2. Dolby Atmos for Headphones has already rendered the scene

If Windows/Dolby has already converted the spatial scene to final binaural stereo before the Omniphony endpoint effect sees it, Omniphony must **not spatialize it again**.

The safe target is:

```text
native Dolby binaural stereo
        ↓
Omniphony clean spatial bypass
or explicitly validated non-spatial endpoint correction only
        ↓
headphones
```

A reliable host signal/detection mechanism is required before automating this policy. Do not infer "already binaural" from stereo channel count alone.

### 3. Encoded Dolby media

Windows Media Foundation and the Windows spatial-audio platform already provide native Dolby decode/playback paths. Omniphony should prefer those supported operating-system facilities over implementing or reverse-engineering proprietary Dolby bitstream/object codecs merely to reach the same scene.

If a future supported seam exposes decoded spatial bed/object semantics, ingest them into the canonical scene. If the platform exposes only the final Dolby-rendered binaural result, follow case 2 and avoid double virtualization.

This makes **Dolby interoperability a host-ingress problem, not a second Omniphony renderer**.

## Current hard boundary: post-mix EFX is not proven raw-object ingress

Windows documents an endpoint effect (EFX) as occurring after the render mix for the endpoint. Windows documents `ISpatialAudioClient` separately as the application-facing static/dynamic object path delivered to the active spatial renderer.

The public documentation reviewed so far does **not** establish that an ordinary third-party post-mix EFX can recover the raw `ISpatialAudioClient` objects, their original object identities, or their original x/y/z metadata after Windows/Dolby has rendered them.

Therefore:

- do not claim raw Atmos/Spatial Audio object ingestion is solved by the current EFX architecture;
- do not reconstruct object positions from already-rendered binaural audio and call them native objects;
- keep the conventional EFX path as the robust 2.0/5.1/7.1 system-wide route;
- investigate a supported richer spatial ingress in parallel;
- do not hook/inject into games or anti-cheat-protected processes to obtain object metadata;
- do not revive a user-visible virtual-cable/second-endpoint architecture merely to make object capture easier.

A negative result from this research is acceptable. If Windows exposes no supported third-party scene seam outside the spatial-format/provider path, preserve that as an architectural boundary and keep the 7.1 EFX route as the safe fallback.

## Omniphony's 22-direction field is different

The existing Omniphony Current-model support shell uses a richer internal full-sphere directional lattice. That is **renderer geometry**, not an authored Windows input format and not a replacement for the 8.1.4.4 semantic scene frame.

```text
source / Windows scene
        ↓
8.1.4.4-capable canonical static frame
+ continuous dynamic objects where supplied
        ↓
source-authority / provenance state
        ↓
Omniphony 22-direction or continuous rendering geometry as needed
        ↓
HRTF / ITD / distance / room / binaural processing
        ↓
stereo DAC output
```

The relationship is therefore:

```text
8.1.4.4 = standardized semantic skeleton
22-direction field = Omniphony rendering/support lattice
continuous objects = higher-precision source geometry when available
```

Do not expose the 22 directions to a game as if they were 22 authored source channels.

For stereo material, the internal field may be derived from bounded evidence while the finished stereo master remains protected. For authored multichannel or object material, supplied positions outrank inferred support geometry.

## Research result

The SciSpace pass supports the representation split above rather than a "maximum channel count wins" rule.

Relevant research on object-based audio, MPEG-H 3D Audio binaural rendering, higher-order Ambisonics, virtual loudspeaker rendering, and surround-with-height repeatedly supports retaining supplied directional/height/object information until the binaural stage. Research also supports richer full-3D spatial representations while showing that binaural reduction and virtual-loudspeaker layout can introduce timbral/localization artifacts when designed poorly.

The research does **not** establish Microsoft's exact four-lower-channel `.4` layer as a psychoacoustic optimum. Omniphony adopts those four lower anchors because they provide useful full-sphere semantics and match the Windows Spatial Audio static vocabulary. The denser 22-direction Omniphony field remains free to exceed that static skeleton where listening and measurement justify it.

The practical conclusion is:

> **one rich coordinate system, explicit provenance, minimum invention**

## GitHub implementation result

The implementation pass points to complementary reference families:

- `microsoft/Windows-driver-samples/audio/sysvad/APO` is the reference architecture for a componentized Windows APO associated with an audio endpoint. The Swap APO demonstrates realtime APO processing, custom-format support, format validation, system-effect registration, and componentized APO packaging/association.
- `microsoft/Xbox-ATG-Samples/UWPSamples/Audio/AdvancedSpatialSoundsUWP` demonstrates `ISpatialAudioClient`, static spatial objects, and dynamic-object capacity.
- `microsoft/Windows-universal-samples/Samples/SpatialSound` demonstrates application-level Windows spatial rendering.
- `kcat/openal-soft` is a useful adjacent mature renderer because it keeps a richer portable spatial engine separate from the platform-specific output backend instead of defining internal scene semantics by the final stereo endpoint.

This supports dual Windows ingress feeding one portable scene and one renderer:

```text
conventional Windows shared-mode PCM
        ↓
Omniphony endpoint-effect APO
        ↓
8.1.4.4-capable portable scene

supported Windows Spatial Audio scene seam
        ↓
static objects + dynamic objects
        ↓
8.1.4.4-capable portable scene

both
        ↓
one Omniphony renderer
        ↓
stereo FiiO endpoint
```

## Required validation matrix

Before calling the Windows spatial path complete, test at least:

```text
2.0
5.1
7.1
7.1 mapped into sparse 8.1.4.4 scene
7.1.4
7.1.4.4
8.1.4.4
one or more dynamic objects
native-Dolby-already-binaural bypass
```

For each relevant layout prove:

- channel/object identity is retained at ingress;
- authored/derived/empty provenance remains correct;
- no premature stereo downmix occurs before Omniphony when richer source data is available;
- no derived channel is mislabeled as authored;
- LFE remains semantically distinct;
- upper and lower channels remain distinct when supplied;
- dynamic objects keep continuous positions where supplied;
- output is exactly two channels to the physical headphone endpoint;
- already-binaural Dolby/native spatial output is not virtualized twice;
- block-size changes do not change spatial behavior;
- bypass/identity mode remains deterministic;
- physical listening agrees with the engineering result.

## Primary implementation and platform references

- Microsoft Windows Driver Samples, SysVAD APO: https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO
- Microsoft Xbox ATG Advanced Spatial Sounds sample: https://github.com/microsoft/Xbox-ATG-Samples/tree/main/UWPSamples/Audio/AdvancedSpatialSoundsUWP
- Microsoft UWP Spatial Sound sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound
- OpenAL Soft: https://github.com/kcat/openal-soft
- Microsoft preferred APO input format documentation: https://learn.microsoft.com/windows/win32/api/audioengineextensionapo/nf-audioengineextensionapo-iaudioprocessingobjectpreferredformatsupport-getpreferredinputformat
- Microsoft Spatial Sound overview and format/object limits: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft Spatial Audio object rendering/channel masks: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- Microsoft APO architecture: https://learn.microsoft.com/windows-hardware/drivers/audio/audio-processing-object-architecture
- Dolby Windows implementation guidance: https://professionalsupport.dolby.com/s/article/Windows-Implementation
