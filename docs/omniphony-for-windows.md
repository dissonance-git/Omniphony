# Omniphony for Windows

## Product identity

This repository remains a downstream fork of `mgth/Omniphony`.

**Omniphony is the renderer/engine family. Omniphony for Windows is an operating-system host around that engine, not a second renderer.**

The long-term shape is:

```text
Omniphony portable engine / rendering contracts
        │
        ├── Omniphony Studio
        │     visualization / supervision / advanced control
        │
        ├── Omniphony for Windows
        │     installation
        │     virtual render endpoint
        │     system-wide routing
        │     physical-device selection
        │     lifecycle / watchdog / recovery
        │     autostart / updates / uninstall
        │     lightweight tray controls
        │     optional access to the same advanced controls Studio exposes
        │
        ├── future operating-system hosts
        │     replace the host shell without replacing the engine
        │
        └── application-specific hosts such as foo_omniphony
```

The Windows host may be replaced later. Windows concepts must not leak into the portable renderer merely because Windows is the first productized host.

## Why this boundary exists

Upstream already separates the real-time renderer from Studio and from platform output backends. This fork should preserve that separation while adding a Windows integration layer suitable for ordinary daily use.

The Windows wrapper should make Omniphony behave like installed system audio software:

```text
install once
→ choose/remember physical headphone output
→ create Omniphony system endpoint
→ make it the ordinary Windows output
→ render headlessly in the background
→ recover automatically
→ uninstall cleanly
```

Normal use must not require ASIO Bridge, HeSuVi, Hi-Fi Cable, manual device switching, command files, PowerShell, a terminal, or Studio.

Studio remains valuable as an advanced control and visualization surface. Omniphony for Windows should eventually expose or forward the same useful engine controls without duplicating the renderer or inventing a parallel control model.

## Portable core vs host adapters

The architectural boundary is:

```text
PORTABLE
Omniphony renderer
source authority
PCM + source-layout contracts
binaural / speaker rendering
HRTF / ITD / geometry
room / reflection machinery
profile parameters that describe rendering
validation / measurement

HOST-SPECIFIC
Windows virtual endpoint
WASAPI / Windows audio-session integration
installer / driver package
physical endpoint discovery
set-default-device behavior
tray / service / watchdog
startup / update / uninstall
```

A future macOS or Linux product should be able to replace the host-specific half while reusing the same renderer and profile semantics.

## Source authority remains unchanged

More source truth means less inference:

```text
stereo
→ preserve the finished stereo master + bounded inferred support

5.1 / 7.1
→ preserve authored directional channels

5.1.2 / 7.1.4
→ preserve authored height

object audio
→ preserve supplied object positions

Ambisonics / HOA
→ preserve the supplied field representation

already-binaural
→ avoid destructive double virtualization
```

The physical headphone DAC remains stereo. The Windows-facing Omniphony endpoint is the place that should eventually advertise the richer source-side capabilities.

## Profile boundary

The publishable product and a listener's personal tuning are different objects.

```text
Omniphony for Windows
        │
        ├── public/default profile
        │     hardware-agnostic
        │     listener-agnostic
        │     no private hearing compensation
        │     no device-specific personal EQ
        │     conservative, documented defaults
        │
        └── user profiles
              headphone/device correction
              listener-specific balance
              HRTF selection/personalization
              geometry preferences
              comfort/listening-level choices
              source-aware personalization
```

The current primary listening profile is the first strong customization case and an engineering testbed. It is **not** the definition of the public default.

Personal profile evidence may justify a general mechanism only after the mechanism is separated from the personal parameter values and survives the project's normal research, validation, and listening gates.

Private or sensitive listener-specific values do not need to live in the public repository merely to prove that the profile mechanism exists.

## Current listening evidence vs public defaults

Existing listening work in this fork remains valuable, but it must be interpreted in the correct layer:

