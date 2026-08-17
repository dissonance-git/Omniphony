# Windows spatial input contract

This note defines how Omniphony for Windows should interpret Windows audio layouts before the portable binaural renderer. It complements `docs/omniphony-for-windows.md` and exists to keep three different concepts separate:

1. conventional shared-mode PCM beds;
2. Windows Spatial Audio static/dynamic objects;
3. Omniphony's own internal support/rendering lattice.

## Governing rule

**Preserve the richest source representation Windows actually supplies. Do not invent a richer authored bed merely because Omniphony can render one.**

The physical headphone endpoint may remain stereo while the upstream Windows graph supplies richer spatial information.

```text
SOURCE TRUTH                         OMNIPHONY OUTPUT
stereo ---------------------------> binaural stereo
5.1 ------------------------------> binaural stereo
7.1 ------------------------------> binaural stereo
7.1.4 ----------------------------> binaural stereo
7.1.4.4 --------------------------> binaural stereo
8.1.4.4 --------------------------> binaural stereo
dynamic spatial objects ----------> binaural stereo
```

The FiiO/DAC side remains two physical channels throughout.

## Conventional game and media path

For ordinary Windows shared-mode applications and games, the primary compatibility target is conventional PCM:

```text
stereo
5.1
7.1
```

On Windows 11 23H2+, `IAudioProcessingObjectPreferredFormatSupport::GetPreferredInputFormat` is specifically documented for headphone virtualization. Microsoft gives the example of a stereo-rendering endpoint whose APO requests 7.1 input.

Therefore the Omniphony endpoint effect should negotiate and preserve authored 5.1/7.1 PCM before reducing it to binaural stereo.

A game that authors 7.1 must stay 7.1 at the Omniphony boundary. Do not inflate it to 7.1.4 or 8.1.4.4 by fabricating height or bottom channels.

This is the important compatibility route for games that expose a generic Home Theater / 7.1 output mode.

## Windows Spatial Audio path

Windows Spatial Audio is a richer source path than conventional shared-mode PCM. `ISpatialAudioClient` supports static spatial objects assigned to predefined speaker positions plus dynamic objects with arbitrary 3D positions.

Microsoft currently defines a maximum static spatial bed of **8.1.4.4**, seventeen static channels:

```text
horizontal
FL FR C LFE SL SR BL BR BC

upper
TFL TFR TBL TBR

lower
BFL BFR BBL BBR
```

Common useful subsets include:

```text
7.1       = 8 static channels
7.1.4     = 12 static channels
7.1.4.4   = 16 static channels
8.1.4.4   = 17 static channels
```

The portable Omniphony multichannel contract should therefore be capable of representing the full 8.1.4.4 static bed, even though many applications will provide less.

When a Spatial Audio-aware application supplies 7.1.4, 8.1.4.4, or dynamic object positions, preserve that supplied geometry. Do not collapse it to 7.1 merely because 7.1 is the conventional compatibility path.

The endpoint-effect APO and Spatial Audio ingestion may require different Windows host seams. That is acceptable. They must converge on the same portable Omniphony scene/channel semantics rather than duplicating the renderer.

## 8.1.4.4 is a ceiling, not a default

Omniphony should **support up to 8.1.4.4 as the highest conventional static Windows spatial bed**.

It should not request or synthesize 8.1.4.4 indiscriminately.

```text
source says stereo    -> preserve stereo authority
source says 5.1       -> preserve 5.1
source says 7.1       -> preserve 7.1
source says 7.1.4     -> preserve 7.1.4
source says 8.1.4.4   -> preserve 8.1.4.4
source supplies objects -> preserve object positions
```

This keeps source authority above renderer capability.

## Omniphony's 22-direction field is different

The existing Omniphony Current-model support shell uses a richer internal full-sphere directional lattice. That is **renderer geometry**, not an authored Windows input format.

Do not expose those 22 directions to a game as if they were 22 physical or authored source channels.

For stereo material, the internal field may be derived from bounded evidence while the finished stereo master remains protected. For authored multichannel or object material, those supplied positions outrank inferred support geometry.

```text
Windows/source channels or objects
        ↓
source-authoritative scene representation
        ↓
Omniphony internal rendering geometry as needed
        ↓
HRTF / room / binaural processing
        ↓
stereo DAC output
```

## Research result

The SciSpace pass supports this separation rather than a "maximum channel count wins" rule.

Relevant research repeatedly treats multichannel, object-based, and Ambisonic representations as spatial information to be preserved into binaural rendering. Work on object-aware binaural externalization, MPEG-H 3D Audio binaural rendering, higher-order Ambisonics, and surround-with-height all reinforces the value of retaining supplied directional/height structure until the binaural stage. The literature also shows that richer spatial resolution can improve localization/spatial impression, but does not justify fabricating source channels that were never authored.

The practical conclusion is:

> **rich capability, conservative interpretation**

Omniphony should understand rich beds and objects, while using only the source truth actually available.

## GitHub implementation result

The Microsoft samples point to two distinct implementation families:

- `microsoft/Windows-driver-samples/audio/sysvad/APO` is the reference architecture for a componentized Windows APO associated with an audio endpoint. The Swap APO demonstrates realtime APO processing, custom-format support, format validation, system-effect registration, and componentized APO packaging/association.
- `microsoft/Xbox-ATG-Samples/UWPSamples/Audio/AdvancedSpatialSoundsUWP` demonstrates `ISpatialAudioClient`, static spatial objects, and dynamic-object capacity. Its sample static mask is a 7.1.4 bed and it queries dynamic-object availability from Windows at runtime.
- `microsoft/Windows-universal-samples/Samples/SpatialSound` demonstrates HRTF-based spatial rendering as a separate application-level spatial-audio path.

This supports a dual Windows ingress architecture feeding one portable renderer:

```text
conventional Windows shared-mode PCM
        ↓
Omniphony endpoint-effect APO
        ↓
portable Omniphony scene/channel contract

Windows Spatial Audio
        ↓
spatial static/dynamic object host seam
        ↓
portable Omniphony scene/channel contract

both
        ↓
one binaural renderer
        ↓
stereo FiiO endpoint
```

## Required validation matrix

Before calling the Windows path complete, test at least:

```text
2.0
5.1
7.1
7.1.4
7.1.4.4
8.1.4.4
one or more dynamic objects
```

For each layout prove:

- channel/object identity is retained at ingress;
- no premature stereo downmix occurs;
- no invented channels appear;
- LFE remains semantically distinct;
- upper and lower channels remain distinct when supplied;
- output is exactly two channels to the physical headphone endpoint;
- block-size changes do not change spatial behavior;
- bypass/identity mode remains deterministic;
- physical listening agrees with the engineering result.

## Primary implementation references

- Microsoft Windows Driver Samples, SysVAD APO: https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO
- Microsoft Xbox ATG Advanced Spatial Sounds sample: https://github.com/microsoft/Xbox-ATG-Samples/tree/main/UWPSamples/Audio/AdvancedSpatialSoundsUWP
- Microsoft UWP Spatial Sound sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound
- Microsoft preferred APO input format documentation: https://learn.microsoft.com/windows/win32/api/audioengineextensionapo/nf-audioengineextensionapo-iaudioprocessingobjectpreferredformatsupport-getpreferredinputformat
- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft Spatial Audio object rendering/channel masks: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
