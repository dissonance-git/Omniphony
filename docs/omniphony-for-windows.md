# Omniphony for Windows

## Product identity

This repository remains a downstream fork of `mgth/Omniphony`.

**Omniphony is the renderer/engine family. Omniphony for Windows is the Windows operating-system integration of that engine, not a second renderer and not a virtual-cable product.**

The Windows product should live as close as practical to the final Windows render boundary:

```text
applications / games / browsers / players
        ↓
Windows Audio Engine
        ↓
Omniphony endpoint effect (EFX APO)
        ↓
physical endpoint driver
        ↓
DAC / headphones
```

For the primary development system that means:

```text
Windows Audio Engine
        ↓
Omniphony EFX
        ↓
Dan Clark Noire X (FiiO Q series)
        ↓
FiiO K7 / headphones
```

No virtual cable, loopback capture, duplicated dry stream, or second user-visible playback endpoint belongs in the mature path.

## Why EFX is the correct conventional Windows boundary

Windows already defines an Audio Processing Object architecture for system audio effects. An endpoint effect (EFX) is applied to all streams that use one endpoint and is positioned after the render mix. That is the correct conventional system-wide product role for Omniphony for Windows: one post-mix DSP layer immediately before the physical endpoint.

The intended conventional topology is therefore:

```text
many Windows streams
        ↓
Windows mixing / normal app volume / normal device policy
        ↓
ONE Omniphony processing graph
        ↓
physical FiiO endpoint
```

This eliminates the architectural problem that caused the temporary bridge to become fragile: capture and playback no longer need to be separated by a borrowed virtual sink. Windows owns the mix; Omniphony owns the final endpoint processing; the FiiO owns physical playback.

The EFX path is not automatically the richest Windows spatial ingress. Raw Windows Spatial Audio static/dynamic objects may require a different supported host seam before the endpoint mix. That richer ingress must converge on the same portable Omniphony scene and renderer rather than creating a second audio engine.

## Canonical scene requirement: 8.1.4.4 base

Omniphony's portable Windows-facing scene contract uses **8.1.4.4 as its canonical static coordinate frame**.

```text
horizontal
FL FR C LFE SL SR BL BR BC

upper
TFL TFR TBL TBR

lower
BFL BFR BBL BBR
```

This is a semantic frame, not a claim that every source contains seventeen authored signals.

Every static anchor must distinguish at least:

```text
AUTHORED
DERIVED
EMPTY
```

The same scene may therefore represent stereo, 5.1, 7.1, 7.1.4, 7.1.4.4, or full 8.1.4.4 without changing renderer architecture. Dynamic objects with supplied x/y/z positions live in a parallel continuous object layer and must not be prematurely snapped to the static anchors.

```text
source representation
        ↓
8.1.4.4-capable static scene
+ continuous dynamic objects when available
        ↓
Omniphony rendering geometry
        ↓
headphone binaural stereo
```

This canonical scene is distinct from the Current-model 22-direction full-sphere support shell. The 17 anchors are standardized semantic positions; the 22-direction field is internal renderer/support geometry.

The source-authority law remains absolute:

> **The coordinate system may be rich even when the source knowledge is sparse. Missing authorship must remain missing authorship.**

See `docs/windows-spatial-input-contract.md` for the detailed mapping and provenance rules.

## Native multichannel requirement

A stereo physical headphone endpoint must **not** force Omniphony to receive only stereo.

Windows 11 version 23H2 introduced `IAudioProcessingObjectPreferredFormatSupport`. Its `GetPreferredInputFormat` callback exists specifically so an APO can request a richer input format than the endpoint's output format. Microsoft's documented headphone-virtualization example is a stereo-rendering endpoint whose APO requests **7.1 input**.

That becomes a hard Omniphony for Windows requirement on supported Windows 11 systems:

