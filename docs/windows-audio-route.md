# Windows audio route

This document owns the **Windows host/transport decision**. It does not own the portable Omniphony core.

The root `README.md` owns product intent and priority.

Core rule:

> **Windows is the first host for Omniphony, not the architecture of Omniphony.**

The Windows layer should use native Windows facilities aggressively where they help, then translate their results into platform-neutral Omniphony stream contracts.

---

## 1. Portable/core boundary

The renderer/core should conceptually receive:

```text
InputStream
  id
  sample_rate
  channel_layout
  PCM
  optional spatial/object metadata
  timing / generation
```

and produce:

```text
binaural stereo PCM
```

The core must not know about:

```text
WASAPI
ASIO
VB-Audio
Windows endpoint names
Windows sessions
virtual-device implementation details
```

Those belong here, in the Windows host.

---

## 2. Main Windows use case

Ordinary stereo music is the dominant everyday path.

The finished user should be able to keep the physical DAC/headphones as ordinary 2-channel output:

```text
foobar / Spotify / browser
→ stereo source
→ Omniphony
→ binaural stereo
→ physical 2.0 DAC/headphones
```

The listener must not be required to configure Windows as 7.1 merely to hear stereo music correctly.

Richer source layouts may exist upstream while the physical output remains stereo.

---

## 3. Concurrent source-layout requirement

Windows may have several active applications at once:

```text
foobar       stereo
Overwatch    native surround / home-theater bed
chat         mono / stereo
```

These must coexist without a global Omniphony channel mode.

Target:

```text
Stream A { stereo }
Stream B { 7.1 }
Stream C { mono }
        ↓
portable Omniphony core
        ↓
binaural stereo
```

A surround game starting must not reconfigure a playing stereo song.

A stereo song playing beside a surround game must not cause the game to be flattened unnecessarily.

The current loopback prototype receives a platform mix, so it does not yet preserve ideal per-session source boundaries. That is acceptable for proving transport, not the final architecture.

Future Windows integration should preserve per-session/per-source truth when the APIs and chosen route permit it.

---

## 4. Single-path law

Ordinary playback must reach the listener **once**.

Correct:

```text
source
→ Omniphony
→ physical headphones
```

Forbidden:

```text
source ─────────────→ physical headphones
   └→ Omniphony ───→ physical headphones
```

Two copies with even a small delay can create comb filtering, thin/hollow tone, hallway character and echo.

Therefore:

> **No listening comparison is trustworthy until the physical route is proven single.**

---

## 5. Incumbent coexistence / migration law

The existing HeSuVi/Hi-Fi/ASIO setup was difficult to assemble and remains valuable as a reference.

Do not require uninstalling it during development.

Use:

```text
keep installed
→ disable one active stage
→ let Omniphony replace that function
→ verify
→ remove old component only when obsolete
```

Current installed/reference chain:

```text
foobar DSP
→ 5.1-side upmix
→ VB-Audio / Hi-Fi Cable
→ HeSuVi / DTS Virtual:X
→ ASIO Bridge / FiiO ASIO
→ FiiO
→ Noire X
```

Installed does not mean active.

For a clean Omniphony test, old forwarding that can also reach the FiiO must be stopped/bypassed while leaving the software installed.

---

## 6. First live prototype result

On 2026-08-10 the native app prototype successfully played arbitrary Windows/foobar audio through Omniphony to the real FiiO/headphones.

That proves:

```text
Omniphony.exe
→ hidden worker
→ Windows live capture
→ protected Omniphony renderer
→ FiiO
→ headphones
```

Observed first listen:

```text
audio works
but sounded:
- tinny
- hallway-like
- less bubble-like than desired
- small echo remained after OFF
```

This is **not yet a renderer-quality verdict**.

Only HeSuVi had been disabled. The rest of the incumbent routing remained configured. A strong current hypothesis is that an old ASIO/forwarding path remained physically audible alongside Omniphony, creating a duplicate delayed path.

There is also a known prototype bypass weakness: wet data can already be queued when OFF is requested.

Current evidence state:

```text
live arbitrary-audio transport = proven
single physical path = not yet proven
clean bypass = not yet proven
fair music-quality A/B = not yet proven
```

---

## 7. Current prototype route

Current temporary path:

```text
Windows / foobar
→ existing Hi-Fi Cable render endpoint
→ self-excluding WASAPI process-loopback capture
→ protected Omniphony renderer
→ automatically preferred FiiO output
→ headphones
```

This route exists because it is fast, reversible and good enough to prove live audio.

It is **development scaffolding**, not the final transparent product route.

The current app structure is worth keeping:

```text
Omniphony.exe
        ↓
platform worker/host
        ↓
portable Omniphony core
```

Future Windows routing can replace the loopback/cable layer without replacing the app/core ownership boundary.

---

## 8. Bypass acceptance law

OFF is a real transport feature, not a cosmetic UI state.

Required final behavior:

```text
OFF
→ no wet queue tail
→ no stale room tail selected from the wet path
→ no second physical forwarding path
→ no duplicate dry copy
→ no renderer leakage
```

The polished comparison path should be latency-aligned and switch near the physical-output boundary so previously queued blocks cannot leak the old selection.

Current prototype behavior does not fully satisfy this yet.

---

## 9. Windows host responsibilities

The Windows layer may own:

- application/session discovery;
- source-layout discovery;
- process/session capture/interception;
- endpoint creation/selection if required;
- output-device tracking;
- shared/exclusive mode decisions;
- clock/drift handling;
- sample-rate conversion at platform boundaries;
- sleep/wake and endpoint recovery;
- installer/signing integration;
- diagnostics;
- platform-specific latency controls.

