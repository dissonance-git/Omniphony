# Omniphony fork notice

This repository is a fork of the original **Omniphony** project by the upstream maintainers at:

- https://github.com/mgth/Omniphony

The fork retains the original repository history and is distributed under the inherited **GPL-3.0-or-later** license. Existing upstream copyright, authorship, licensing, and attribution remain in force.

## What came from upstream

The original project supplied the foundation on which this fork is built, including substantial work around:

- spatial/object rendering;
- VBAP and speaker-layout machinery;
- binaural HRTF and ITD rendering;
- room/reflection and distance processing;
- audio input/output architecture;
- decoder/bridge interfaces;
- realtime control and OSC infrastructure;
- cross-platform integration;
- Omniphony Studio and associated visualization/control work;
- tests, documentation, fixtures, and engineering accumulated in upstream history.

The fork must not imply that this body of work originated here.

## What this fork changes

`dissonance-git/Omniphony-Headphones` narrows the inherited project around a different practical goal:

> **a Windows-first binaural spatial-audio system for headphones, focused first on making the already-good Omniphony renderer practical for ordinary music and normal Windows listening.**

The fork preserves the upstream binaural sound as a protected perceptual floor while building native Windows transport, simpler product behavior, ordinary-stereo presentation, and later optional calibration/personalization around it.

The separate `libaural` project may later provide bounded research evidence for adaptive presentation, but Omniphony for Headphones does not depend on libaural to render or play audio.

As the fork contracts, upstream subsystems that do not serve the Windows headphone product, protected renderer, or deterministic validation may be removed. Useful upstream renderer machinery should be retained and modified rather than gratuitously rewritten.

## Other inherited experiments

The fork also reimplements selected ideas from the owner's earlier `spatial-dsp` / Real3D foobar2000 experiment. Those ideas are migrated as inspectable evidence/rendering mechanisms rather than preserving the old stereo → pseudo-7.1 → external virtual-surround topology.

Third-party code or assets retain their own applicable licenses and attribution. No file should be assumed to become relicensed merely because it is incorporated into this fork.