```text
game / application authors real 5.1 or 7.1 PCM
        ↓
Windows Audio Engine
        ↓
Omniphony APO requests / receives the authored multichannel bed
        ↓
FL / FR / C / LFE / SL / SR / BL / BR remain distinct
        ↓
map into canonical 8.1.4.4 scene as AUTHORED anchors
        ↓
remaining anchors EMPTY or bounded DERIVED
        ↓
Omniphony binaural render
        ↓
stereo
        ↓
FiiO DAC / headphones
```

Do **not** downmix an authored 5.1/7.1 bed to stereo before Omniphony and then try to infer the lost geometry.

Do **not** describe a 7.1 source as authored 8.1.4.4 merely because it now inhabits Omniphony's 8.1.4.4-capable scene.

The preferred-input contract should be mode-aware where useful. Media/game modes may request 5.1/7.1 input while communications or other modes can remain stereo when that is the more appropriate format.

The first implementation target remains conventional PCM beds. Dolby/DTS bitstream decoding is not required for ordinary PC games that already present decoded multichannel PCM to Windows. Rich Windows Spatial Audio/object metadata is a separate host problem because object identity may not survive to the post-mix endpoint stage.

## Native Windows Spatial Audio and Dolby target

Omniphony should behave as its own Windows spatial renderer **without requiring the user-visible Windows Spatial Sound provider slot**, while interoperating natively with Windows/Dolby where supported APIs allow it.

Microsoft Spatial Sound uses `ISpatialAudioClient` for static spatial objects and dynamic x/y/z objects. Microsoft's current Windows resource tables expose Dolby Atmos for Headphones with **17 maximum static objects / 8.1.4.4**, the same static vocabulary used by Windows Sonic for Headphones and DTS Headphone:X. Dolby's own Windows implementation guidance directs Atmos applications to Microsoft's Spatial Audio APIs and `ISpatialAudioClient`.

That makes the intended rich path:

```text
Windows spatial application / game
        ↓
8.1.4.4 static objects + dynamic objects
        ↓
SUPPORTED RICH WINDOWS HOST SEAM
        ↓
Omniphony canonical scene
        ↓
Omniphony renderer
        ↓
stereo physical endpoint
```

If that supported scene seam can be reached without registering Omniphony as a consumer-selectable spatial format/provider, use it. If Windows does not expose such a public seam, record the limitation rather than hiding it behind reconstruction or hooking.

### Dolby-native behavior

"Works with Dolby natively" has two valid meanings and they must not be conflated.

**Native scene ingestion** is preferred when the platform exposes real bed/object metadata to Omniphony before final binaural rendering:

```text
Dolby/Windows spatial source truth
        ↓
8.1.4.4 + dynamic objects
        ↓
Omniphony scene
        ↓
Omniphony binaural renderer
```

**Native Dolby renderer coexistence** applies when Dolby Atmos for Headphones has already rendered the scene to binaural stereo before Omniphony sees it:

```text
Dolby Atmos for Headphones
        ↓
already-binaural stereo
        ↓
Omniphony spatial bypass
or explicitly validated non-spatial correction only
        ↓
headphones
```

Do not run Omniphony's HRTF/room/spatialization over an already-binaural Dolby headphone render. Stereo channel count alone is not enough to identify this case; a reliable Windows host signal or explicit mode state is required before automatic switching is trusted.

For encoded Dolby media, prefer native Windows facilities. Microsoft documents Media Foundation/Spatial Sound support for Dolby Atmos playback without requiring every application to implement its own decoder. Omniphony should not make proprietary Dolby codec reverse engineering a prerequisite for normal compatibility. If Windows can expose the decoded spatial scene through a supported seam, preserve it; if it exposes only the final Dolby binaural result, use the coexistence/bypass rule above.

This keeps Dolby support inside the same architecture:

> **one scene, one Omniphony renderer when Omniphony owns rendering, and no double renderer when Dolby already owns it.**

## Current spatial-ingress boundary

The post-mix EFX and the Windows Spatial Audio object path are documented as different parts of the Windows audio architecture.

