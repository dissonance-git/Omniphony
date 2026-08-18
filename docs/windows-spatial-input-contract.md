# Windows spatial input contract

This note defines how Omniphony for Windows interprets Windows audio layouts before the portable binaural renderer. It keeps four concepts separate:

1. conventional shared-mode PCM beds;
2. Windows Spatial Audio static/dynamic objects;
3. Omniphony's canonical static scene frame;
4. Omniphony's denser internal support/rendering lattice.

## Governing rule

**Preserve the richest source representation Windows actually supplies. Represent it inside one canonical 8.1.4.4-capable scene without pretending missing channels were authored.**

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

The FiiO/DAC side remains two physical channels throughout.

## Canonical static scene: 8.1.4.4

Omniphony's **ideal full fixed static scene contract is 8.1.4.4**: seventeen semantic spatial anchors matching the complete predefined static-channel vocabulary exposed by Microsoft Spatial Sound for headphone spatial renderers.

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

Every static anchor therefore carries source authority explicitly:

```text
AUTHORED   source or host supplied this signal/position
DERIVED    Omniphony inferred bounded support for this anchor
EMPTY      no trustworthy signal is assigned here
```

The central law is:

> **8.1.4.4 is the base coordinate frame, not the default claim about what the source contained.**

Dynamic XYZ objects sit above this fixed skeleton when available. They are not extra fixed channels and should retain continuous position rather than being prematurely snapped to 8.1.4.4 anchors.

## Accepted conventional Windows baseline

The physically accepted Windows production baseline is conventional shared-mode 7.1 PCM through the Omniphony stream SFX while the hardware endpoint remains stereo.

Accepted 2026-08-18 machine evidence established:

```text
physical endpoint mix
48 kHz / 32-bit / stereo

+

shared client input
48 kHz / float32 / 7.1

+

Omniphony stream SFX active
stereo rollback EFX absent
```

The decisive client-boundary evidence is:

```text
SHARED_7_1_FORMAT_SUPPORTED      CHANNELS=8 BITS=32 RATE=48000
SHARED_7_1_INITIALIZE_OK         INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2
NATIVE_SURROUND_CLIENT_FORMAT_OK INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2
NATIVE_SURROUND_SFX 1
NATIVE_SURROUND_EFX 0
```

`IAudioClient::GetMixFormat` remaining stereo is expected because it describes the endpoint/shared engine mix. The richer authored client stream exists upstream of that stereo endpoint mix and is reduced by Omniphony's format-changing SFX.

For conventional games, the production topology is therefore:

```text
game Home Theater / surround mix
        ↓
authored stereo / 5.1 / 7.1 shared client stream
        ↓
Omniphony stream SFX
        ↓
canonical source scene
        ↓
Current shell / binaural renderer
        ↓
stereo endpoint mix
        ↓
DAC / headphones
```

For a 7.1 game stream:

```text
FL FR C LFE SL SR BL BR = AUTHORED
BC TFL TFR TBL TBR BFL BFR BBL BBR = EMPTY or bounded DERIVED
```

Do not label derived height, bottom, or back-center content as authored merely because the internal frame contains those positions.

The intended conventional-game configuration is:

```text
Windows Spatial Sound: OFF
game mix: Home Theater / surround
in-game headphone virtualization: OFF
```

This avoids double binaural rendering and lets Omniphony own the final headphone render.

## Preferred-format semantics

On Windows 11 23H2+, `IAudioProcessingObjectPreferredFormatSupport::GetPreferredInputFormat` is specifically documented for headphone virtualization, including the case where a stereo-rendering endpoint's APO requests 7.1 input.

Omniphony implements that contract in `OmniphonyStreamAPO.dll`.

The important distinction is:

```text
client-facing authored input may be 7.1
while
physical endpoint/shared engine mix remains stereo
```

Therefore production acceptance must test the actual client stream boundary, not require the DAC's `GetMixFormat` result to become eight channels.

## Richer conventional PCM

The stream APO/native-bed path also implements and regression-tests authored 7.1.4 processing:

```text
authored 7.1.4
→ twelve-channel input
→ native-bed realtime ABI
→ authored source coordinates
→ Current shell / binaural
→ stereo output
```

That is implementation evidence, not yet a claim that arbitrary Windows applications will open 7.1.4 shared streams on the current host. Physical application-level proof remains required before promoting a richer conventional bed above the accepted 7.1 baseline.

## Windows Spatial Audio path

Windows Spatial Audio is a richer source path than conventional shared-mode PCM. `ISpatialAudioClient` supports static spatial objects assigned to predefined speaker positions plus dynamic objects with arbitrary 3-D positions.

For headphone spatial renderers, the full predefined static vocabulary reaches **8.1.4.4 / 17 static positions**. That is why Omniphony's ideal static scene vocabulary remains 8.1.4.4 even though the current conventional production baseline is 7.1.

When a Spatial Audio-aware application supplies 7.1.4, 7.1.4.4, 8.1.4.4, or dynamic object positions, preserve that supplied geometry and authority. Do not collapse it to 7.1 merely because 7.1 is the conventional production baseline.

The stream-SFX path and Spatial Audio ingestion may require different Windows host seams. That is acceptable. They must converge on the same portable Omniphony scene semantics rather than duplicating the renderer.

## Dynamic objects are parallel to the static frame

Dynamic spatial objects are not eighteenth, nineteenth, or later fixed channels. They carry continuous 3-D position and may move over time.

