# Windows spatial input contract

This document defines how Omniphony for Windows interprets source layouts before the portable binaural renderer.

It keeps four concepts separate:

1. conventional shared-mode PCM beds;
2. Windows Spatial Audio static and dynamic objects;
3. Omniphony's canonical static scene frame;
4. Omniphony's denser internal rendering geometry.

## Governing rule

> **Preserve the richest source representation Windows actually supplies. Represent it inside one canonical 8.1.4.4-capable scene without pretending missing channels were authored.**

The physical headphone endpoint may remain stereo while the upstream Windows graph supplies richer source information.

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

## One renderer across source types

Stereo, surround, height beds, and spatial objects are not separate rendering products.

```text
stereo
→ preserve master
→ infer missing geometry carefully
→ Omniphony

5.1 / 7.1 / height PCM
→ preserve authored speaker geometry
→ infer less
→ same Omniphony renderer

8.1.4.4 static objects + dynamic XYZ objects
→ preserve supplied scene geometry
→ avoid reconstructing what is already known
→ same Omniphony renderer
```

The richer the input, the less spatial invention Omniphony should perform.

## Canonical static scene: 8.1.4.4

Omniphony's ideal fixed Windows scene contract is **8.1.4.4**: seventeen semantic spatial anchors matching the complete predefined static-channel vocabulary exposed by Microsoft Spatial Sound for headphone spatial renderers.

```text
horizontal
FL FR C LFE SL SR BL BR BC

upper
TFL TFR TBL TBR

lower
BFL BFR BBL BBR
```

Common source subsets map naturally into this frame:

```text
7.1       = FL FR C LFE SL SR BL BR
7.1.4     = 7.1 + TFL TFR TBL TBR
7.1.4.4   = 7.1.4 + BFL BFR BBL BBR
8.1.4.4   = 7.1.4.4 + BC
```

The frame is always available as a coordinate vocabulary. The signals occupying it are not always authored.

Every static anchor therefore carries source authority explicitly:

```text
AUTHORED   source or host supplied this signal / position
DERIVED    Omniphony inferred bounded support for this anchor
EMPTY      no trustworthy signal is assigned here
```

> **8.1.4.4 is the base coordinate frame, not the default claim about what the source contained.**

Dynamic XYZ objects sit beside this fixed skeleton when available. They are not extra fixed channels and should retain continuous position rather than being snapped prematurely to static anchors.

## Conventional Windows PCM baseline

The accepted Windows production baseline is conventional shared-mode multichannel PCM through the Omniphony stream SFX while the physical headphone endpoint remains stereo.

For authored 7.1:

```text
48 kHz / float32 / 7.1 client stream
        ↓
Omniphony stream SFX
        ↓
FL FR C LFE SL SR BL BR = AUTHORED
        ↓
canonical source scene
        ↓
Current spatial renderer
        ↓
stereo endpoint mix
```

The accepted-state client boundary is:

```text
shared 7.1 format supported
shared 7.1 client initializes successfully
stream SFX remains attached
stereo rollback EFX is absent
physical endpoint remains stereo
```

`IAudioClient::GetMixFormat` remaining stereo is expected because it describes the endpoint/shared engine mix. The richer authored client stream exists upstream of that final endpoint mix and is reduced by Omniphony's format-changing SFX.

Do not label derived height, lower, or back-center content as authored merely because the canonical frame contains those positions.

## Preferred-format semantics

On Windows 11 23H2+, `IAudioProcessingObjectPreferredFormatSupport::GetPreferredInputFormat` is documented for headphone virtualization, including the case where a stereo-rendering endpoint's APO requests 7.1 input.

Omniphony implements that contract in `OmniphonyStreamAPO.dll`.

The important distinction is:

```text
client-facing authored input may be multichannel
while
physical endpoint/shared engine mix remains stereo
```

Production acceptance must therefore test the actual client stream boundary rather than require the DAC's `GetMixFormat` result to become multichannel.

## Richer conventional PCM

The stream APO/native-bed path also implements and regression-tests authored 7.1.4 processing:

```text
authored 7.1.4
→ twelve-channel input
→ native-bed realtime ABI
→ authored source coordinates
→ Current spatial renderer
→ stereo output
```

A renderer fixture proves format handling. It is distinct from proving that an arbitrary Windows application will open that exact richer shared stream on a given host.

## Windows Spatial Audio path

Windows Spatial Audio is a richer source path than conventional shared-mode PCM. `ISpatialAudioClient` supports static spatial objects assigned to predefined speaker positions plus dynamic objects with arbitrary 3-D positions.

For headphone spatial renderers, the full predefined static vocabulary reaches **8.1.4.4 / 17 static positions**. This is why Omniphony's ideal static scene vocabulary remains 8.1.4.4 even though the conventional shared-client baseline is 7.1.

When a spatial-aware application supplies 7.1.4, 7.1.4.4, 8.1.4.4, or dynamic object positions, preserve that supplied geometry and authority. Do not collapse it to 7.1 merely because 7.1 is the conventional compatibility floor.

The stream-SFX path and Spatial Audio ingestion may require different Windows host seams. They must converge on the same portable Omniphony scene semantics and the same final renderer.

## Dynamic objects are parallel to the static frame

Dynamic spatial objects are not eighteenth, nineteenth, or later fixed channels. They carry continuous 3-D position and may move over time.

