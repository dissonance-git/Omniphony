# Omniphony Windows Spatial Sound provider probe

This is a bounded Windows experiment for the platform seam Omniphony ultimately needs to occupy as a system spatial renderer.

The current questions are deliberately separated:

1. Can an independently registered Omniphony format be enumerated by the Windows **Spatial sound** selector?
2. If Windows activates the registered COM class, will it accept a standards-shaped `ISpatialAudioClient` capability object?
3. Can a real static Spatial Audio object cross the COM stream, the existing Omniphony realtime worker, and the final Windows output boundary without another headphone renderer touching it?

This probe is **not yet an audible Windows spatial renderer** and it does not change Omniphony's production signal path.

## Current capability stage

The COM DLL implements `ISpatialAudioClient` rather than only `IUnknown`.

The repository also contains an internal static-only `ISpatialAudioObjectRenderStream` lifecycle implementation. That stream accepts the `VT_BLOB` activation shape documented for `ISpatialAudioClient::ActivateSpatialAudioStream`, including exact structure-size, interface-ID, object-format, static-mask, and zero-dynamic-capacity validation.

The static-object renderer path behind that COM surface is now substantially built:

```text
Windows static role + authored position
        ↓
fixed-topology static-object ABI
        ↓
preallocated planar object ring
        ↓
dedicated WindowsStaticObjectPipeline worker
        ↓
existing source-aware Omniphony Current renderer
        ↓
binaural stereo
```

`omniphony_realtime.dll` already exposes this fixed-topology static-object ABI. A C++ bridge in this directory loads that ABI only from an explicit absolute DLL path, verifies ABI compatibility before processor creation, owns the processor/module lifetime in the safe order, and provides a registry-free diagnostic seam.

The public provider deliberately does **not** expose the static stream yet. Two boundaries are still missing:

1. the COM stream must hand each completed update quantum to the existing realtime bridge without changing its immutable static topology;
2. the returned binaural stereo must have a proven Windows output/cadence path to the real headphone endpoint.

Until both exist, accepting a public Spatial Audio stream could create a silent sink. The gate therefore remains closed.

Current state:

```text
static object vocabulary         17 roles / 8.1.4.4
object format                    mono float32 / 48 kHz
max frame count                  480 frames
max dynamic objects              0
internal static COM stream       implemented
VT_BLOB activation marshalling   implemented
static object -> Current worker  implemented
C++ realtime ABI bridge          implemented
immutable provider staging       implemented as inert primitive
COM quantum -> bridge wiring     pending
final Windows output/cadence     pending
provider render stream available no
provider render stream activation no
```

`GetMaxDynamicObjectCount` intentionally returns `0`. Dynamic capacity will increase only when a real dynamic-object path exists.

`IsSpatialAudioStreamAvailable` and `ActivateSpatialAudioStream` intentionally return `SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE`. The internal machinery behind that gate is preparation, not permission to make an end-to-end claim early.

## Safety boundary

The probe remains smaller than the product path:

- it writes only two project-owned registry subtrees when the explicit registration experiment is invoked;
- it does not write `MMDevices` state;
- it does not change the default playback endpoint;
- it does not install a virtual audio device;
- it does not hook or inject into applications;
- it does not replace Windows system files or HRTFs;
- the public provider cannot activate a spatial render stream yet;
- `unregister` deletes only the two Omniphony probe keys;
- the immutable package-staging script performs **no registry writes and no provider selection**.

Stable experimental identities:

```text
Spatial format GUID  {4BD75423-A66C-4586-B782-1FCBBDF2AE74}
COM provider CLSID   {F3CDF827-20C4-405E-A430-8F739343FC89}
```

Candidate registration surface under test:

```text
HKLM\SOFTWARE\Microsoft\Multimedia\Audio\Spatial\Encoder\{format-guid}
HKLM\SOFTWARE\Classes\CLSID\{provider-clsid}\InProcServer32
```

The first path is an experimentally inferred Windows registration seam, not a documented public third-party provider contract.

The same `Spatial\Encoder` surface has also been independently explored by the open-source MSSOAL project. That is useful implementation evidence, not a substitute for a real-machine Omniphony result. MSSOAL itself describes its current provider work as a proof of concept rather than a working product, so its output architecture remains a quarry rather than proof of the Windows boundary.

## Registry-free static-stream contract

`OmniphonySpatialStaticStreamSmoke` exercises the internal COM-shaped stream without registry writes or Windows provider selection.

It covers:

- the documented `VT_BLOB` activation payload shape;
- exact activation-structure size;
- supported `ISpatialAudioObjectRenderStream` IID;
- zero dynamic-object capacity;
- canonical static-mask validation;
- static object activation and duplicate-role rejection;
- unavailable-role rejection;
- update ordering;
- 480-frame float buffers;
- static-position immutability;
- volume validation;
- implicit end-of-stream behavior;
- static-role reactivation;
- start, stop, and reset lifecycle.

This is a source-level smoke target. It does not by itself prove Windows provider enumeration, application object delivery, or audible endpoint output.

