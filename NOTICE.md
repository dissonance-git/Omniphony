# Omniphony fork notice

This repository is derived from the original **Omniphony** project by the upstream maintainers at:

- https://github.com/mgth/Omniphony

The repository retains upstream history and is distributed under the inherited **GPL-3.0-or-later** license. Existing upstream copyright, authorship, licensing, and attribution remain in force.

## Upstream foundation

The original project supplied substantial technical foundations that remain important to this fork, including work around:

- spatial and object rendering;
- VBAP and speaker-layout machinery;
- binaural HRTF and ITD rendering;
- room, reflection, and distance processing;
- audio input/output infrastructure;
- decoder and bridge interfaces;
- realtime control infrastructure;
- cross-platform integration;
- visualization and control tooling;
- tests, documentation, fixtures, and engineering accumulated in upstream history.

This fork does not claim that body of work as originating here.

## Scope of this fork

`dissonance-git/Omniphony-Headphones` develops Omniphony toward a free and open-source spatial audio renderer for headphones.

The product goal is one renderer that can accept progressively richer source representations while preserving their authority:

```text
stereo
→ bounded spatial inference

5.1 / 7.1 / height PCM
→ authored channel geometry

8.1.4.4 static spatial scenes
→ authored fixed spatial roles

dynamic spatial objects
→ authored object identity and continuous position

all
→ one Omniphony spatial renderer
→ one final binaural headphone output
```

Windows is the first system-wide host, while the portable scene model and renderer are intended to remain host-independent.

The fork extends the upstream foundation with work including stereo presentation, native multichannel ingress, Windows APO hosting, source-authority semantics, canonical spatial-scene handling, realtime integration, validation, and system-wide headphone rendering.

The project may remove or replace inherited subsystems that do not serve the current renderer architecture, while preserving attribution for retained or derived upstream work.

## Third-party code, data, and assets

Third-party code, datasets, HRTFs, models, media, and other assets retain their own applicable licenses and attribution requirements. Incorporation into this repository does not automatically relicense them.

Contributors should verify redistribution, attribution, and compatibility requirements before adding third-party material.

## References to proprietary spatial-audio products

Names such as Dolby Atmos for Headphones, DTS Headphone:X, Windows Sonic, Sony 360-related systems, and Waves Nx may appear in documentation as interoperability references, comparison targets, or examples of the broader headphone-spatial-renderer product class.

Such references do not imply ownership of those technologies, incorporation of their proprietary implementations, or endorsement or affiliation by their respective owners.
