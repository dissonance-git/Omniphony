# Omniphony fork notice

This repository is a fork of the original **Omniphony** project by the upstream project maintainers at:

- https://github.com/mgth/Omniphony

The fork retains the original repository history and is distributed under the inherited **GPL-3.0-or-later** license. Existing upstream copyright, authorship, licensing and attribution remain in force.

## What came from upstream

The original project supplied the foundation on which this fork is built, including substantial work around:

- the spatial/object rendering engine;
- VBAP and speaker-layout machinery;
- binaural HRTF and ITD rendering;
- room/reflection and distance processing;
- audio input/output architecture;
- decoder/bridge interfaces;
- realtime control and OSC infrastructure;
- cross-platform integration;
- Omniphony Studio and associated visualization/control work;
- tests, documentation, fixtures and engineering accumulated in upstream history.

The fork should not imply that this body of work originated here.

## What this fork changes

`dissonance-git/Omniphony` is narrowing the inherited project around a different practical goal:

> a Windows-first stereo music enhancer that reconstructs a stable, convincing full-sphere auditory scene and renders it binaurally over headphones while preserving musical fidelity.

The fork also serves as the first practical consumer/testbed for the separate `dissonance-git/libaural` auditory-intelligence research project.

As this refactor proceeds, upstream subsystems that are not useful to the Windows stereo-to-binaural product may be removed. Useful upstream renderer machinery will be retained and modified rather than gratuitously rewritten.

## Other inherited experiments

The fork may also reimplement useful ideas from the owner's earlier private `dissonance-git/spatial-dsp` foobar2000 experiment. Those ideas are being migrated as inspectable algorithms rather than preserving the old stereo -> pseudo-7.1 -> external virtual-surround chain.

Third-party code or assets retain their own applicable licenses and attribution. No file should be assumed to become relicensed merely because it is incorporated into this fork.