The public Microsoft/Dolby documentation reviewed so far does not prove that an arbitrary third-party endpoint EFX can receive another application's raw `ISpatialAudioClient` object identities and x/y/z metadata before the active spatial renderer consumes them.

Therefore the product contract must distinguish:

```text
PROVEN / IMPLEMENTATION TARGET
system-wide EFX on the physical endpoint
2.0 / 5.1 / 7.1 conventional PCM preservation
canonical 8.1.4.4 scene mapping inside Omniphony
own binaural renderer

SUPPORTED BY WINDOWS SPATIAL MODEL, RICH INGRESS STILL TO PROVE
7.1.4 / 7.1.4.4 / 8.1.4.4 static spatial objects
dynamic x/y/z objects
raw scene ingestion from arbitrary spatial-aware applications

SAFE COEXISTENCE TARGET
already-binaural Dolby/Windows spatial output
→ Omniphony spatial bypass
```

Do not hook or inject into games to obtain object metadata, especially anti-cheat-protected games. Do not weaken Windows security/integrity boundaries. Do not revive the virtual-sink architecture merely because it is easier to intercept a scene there.

## Windows APO constraint

The correct architecture does not remove Windows deployment requirements.

Custom APOs are user-mode COM DSP components loaded by the Windows audio engine, but modern Windows associates them with a specific audio device through the componentized audio-driver model. Windows does not support registering one global custom APO and automatically attaching it to arbitrary third-party audio drivers.

For the current development target, Omniphony for Windows should therefore build and validate an endpoint-effect package associated with the FiiO render device. A future general-public product needs a supported device-association and signing strategy rather than registry hacks or a hidden virtual cable.

Do not weaken Secure Boot, protected audio, driver signing, or other Windows integrity protections to make development easier.

## Retired bootstrap transport

The Steam Streaming Speakers / process-loopback / endpoint-loopback work was a temporary bootstrap used to exercise installer, routing, watchdog, and installed-runtime behavior while the real Windows boundary was unresolved.

It is now retired as a product direction.

Historical lessons remain useful:

```text
KEEP
single EXE installation goal
headless lifecycle
writable per-user runtime logs
explicit physical endpoint identity
watchdog / restart behavior
transactional rollback
safe uninstall
profile separation

RETIRE
Steam Streaming Speakers as transport
virtual-cable-style default-device routing
process loopback as normal ingestion
endpoint loopback as normal ingestion
borrowed virtual endpoint as product foundation
```

Do not spend new stabilization effort polishing the retired transport unless needed only as a bounded diagnostic control.

## Portable core vs Windows adapter

The portable renderer remains independent of Windows.

```text
PORTABLE CORE
Omniphony rendering laws
source authority
canonical 8.1.4.4-capable scene contract
AUTHORED / DERIVED / EMPTY provenance
continuous object positions
PCM / channel-layout contracts
stereo inference
multichannel / object preservation
binaural rendering
HRTF / ITD / geometry
room / reflection machinery
profile parameters
validation / measurement

WINDOWS ADAPTER
EFX APO host
Windows audio format negotiation
preferred multichannel input negotiation
supported Spatial Audio scene ingress when available
native-Dolby/Windows spatial coexistence state
endpoint/device association
profile/config loading
realtime-safe call into the portable core
installer / servicing package
lifecycle / diagnostics
update / uninstall
```

The APO and any future Spatial Audio ingress are host seams, not places to fork the renderer.

The realtime portable ABI should evolve until the APO, a supported Spatial Audio host seam, `foo_omniphony`, future macOS/Linux hosts, and test harnesses can all call the same scene/processing contracts without duplicating Omniphony DSP.

## Realtime law for the APO

The endpoint effect runs in the Windows audio engine's realtime path. It must therefore remain a thin, deterministic host around preallocated portable DSP.

The realtime processing callback must not:

- block;
- allocate unpredictably;
- perform filesystem or network I/O;
- launch subprocesses;
- acquire unbounded locks;
- perform device discovery;
- perform research/analysis work that is not already converted into bounded realtime state.

