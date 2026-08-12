# Listening history

This file preserves physical listening evidence from retired Omniphony comparison paths.

These are **historical research controls**, not user-facing listening modes. The tray and launcher no longer expose a profile selector. Normal Windows playback uses one **Current model**.

## 2026-08-12 · profile comparison

The first tray-profile comparison produced a strong compression result:

- the non-PRTF variants were not clearly distinguishable in ordinary listening;
- the music still sounded good across those variants;
- the structural PRTF path was clearly distinguishable, but in the wrong direction: it sounded **tinnier and worse**;
- the hybrid direct-height path did not produce an obvious perceptual gain and therefore did not justify its added routing complexity.

This result did not prove that the underlying mechanisms are perceptually identical in general. It established that, under the tested system and listening conditions, those profile-level differences did not earn separate product modes.

## 2026-08-12 · measured-HRTF early reflections

A later build replaced the lightweight analytic first-order reflection panner with a bounded six-bus measured-HRTF early field while preserving the protected master and the rest of the successful music path.

Listening described this path as **a little better**, while explicitly noting that placebo could not be excluded.

The project therefore records the result conservatively:

> slight subjective preference; not a demonstrated perceptual law.

The path is nevertheless adopted provisionally as the **Current model** because:

- it was not heard as worse;
- it represents a materially different and more physically meaningful early-field mechanism;
- its reflection energy is approximately power-matched rather than simply louder;
- it preserves the existing protected-master architecture;
- carrying many weakly distinguished product profiles no longer helps development.

The former tray label `Externalization` is retired. The mechanism is now simply part of the Current model.

## Current model inherited from the comparison

The Current model retains:

- protected finished stereo master;
- coherent low-frequency/body foundation;
- analysis-only stereo evidence extraction;
- derived 7.1.4 support field;
- coherent elevation transfer;
- grid-aligned +60-degree upper shell;
- measured SAF/KEMAR binaural rendering;
- current room balance and short late field;
- support-only spectral compensation;
- fixed output makeup and stereo-linked peak safety;
- Windows realtime continuity guards.

Its first-order early field now uses:

```text
support lanes
    ↓
first-order image timing / wall filtering
    ↓
six wall-grouped buses
    ↓
measured SAF/KEMAR HRTF + ITD
    ↓
linear support sum
```

The primary engine's older analytic reflection bank is disabled on this path so the same early energy is not routed twice.

## Retired controls

### `control`

Earlier cascaded-binaural reference topology. Useful only for historical comparison.

### `all`

Previous Current model before measured-HRTF wall-bus reflections were promoted.

This path established much of the current successful sound but no longer represents normal playback.

### `hybrid`

Split the four height evidence lanes into a direct measured-HRTF path while leaving the surrounding eight lanes in the cascaded world.

Mechanical tests established exclusive routing and aligned first arrivals, but physical listening did not reveal a clear benefit over the then-current model. The extra runtime complexity was not promoted.

### `direct`

Rendered all evidence lanes through direct HRTF instead of the virtual-speaker cascade.

No clear listening advantage was established in the profile pass.

### `external`

The name was used twice during research.

The earlier version was a room-balance control and did not earn retention.

The later version introduced the measured-HRTF six-bus early field. That **mechanism** is now promoted into the Current model, but `external` is no longer a product/profile concept.

### `prtf`

Structural PRTF alternative to the measured KEMAR path.

Physical listening described it as **tinnier and worse**. This is a retained negative result: a different or more structural pinna model does not automatically improve elevation or externalization.

### `close`

Shorter-distance / smaller-room control. No clear listening benefit was established in the profile pass.

### `tracked`

Head-tracking-ready configuration. Without live head-motion input this was never a valid head-motion comparison, so no perceptual conclusion about world locking follows from the static profile test.

### `diffuse`

Deliberately stronger diffuse late-field control. It did not earn a separate listening mode.

## Promotion rule going forward

The project no longer keeps a broad tray matrix of speculative modes.

New audible mechanisms should normally enter as bounded research challengers and then either:

```text
beat / clearly improve the Current model
→ promote into Current model

fail to improve it
→ retain only the useful negative evidence

remain ambiguous
→ do not multiply product modes
```

The next isolated audible frontier is **transient-aware live-drum presentation**. Source/instrument awareness from libaural should follow only after the spatial transient mechanism independently earns itself, so two unknown mechanisms are not evaluated at once.