## Registry-free realtime bridge

`OmniphonySpatialRealtimeBridge` is the narrow C++ boundary between the provider experiment and the existing Rust realtime ABI.

It deliberately does **not** search `PATH`, use the process working directory, or discover a renderer during an audio update. `Open` requires an absolute `omniphony_realtime.dll` path and resolves the static-object ABI before processing begins.

The bridge verifies:

- realtime ABI major equality and compatible minor level;
- all required static-object exports;
- fixed sample rate / quantum / role descriptors through the Rust creation contract;
- processor destruction before `FreeLibrary`;
- no DLL discovery on the realtime processing call.

`OmniphonySpatialRealtimeBridgeSmoke` is designed to exercise:

```text
FL + TFL planar object PCM
→ dynamically loaded omniphony_realtime.dll
→ fixed static-object worker
→ existing Current source renderer
→ finite nonzero binaural stereo
```

Example once the binaries have been built:

```powershell
.\OmniphonySpatialRealtimeBridgeSmoke.exe C:\absolute\path\omniphony_realtime.dll
```

The smoke reports latency and processed-block observability. It remains registry-free and does not select Omniphony in Windows.

## Immutable provider generations

The future installer should never overwrite a COM DLL that Windows or an application may still have loaded. `Stage-OmniphonySpatialProvider.ps1` therefore prepares provider packages as immutable, content-addressed generations before any registration transaction exists.

Required package members are currently:

```text
OmniphonySpatialProbe.dll
omniphony_realtime.dll
OmniphonySpatialProbeCtl.exe
OmniphonySpatialProbeSmoke.exe
OmniphonySpatialStaticStreamSmoke.exe
OmniphonySpatialRealtimeBridgeSmoke.exe
CaptureSpatialProviderState.ps1
```

The staging script:

- hashes every package member with SHA-256;
- derives a generation identity from the complete sorted package hash set;
- copies a new candidate into a temporary generation directory;
- verifies every copied file before promotion;
- is designed to run capability, static-stream, and realtime-bridge smokes from the temporary candidate;
- moves the candidate into its immutable final generation path;
- re-runs path-sensitive provider and realtime-bridge smokes from that final path;
- writes an atomic `staged-generation.json` manifest containing the exact package and per-file hashes;
- explicitly records `registry_mutated=false` and `provider_selected=false`.

Example once the staged binaries exist:

```powershell
.\Stage-OmniphonySpatialProvider.ps1 `
  -PackageRoot C:\path\to\spatial-provider-payload `
  -AppRoot 'C:\Program Files\Omniphony'
```

An already-existing generation is verified rather than modified. This is intended to make eventual install, repair, upgrade, and rollback safer:

```text
new generation
→ stage beside current generation
→ verify in place
→ later switch only Omniphony-owned registration
→ keep previous generation available for rollback / loaded COM users
```

The staging script is **not yet wired into `OmniphonySetup.exe`**. That is intentional while public Spatial Audio stream activation remains closed.

## Run the enumeration experiment

The existing provider-registration experiment remains useful because the Windows seam itself is still unproven.

Extract these files into the same directory and leave them there while the probe is registered:

```text
OmniphonySpatialProbeCtl.exe
OmniphonySpatialProbe.dll
CaptureSpatialProviderState.ps1
```

First capture the current state from a normal terminal:

```powershell
.\OmniphonySpatialProbeCtl.exe contract
.\OmniphonySpatialProbeCtl.exe list
.\OmniphonySpatialProbeCtl.exe status
.\CaptureSpatialProviderState.ps1 > before-registration.txt
```

`status` returns exit code 3 before registration by design.

Then open **PowerShell as Administrator** in that directory and run:

```powershell
.\OmniphonySpatialProbeCtl.exe register .\OmniphonySpatialProbe.dll
.\OmniphonySpatialProbeCtl.exe diagnose
```

The registration command verifies that the COM DLL can be activated through the newly written CLSID before leaving the registry state in place.

Return to a normal, non-elevated terminal and capture the registered state:

```powershell
.\CaptureSpatialProviderState.ps1 > registered.txt
```

Now close and reopen Windows Settings and inspect the current physical output under:

```text
Settings
→ System
→ Sound
→ output device
→ Spatial sound
```

### Result A — `Omniphony` does not appear

This falsifies the current `Spatial\Encoder` enumeration hypothesis on that Windows build.

Preserve `before-registration.txt`, `registered.txt`, and the `list`, `status`, and `diagnose` output. The next step is read-only Process Monitor / registry-delta observation around a known provider rather than progressively broader registry writes.

### Result B — `Omniphony` appears but cannot be selected

This proves **provider enumeration** and advances the experiment to COM/provider capability negotiation.

That is meaningful evidence. The provider intentionally still reports render-stream activation unavailable.

The source frontier is no longer “invent a static renderer.” The static COM lifecycle, static-object Current worker, and C++ ABI bridge exist. The next provider work is to join the COM update quantum to that bridge and identify the valid final Windows output/cadence path.