Configuration, HRTF/profile preparation, device discovery, updates, and heavy initialization belong outside the realtime callback.

## Source authority remains unchanged

Moving Omniphony into an EFX or mapping all sources into an 8.1.4.4-capable scene does not change the fidelity laws.

```text
stereo
→ preserve the finished stereo master + bounded inferred support
→ canonical frame may contain DERIVED/EMPTY anchors, never false authorship

5.1 / 7.1 PCM
→ request and preserve authored directional channels before binaural reduction
→ map them to matching AUTHORED anchors in the canonical frame

7.1.4 / 7.1.4.4 / 8.1.4.4
→ preserve supplied authored upper/lower/back-center geometry when the host exposes it

object audio
→ preserve supplied continuous positions when available at a suitable host boundary

Ambisonics / HOA
→ preserve the field representation rather than collapsing it prematurely

already-binaural
→ avoid destructive double virtualization
```

The physical DAC remains stereo. Omniphony converts the source representation available at its host boundary into final headphone stereo.

For Windows 11 23H2+, conventional 5.1/7.1 beds should use preferred APO input-format negotiation so the physical stereo endpoint does not erase the authored bed before Omniphony. Rich object metadata that Windows has already flattened before the endpoint effect may require an additional richer host integration. Do not pretend an endpoint effect can recover source metadata that the platform no longer supplies at that stage.

## Profile boundary

The publishable product and a listener's personal tuning remain separate objects.

```text
Omniphony for Windows
        │
        ├── public/default profile
        │     hardware-agnostic where possible
        │     listener-agnostic
        │     conservative documented defaults
        │
        └── user profile
              headphone/device correction
              listener-specific balance
              HRTF personalization
              hearing-asymmetry compensation
              geometry preferences
              comfort / listening-level choices
```

The current primary listening profile is a strong customization case and engineering testbed. It is not automatically the public default.

The Windows EFX association may be device-specific while the audible profile remains a separate layer. Do not confuse "this APO is attached to the FiiO endpoint" with "the renderer's public tuning is FiiO-specific."

## Installer contract

The user-facing artifact remains one installer executable.

Target experience:

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
install / update Omniphony APO package
associate it with the selected physical render endpoint
install profile/config/control support
restart/rebuild the endpoint graph when required
        ↓
done
```

Normal use should leave Windows' ordinary physical output selected. The user should not select an Omniphony virtual device because the mature design has no Omniphony virtual playback endpoint.

The installer should remember the intended physical endpoint by stable device identity, verify that the APO is actually active in the endpoint graph, and roll back cleanly if installation fails.

Uninstall should remove only Omniphony-owned components and restore the endpoint to its pre-Omniphony processing state.

## Headless and reliability law

Omniphony for Windows should be nearly invisible in normal use.

Required behavior:

```text
Windows output remains the real physical device
Omniphony processing is automatically present in that endpoint graph
no console
no virtual-device selection
no loopback routing
no duplicate dry stream
no daily device configuration
no user-managed helper program
```

Diagnostics must be available, but diagnostics are not the product workflow.

A useful health check should be able to prove independently that:

```text
APO registered
→ APO associated with expected endpoint
→ Windows loaded APO
→ negotiated conventional input is expected stereo / 5.1 / 7.1
→ canonical scene mapping retains correct AUTHORED / DERIVED / EMPTY state
→ realtime callbacks are occurring
→ non-silent input reached the APO
→ non-silent output left the APO
```

A future rich-Spatial-Audio health check must additionally prove the actual supported host seam, active static-object mask, dynamic-object capacity, and whether Omniphony or another renderer owns final binauralization.

Installation success is not listening success. Physical listening remains the final audible gate.

## Studio and control relationship

Omniphony Studio remains the advanced visualization/control frontend. Omniphony for Windows should eventually expose or forward compatible engine controls rather than inventing a second incompatible control model.

The realtime APO must not depend on Studio being open.

## Future operating-system hosts

The Windows EFX is one operating-system adapter around the portable renderer.

```text
portable Omniphony core
        │
        ├── Windows endpoint-effect host
        ├── supported Windows Spatial Audio scene host if available
        ├── future macOS host
        ├── future Linux host
        ├── foo_omniphony
        └── deterministic test/file hosts