- source-preservation and fidelity laws are product/core laws;
- portable rendering mechanisms may become product mechanisms when validated;
- listener-approved geometry, level, comfort, headphone correction, or asymmetric compensation belong to a profile unless separately generalized;
- experimental candidates remain experimental until adjudicated;
- a successful personal profile is evidence that the system is customizable, not evidence that every listener should receive the same settings.

## Windows 11 first target

The first product target is Windows 11 x64.

The target daily path is:

```text
Windows / games / music players / browsers
        ↓
Omniphony virtual render endpoint
        ↓
Windows host adapter
        ↓
portable Omniphony renderer
        ↓
selected user profile
        ↓
binaural stereo
        ↓
physical headphone endpoint
```

The initial endpoint may begin as stereo-only to establish a single stable Windows-wide stream. The same endpoint architecture should then grow to accept conventional 5.1/7.1 PCM and richer spatial/object input without replacing the core or creating a second audio path.

48 kHz float is the normal Windows rendering target unless source/host evidence justifies another rate. Higher nominal sample rates are not themselves a spatial-quality feature.

## Installer contract

The product-facing artifact is a **single installer executable**.

Normal installation target:

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
install/upgrade the Windows endpoint
install the Omniphony host
remember the physical output
configure routing
make Omniphony the Windows default output
configure autostart
start the host
        ↓
done
```

Normal users should not see internal driver files, certificates, command scripts, DevCon/DevGen, WDK artifacts, or manual endpoint plumbing.

Uninstall must reverse the owned machine state cleanly and must not remove unrelated audio devices or user configuration.

### Development-signing boundary

During private development, the custom kernel audio endpoint may be WDK development/test signed rather than Microsoft production signed. Windows 11 kernel-signing policy is an external security boundary and must not be bypassed or silently weakened by the installer.

The development installer may automate every legitimate step around that boundary, detect when the driver cannot load under the current boot policy, and report that condition clearly. It must not silently disable Secure Boot, BitLocker, driver-signature enforcement, or boot integrity protections.

A publishable installer requires an appropriately Microsoft-signed driver package so installation becomes the ordinary one-UAC experience described above.

## Headless and reliability law

Omniphony for Windows should be nearly invisible in normal use.

Required behavior:

- one installed app/host, not a pile of helper programs;
- no console windows;
- no manual routing after installation;
- automatic recovery from renderer/output failure;
- hard OFF means the audio engine is actually stopped and releases its owned path;
- crash/exit cannot intentionally leave a ghost child renderer running;
- the physical output can never resolve to the Omniphony virtual endpoint itself;
- installer/updater operations are idempotent and reversible;
- logs/telemetry exist for diagnosis but do not become a user workflow;
- sound-changing work remains separate from packaging/host mechanics.

## Naming

Use **Omniphony for Windows** for the Windows product/host.

Prefer these user-facing names unless a later upstream-aligned convention supersedes them:

```text
installer     OmniphonySetup.exe
application   Omniphony
endpoint      Omniphony
product       Omniphony for Windows
```

Historical `Spatial` labels are temporary private bootstrap names and should disappear as the Windows shell is consolidated. Internal renderer names may retain upstream Omniphony terminology.

## Upstream and fork discipline

Keep the relationship to `mgth/Omniphony` explicit.

- prefer upstream renderer machinery when it already owns the job;
- keep Windows integration in the downstream host layer;
- keep local extensions bounded and attributable;
- do not copy Studio's rendering-independent control model into a new incompatible model;
- periodically compare upstream changes before deepening a fork-specific seam;
- preserve the GPL-3.0-or-later obligations of the upstream project when distributing derivative work.

## Immediate frontier

The next sequence is intentionally narrow:

```text
P0
single Windows-wide stream
single EXE development installer
headless lifecycle
stable physical-output routing

P1
5.1 / 7.1 source-side endpoint formats
explicit channel/layout semantics through the host ABI

P2
richer Windows spatial/object input where the platform exposes trustworthy metadata

P3
Studio-control parity / advanced optional control surface

parallel
profile system maturation
personalized listening profile as a strong test case
public/default profile kept independent
```

Do not create a second renderer to achieve any of these milestones.