```text
canonical static frame: 17 semantic anchors
        +
dynamic object layer: arbitrary x/y/z objects
        ↓
one portable Omniphony scene
```

When real object coordinates are supplied, preserve them continuously as far into rendering as possible. Do not snap a moving object prematurely to the nearest fixed anchor merely to fit the 8.1.4.4 bed.

Source authority increases approximately as:

```text
stereo evidence
    < authored horizontal bed
    < authored height/lower bed
    < supplied continuous object position / scene field
```

This is an authority ordering, not a claim that every object mix necessarily sounds better than every channel mix.

## Interoperability with other spatial renderers

Omniphony aims to occupy the final headphone-rendering role itself when a trustworthy pre-render spatial seam is available.

There are three distinct cases.

### Raw static/object scene reaches Omniphony

Preferred path:

```text
spatial application
        ↓
8.1.4.4 static objects + dynamic x/y/z objects
        ↓
Omniphony canonical scene / object layer
        ↓
Omniphony HRTF / distance / room / binaural renderer
        ↓
stereo headphones
```

### Another renderer has already produced binaural stereo

If Windows Sonic, Dolby, DTS, or another headphone renderer has already converted the scene to final binaural stereo before Omniphony receives it, Omniphony must not spatialize it again.

```text
already-binaural stereo
        ↓
Omniphony spatial bypass
or explicitly validated non-spatial correction only
        ↓
headphones
```

A trustworthy detection signal is required before automating this policy. Stereo channel count alone is not sufficient.

### Encoded spatial media

Prefer supported operating-system decode/render facilities over reverse-engineering proprietary bitstream or object codecs merely to reach equivalent source semantics.

If a supported seam exposes decoded bed/object geometry, ingest it. If the platform exposes only the final binaural result, follow the already-binaural path.

## Hard boundary: conventional SFX is not raw-object ingress

The accepted stream SFX is a conventional PCM path. It proves authored multichannel shared-client ingress, not raw Spatial Audio object interception.

Public Windows documentation does not establish that an ordinary third-party system effect can recover another process's original `ISpatialAudioClient` object identities and XYZ metadata after the active spatial renderer has consumed them.

Therefore:

- do not claim raw Atmos/Spatial Audio object ingestion is solved by conventional multichannel SFX;
- do not reconstruct object positions from already-rendered binaural audio and call them native objects;
- preserve the conventional multichannel SFX as the robust fallback;
- investigate a supported richer spatial ingress independently;
- do not hook or inject into protected applications to obtain object metadata;
- do not require a user-visible virtual cable or second playback endpoint solely to capture objects.

If Windows exposes no supported third-party scene seam, preserve that as an architectural boundary rather than fabricating one.

## Omniphony's 22-direction field is different

The Current support shell uses a denser internal full-sphere directional lattice. That is **renderer geometry**, not an authored Windows input format and not a replacement for the 8.1.4.4 semantic scene frame.

```text
source / Windows scene
        ↓
8.1.4.4-capable canonical static frame
+ continuous dynamic objects where supplied
        ↓
source authority
        ↓
Omniphony 22-direction or continuous rendering geometry
        ↓
HRTF / ITD / distance / room / binaural processing
        ↓
stereo output
```

The relationship is:

```text
8.1.4.4 = standardized fixed semantic skeleton
22-direction field = Omniphony rendering/support lattice
continuous objects = higher-precision source geometry when available
```

Do not expose the 22 directions to a source application as if they were twenty-two authored input channels.

## Validation matrix

Conventional source validation includes:

```text
2.0
5.1
7.1
7.1 mapped into sparse 8.1.4.4 scene
7.1.4 renderer/APO regression
```

Richer spatial validation includes:

```text
7.1.4 application-level ingress
7.1.4.4
8.1.4.4
one or more dynamic objects
already-binaural bypass
```

For each relevant layout prove:

- channel/object identity is retained at ingress;
- AUTHORED / DERIVED / EMPTY provenance remains correct;
- no premature stereo downmix occurs before Omniphony when richer source data is available;
- no derived channel is mislabeled as authored;
- LFE remains semantically distinct;
- upper and lower channels remain distinct when supplied;
- dynamic objects keep continuous positions where supplied;
- output is exactly two channels to the physical headphone endpoint;
- already-binaural spatial output is not virtualized twice;
- block-size changes do not change spatial behavior;
- bypass/identity mode remains deterministic;
- physical listening agrees with the engineering result.

## Platform references

- Microsoft Windows Driver Samples, SysVAD APO: https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO
- Microsoft Xbox ATG Advanced Spatial Sounds sample: https://github.com/microsoft/Xbox-ATG-Samples/tree/main/UWPSamples/Audio/AdvancedSpatialSoundsUWP
- Microsoft UWP Spatial Sound sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound
- Microsoft preferred APO input format documentation: https://learn.microsoft.com/windows/win32/api/audioengineextensionapo/nf-audioengineextensionapo-iaudioprocessingobjectpreferredformatsupport-getpreferredinputformat
- Microsoft Spatial Sound overview and format/object limits: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft Spatial Audio object rendering/channel masks: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- Microsoft APO architecture: https://learn.microsoft.com/windows-hardware/drivers/audio/audio-processing-object-architecture