```text
canonical static frame: 17 semantic anchors
        +
dynamic object layer: arbitrary x/y/z objects
        ↓
one portable Omniphony scene
```

When real object coordinates are supplied, preserve them continuously as far into rendering as possible. Do not snap a moving object prematurely to the nearest `TFL`, `SL`, `BBR`, or other static anchor merely to fit the 8.1.4.4 bed.

Source authority therefore increases approximately as:

```text
stereo evidence
    < authored horizontal bed
    < authored height/lower bed
    < supplied continuous object position / scene field
```

This is an authority ordering, not a statement that every object mix necessarily sounds better than every channel mix.

## Dolby / Windows spatial interoperability target

Omniphony should work with Dolby or other Windows Spatial Sound content **through supported Windows interfaces wherever the platform exposes a trustworthy seam**. It should not require a second internal renderer when Microsoft's Spatial Audio layer already presents compatible static/object semantics.

There are three distinct cases.

### 1. Raw static/object scene reaches Omniphony

Preferred long-term path:

```text
Windows spatial application
        ↓
8.1.4.4 static objects + dynamic x/y/z objects
        ↓
Omniphony canonical scene / object layer
        ↓
Omniphony HRTF / distance / room / binaural renderer
        ↓
stereo headphones
```

### 2. Sonic / Dolby / DTS has already rendered the scene

If another headphone renderer has already converted the scene to final binaural stereo before Omniphony receives it, Omniphony must **not spatialize it again**.

```text
already-binaural stereo
        ↓
Omniphony clean spatial bypass
or explicitly validated non-spatial correction only
        ↓
headphones
```

A trustworthy detection signal is required before automating this policy. Stereo channel count alone is not enough.

### 3. Encoded spatial media

Prefer supported operating-system decode/render facilities over reverse-engineering proprietary bitstream/object codecs merely to reach equivalent source semantics.

If a future supported seam exposes decoded bed/object geometry, ingest it. If the platform exposes only the final binaural result, follow case 2.

## Current hard boundary: conventional SFX is not proven raw-object ingress

The accepted 7.1 stream SFX is a conventional PCM path. It proves authored multichannel shared-client ingress, not raw Spatial Audio object interception.

The public Windows documentation reviewed so far does **not** establish that an ordinary third-party system effect can recover another process's original `ISpatialAudioClient` object identities and XYZ metadata after the Windows spatial renderer has consumed them.

Therefore:

- do not claim raw Atmos/Spatial Audio object ingestion is solved by the accepted 7.1 SFX;
- do not reconstruct object positions from already-rendered binaural audio and call them native objects;
- preserve the accepted 7.1 SFX as the robust production fallback;
- investigate a supported richer spatial ingress in parallel;
- do not hook/inject into games or anti-cheat-protected processes to obtain object metadata;
- do not revive a user-visible virtual-cable/second-endpoint architecture merely to make object capture easier.

A negative result from raw-object research is acceptable. If Windows exposes no supported third-party scene seam, preserve that as an architectural boundary and keep conventional 7.1 as the safe production path.

## Omniphony's 22-direction field is different

The existing Current-model support shell uses a richer internal full-sphere directional lattice. That is **renderer geometry**, not an authored Windows input format and not a replacement for the 8.1.4.4 semantic scene frame.

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

The relationship is:

```text
8.1.4.4 = ideal standardized fixed semantic skeleton
22-direction field = Omniphony rendering/support lattice
continuous objects = higher-precision source geometry when available
```

Do not expose the 22 directions to a game as if they were 22 authored source channels.

## Validation matrix

The conventional production baseline requires:

```text
2.0
5.1 compatibility
7.1 physically accepted shared-client path
7.1 mapped into sparse 8.1.4.4 scene
7.1.4 renderer/APO regression
```

The richer spatial frontier requires:

```text
7.1.4 application-level ingress proof
7.1.4.4
8.1.4.4
one or more dynamic objects
already-binaural bypass
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
- already-binaural native spatial output is not virtualized twice;
- block-size changes do not change spatial behavior;
- bypass/identity mode remains deterministic;
- physical listening agrees with the engineering result.

## Current product frontier

The next sequence is:

```text
accepted 7.1 client/SFX baseline
        ↓
prove Overwatch Home Theater actually opens/populates that authored 7.1 path
        ↓
retain as regression floor
        ↓
prove supported raw Windows Spatial Audio ingress, if available
        ↓
8.1.4.4 static objects + dynamic XYZ objects
        ↓
same Omniphony renderer
```

The new baseline does not reduce the long-term target. It gives the project a stable floor from which to attack the richer source representation.

## Primary platform references

- Microsoft Windows Driver Samples, SysVAD APO: https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO
- Microsoft Xbox ATG Advanced Spatial Sounds sample: https://github.com/microsoft/Xbox-ATG-Samples/tree/main/UWPSamples/Audio/AdvancedSpatialSoundsUWP
- Microsoft UWP Spatial Sound sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound
- Microsoft preferred APO input format documentation: https://learn.microsoft.com/windows/win32/api/audioengineextensionapo/nf-audioengineextensionapo-iaudioprocessingobjectpreferredformatsupport-getpreferredinputformat
- Microsoft Spatial Sound overview and format/object limits: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft Spatial Audio object rendering/channel masks: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- Microsoft APO architecture: https://learn.microsoft.com/windows-hardware/drivers/audio/audio-processing-object-architecture