It should translate those into portable contracts rather than leaking Windows concepts down into the renderer.

---

## 10. Candidate final Windows route classes

The exact final mechanism is intentionally not frozen.

Candidates include:

### Owned virtual render endpoint

```text
applications
→ Omniphony virtual endpoint
→ Windows host
→ portable core
→ physical stereo endpoint
```

Attractive because the product can advertise richer accepted layouts while the physical DAC remains 2.0.

Costs include driver/endpoint deployment, signing, buffering, clock and lifecycle complexity.

### Native system-effect / in-graph integration

Potentially elegant for ordinary shared-mode use, but only if it preserves the source truth and process isolation the product needs.

Do not choose it merely because it is “more native.”

### Session-aware host routing

Potential route for preserving source/session boundaries so a stereo music stream and a surround game can coexist as separate logical Omniphony inputs.

The exact Windows API path must be proven experimentally.

### Hybrid

A combination may ultimately provide the best user experience and rich-source preservation.

Decision criteria:

```text
single path
source-truth preservation
concurrent-layout correctness
latency
reliability
installability
recovery
clean disable/uninstall
user invisibility
```

---

## 11. ASIO relationship

ASIO remains useful as:

- incumbent/reference plumbing;
- specialist output route;
- development comparison;
- possible permanent advanced option.

It is not the universal consumer requirement.

Do not delete it merely because the normal Windows route improves.

Do not force ordinary users to install/configure it.

---

## 12. Current native pieces

### `Omniphony.exe`

Small native product-shell prototype.

Owns user-facing ON/OFF/status and supervises the hidden audio worker.

### `omniphony_worker.exe`

Hidden Windows audio worker.

Owns process-loopback capture, Omniphony engine execution and physical output for the current prototype.

### `omniphony_live.exe`

Diagnostic/development binary.

### `windows_host.exe`

Older smoke/reference host used to prove native output and the protected reference path.

### `realtime_ffi`

Narrow interleaved-f32 PCM seam useful for isolating host transport from renderer semantics.

### `reference_bridge`

Deterministic bridge into the protected Omniphony engine.

Canonical channel order:

```text
L R C LFE Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

---

## 13. Channel-order law

When a platform layout differs from Omniphony's canonical layout, adapt explicitly at the host boundary.

Example Windows 7.1 interleave:

```text
Windows:
L R C LFE Lb Rb Ls Rs

Omniphony bridge:
L R C LFE Ls Rs Lb Rb
```

Never let a successful compile hide a side/rear swap.

---

## 14. Decision gates

A Windows route graduates only if it proves:

1. one physical audible path;
2. clean ON/OFF with no old-path leakage;
3. ordinary stereo playback with no configuration ritual;
4. richer surround preserved when available;
5. stereo + surround coexistence without a global channel-mode switch;
6. same portable renderer semantics for the same logical input;
7. deterministic endpoint/device recovery;
8. suitable music/video/gaming latency;
9. bounded glitch/underrun behavior;
10. clean install/remove/update;
11. no dependency on the old Hi-Fi/ASIO chain;
12. incumbent coexistence during migration;
13. host/platform work cannot block or redefine realtime renderer semantics;
14. physical output can remain normal binaural stereo.

---

## 15. Current acceptance ladder

### T0 · Native output/reference proof — PASSED

Known protected content reaches the physical headphones.

### T1 · Native product shell — PASSED

`Omniphony.exe` launches and supervises the hidden worker.

### T2 · Arbitrary live Windows audio — PASSED

Real foobar/Windows audio reaches Omniphony and the FiiO.

### T3 · Single-path clean listening — CURRENT

Prove that no old ASIO/forwarding route reaches the FiiO simultaneously.

### T4 · Clean bypass — CURRENT

Remove queued wet-tail/leakage from ON/OFF comparison.

### T5 · Clean stereo baseline

```text
ordinary stereo
→ Omniphony only
→ FiiO
```

Judge tonal fidelity and sphere/externalization only here.

### T6 · Native surround baseline

Verify authored 5.1/7.1 survives correctly.

### T7 · Mixed-layout coexistence

```text
stereo music
+
native surround application
→ stable simultaneous playback
```

### T8 · Owned production Windows route

Replace cable/loopback scaffolding with the best native Windows route.

---

## 16. Immediate test procedure

Keep the incumbent installed.

For the next fair listen:

```text
1. keep Hi-Fi Cable installed/configured
2. disable HeSuVi
3. stop/bypass ASIO Bridge or any other old physical forwarding to FiiO
4. confirm Omniphony is the only active FiiO path
5. test stereo music first
6. verify OFF has no echo/tail
7. only then score tinny/hallway/bubble/externalization
8. later test surround alone
9. later test stereo + surround simultaneously
```

Remove old components only after Omniphony has replaced their function.

---

## 17. Frozen Windows laws

1. **Windows is a host, not the core.**
2. **Ordinary stereo music must work with a normal 2.0 physical headphone output.**
3. **Source layout belongs to each logical stream, not to a global Omniphony mode.**
4. **Stereo and surround applications must be able to coexist.**
5. **Native rich source truth should be preserved rather than reconstructed from stereo.**
6. **One physical audible path only.**
7. **OFF must be route-clean.**
8. **Keep the old HeSuVi/Hi-Fi stack installed during migration; disable before uninstalling.**
9. **Loopback/cable is scaffolding, not the final product route.**
10. **The final Windows mechanism is chosen by evidence, not architectural fashion.**