```

Other platforms may have a different native insertion point. They should reuse the same portable scene and processing contracts rather than copying Windows APO concepts into the core.

## Naming

Use **Omniphony for Windows** for the Windows product.

Preferred user-facing names:

```text
installer     OmniphonySetup.exe
application   Omniphony
endpoint      the user's real physical endpoint
Windows DSP   Omniphony endpoint effect
product       Omniphony for Windows
```

Historical `Spatial` labels and the Steam transport are bootstrap provenance, not mature product identity.

Do not create a second branded subsystem merely for the canonical scene. It is the portable renderer's input/scene contract, not a new product.

## Upstream and fork discipline

Keep the relationship to `mgth/Omniphony` explicit.

Prefer upstream renderer machinery when it already owns the job. Keep Windows association, APO lifecycle, installation, and endpoint policy in the downstream Windows host. Do not copy Studio's control model into a second incompatible renderer. Preserve applicable GPL-3.0-or-later obligations when distributing derivative work.

## Immediate frontier

The stabilization frontier remains implementation-first while richer Spatial Audio ingress is investigated in parallel:

```text
P0
compile a minimal Omniphony EFX APO
associate it with the FiiO render endpoint on Windows 11
prove transparent identity/bypass first
prove one processed stereo stream reaches the FiiO
prove clean install/update/uninstall

P1
implement preferred-input format negotiation on Windows 11 23H2+
prove authored 5.1 / 7.1 beds reach Omniphony without pre-downmix
map those beds into the canonical 8.1.4.4 scene with correct provenance
connect the protected portable renderer behind the APO
realtime-safety + block-size + latency regression gates
physical listening

P2
profile/config servicing without disturbing the realtime graph
sleep/resume, DAC reconnect, audio-engine restart, upgrade recovery

P3
harden conventional 5.1 / 7.1 game/media behavior across processing modes and format changes
prove bounded inferred height/lower support never overwrites authored source truth

P4
integrate richer Windows static/dynamic scene ingress only if a supported host seam is demonstrated
validate 7.1.4 / 7.1.4.4 / 8.1.4.4 + dynamic-object preservation end to end
validate native Dolby scene ingestion or clean already-binaural coexistence as appropriate

PARALLEL SPATIAL-INGRESS RESEARCH
find the earliest supported Windows boundary that can expose raw ISpatialAudioClient static/dynamic scene data to Omniphony
prefer a solution that keeps the real physical endpoint and avoids the consumer Spatial Sound provider slot
verify Dolby Atmos for Headphones interoperability against the same 8.1.4.4 + object scene contract
record a clean negative boundary if public APIs do not expose such a seam

PARALLEL PRODUCT WORK
public/default profile separation
personalized profile maturation
Studio-control compatibility
```

Do not create a second renderer to achieve these milestones. Do not revive the virtual-sink architecture merely because it is easier to prototype. Do not claim raw Dolby/Windows object ingestion until an end-to-end supported path has actually been demonstrated.

## Primary Windows/Dolby references

- Microsoft Spatial Sound overview and current format/object limits: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft Spatial Audio object rendering/channel masks: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- Microsoft APO architecture: https://learn.microsoft.com/windows-hardware/drivers/audio/audio-processing-object-architecture
- Microsoft preferred APO input format documentation: https://learn.microsoft.com/windows/win32/api/audioengineextensionapo/nf-audioengineextensionapo-iaudioprocessingobjectpreferredformatsupport-getpreferredinputformat
- Dolby Windows implementation guidance: https://professionalsupport.dolby.com/s/article/Windows-Implementation