### Result C — Windows accepts/selects the capability-only provider

Record this separately. It would show that provider selection can precede public stream creation, but it would still not prove any spatial application can render through Omniphony.

Do not leave the probe selected for normal listening because this build cannot publicly activate a render stream.

## Deterministic provider-selection snapshots

`CaptureSpatialProviderState.ps1` closes the P1/P2 observation gap without introducing another audio path. It is read-only and records:

- Windows product/version/build context;
- the current `Spatial\Encoder` provider inventory;
- the exact 64-bit registry values beneath the bounded `Spatial\Encoder` tree;
- the bounded per-device state beneath `SpatialAudioEndpoint`;
- value type, byte count, truncation state, and normalized value data;
- explicit markers that the collector performs no `MMDevices` writes.

The registry walk is sorted and bounded by default to depth 8 and 4096 bytes per value so two captures can be compared as ordinary line-oriented evidence instead of screenshots or memory.

For P2, capture each state **before** changing it again:

```powershell
# Windows Sonic selected in the normal Windows UI
.\CaptureSpatialProviderState.ps1 > sonic.txt

# Dolby Atmos for Headphones selected in the normal Windows UI
.\CaptureSpatialProviderState.ps1 > dolby.txt

# DTS Headphone:X selected in the normal Windows UI
.\CaptureSpatialProviderState.ps1 > dts.txt

# Omniphony selected, only if Windows actually allows it
.\CaptureSpatialProviderState.ps1 > omniphony.txt
```

Then inspect exact line deltas, for example:

```powershell
Compare-Object (Get-Content .\sonic.txt) (Get-Content .\dolby.txt)
Compare-Object (Get-Content .\dolby.txt) (Get-Content .\dts.txt)
```

A useful P2 result is a small repeatable delta that tracks the provider selected through the normal Windows UI. Unrelated registry churn is not provider-selection evidence. The snapshot tool does not itself select a provider, edit endpoint state, or prove object delivery.

## Future installer transaction

The spatial provider must eventually join `OmniphonySetup.exe` as a transaction, not as an optimistic registry side effect.

The desired progression is:

```text
stage immutable generation                       implemented as inert primitive
→ verify package hashes + final-path smokes      designed into staging primitive
→ capture prior provider and selection state
→ switch only Omniphony-owned registration to the candidate generation
→ verify COM activation and capability contract
→ verify public stream and endpoint-output path
→ select/enable only after end-to-end transport is proven
→ verify ordinary stereo/non-spatial audio still works
→ commit active-generation state
```

On failure or uninstall:

```text
stop using Omniphony provider if selected
→ restore prior provider/selection state when Omniphony changed it
→ restore previous Omniphony generation if activation failed
→ unregister only Omniphony-owned keys
→ retire old generation files only after COM users release them
→ leave the physical audio driver untouched
```

The installer must never leave Windows pointing at a provider that accepts a spatial stream but drops its audio. Keeping the public stream gate closed until COM input, Current, and final endpoint output are joined is part of installation safety.

## Clean removal of the registration experiment

From an elevated terminal:

```powershell
.\OmniphonySpatialProbeCtl.exe unregister
.\OmniphonySpatialProbeCtl.exe status
```

After `unregister`, `status` should again return exit code 3 and report both owned keys absent.

A final normal-terminal capture can prove the owned registration disappeared without relying on memory:

```powershell
.\CaptureSpatialProviderState.ps1 > after-unregister.txt
```

## Evidence states

Keep these claims separate:

```text
source compiles
≠ registry-free static COM lifecycle works
≠ VT_BLOB activation marshalling works
≠ static-object realtime ABI reaches Current
≠ C++ bridge drives that ABI
≠ immutable provider generation stages successfully
≠ read-only provider/endpoint snapshot succeeds
≠ registry registration succeeds
≠ COM IUnknown / ISpatialAudioClient activation succeeds
≠ Windows Settings enumerates Omniphony
≠ Windows accepts Omniphony as selected provider
≠ COM object quanta reach the realtime bridge
≠ public spatial render stream activates
≠ returned stereo reaches the real headphone endpoint
≠ static objects arrive from a real application end to end
≠ dynamic XYZ objects arrive
≠ Omniphony renders them correctly in listening tests
```

The internal machinery reduces the amount of code behind the remaining Windows boundary. It does not promote uncompiled source, registry-free tests, or provider registration experiments into physical application proof.

## Primary platform references

- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `ISpatialAudioClient::IsSpatialAudioStreamAvailable`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioclient-isspatialaudiostreamavailable
- `ISpatialAudioClient::ActivateSpatialAudioStream`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioclient-activatespatialaudiostream
- `SpatialAudioObjectRenderStreamActivationParams`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ns-spatialaudioclient-spatialaudioobjectrenderstreamactivationparams
- `AudioObjectType`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ne-spatialaudioclient-audioobjecttype
- Spatial object rendering: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects

Open-source comparison implementation:

- MSSOAL: https://github.com/ThreeDeeJay/MSSOAL
