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

## Why EFX is the correct Windows boundary

Windows already defines an Audio Processing Object architecture for system audio effects. An endpoint effect (EFX) is applied to all streams that use one endpoint and is positioned after the render mix. That is the exact product role Omniphony for Windows needs: one post-mix DSP layer immediately before the physical endpoint.

The intended topology is therefore:

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
FL / FR / C / LFE / SL / SR / RL / RR remain distinct
        ↓
Omniphony binaural render
        ↓
stereo
        ↓
FiiO DAC / headphones
```

Do **not** downmix an authored 5.1/7.1 bed to stereo before Omniphony and then try to infer the lost geometry.

The preferred-input contract should be mode-aware where useful. Media/game modes may request 5.1/7.1 input while communications or other modes can remain stereo when that is the more appropriate format.

The first implementation target is conventional PCM beds. Dolby/DTS bitstream decoding is not required for ordinary PC games that already present decoded multichannel PCM to Windows. Rich Windows Spatial Audio / object metadata is a separate host problem because object identity may not survive to the post-mix endpoint stage.

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
endpoint/device association
profile/config loading
realtime-safe call into the portable core
installer / servicing package
lifecycle / diagnostics
update / uninstall
```

The APO is a host seam, not a place to fork the renderer.

The realtime portable ABI should evolve until the APO, `foo_omniphony`, future macOS/Linux hosts, and test harnesses can all call the same processing contracts without duplicating Omniphony DSP.

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

Moving Omniphony into an EFX does not change the fidelity laws.

```text
stereo
→ preserve the finished stereo master + bounded inferred support

5.1 / 7.1 PCM
→ request and preserve authored directional channels before binaural reduction

height / richer spatial input
→ preserve authored geometry where the Windows path exposes trustworthy metadata

object audio
→ preserve supplied positions when available at a suitable host boundary

already-binaural
→ avoid destructive double virtualization
```

The physical DAC remains stereo. Omniphony converts the source representation available at its host boundary into final headphone stereo.

For Windows 11 23H2+, conventional 5.1/7.1 beds should use preferred APO input-format negotiation so the physical stereo endpoint does not erase the authored bed before Omniphony. Rich object metadata that Windows has already flattened before the endpoint effect may require an additional richer host integration later. Do not pretend an endpoint effect can recover source metadata that the platform no longer supplies at that stage.

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
→ negotiated input format is the expected stereo / 5.1 / 7.1 layout
→ realtime callbacks are occurring
→ non-silent input reached the APO
→ non-silent output left the APO
```

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
        ├── future macOS host
        ├── future Linux host
        ├── foo_omniphony
        └── deterministic test/file hosts
```

Other platforms may have a different native insertion point. They should reuse the same portable processing contracts rather than copying Windows APO concepts into the core.

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

## Upstream and fork discipline

Keep the relationship to `mgth/Omniphony` explicit.

Prefer upstream renderer machinery when it already owns the job. Keep Windows association, APO lifecycle, installation, and endpoint policy in the downstream Windows host. Do not copy Studio's control model into a second incompatible renderer. Preserve applicable GPL-3.0-or-later obligations when distributing derivative work.

## Immediate frontier

The stabilization frontier is now deliberately smaller:

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
connect the protected portable renderer behind the APO
realtime-safety + block-size + latency regression gates
physical listening

P2
profile/config servicing without disturbing the realtime graph
sleep/resume, DAC reconnect, audio-engine restart, upgrade recovery

P3
harden conventional 5.1 / 7.1 game/media behavior across processing modes and format changes

P4
investigate richer Windows spatial/object host paths for metadata that cannot survive to a post-mix endpoint effect

parallel
public/default profile separation
personalized profile maturation
Studio-control compatibility
```

Do not create a second renderer to achieve these milestones, and do not revive the virtual-sink architecture merely because it is easier to prototype.