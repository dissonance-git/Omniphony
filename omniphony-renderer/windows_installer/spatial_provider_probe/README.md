# Omniphony Windows Spatial Sound provider probe

This is a bounded Windows experiment for the platform seam Omniphony ultimately needs to occupy as a system spatial renderer.

The current questions are deliberately separated:

1. Can an independently registered Omniphony format be enumerated by the Windows **Spatial sound** selector?
2. If Windows activates the registered COM class, will it accept a standards-shaped `ISpatialAudioClient` capability object?
3. Which additional stream and transport behavior is required before Windows accepts it as a functional spatial renderer?

This probe is **not yet an audible spatial renderer** and it does not change Omniphony's production signal path.

## Current capability stage

The COM DLL implements `ISpatialAudioClient` rather than only `IUnknown`.

The repository also contains an internal static-only `ISpatialAudioObjectRenderStream` lifecycle implementation. That stream now accepts the same `VT_BLOB` activation shape documented for `ISpatialAudioClient::ActivateSpatialAudioStream`, including exact structure-size, interface-ID, object-format, static-mask, and zero-dynamic-capacity validation.

The public provider deliberately does **not** expose that internal stream yet because its object buffers do not have a downstream transport into Omniphony's realtime renderer. Advertising it now would allow a spatial application to submit audio that the probe cannot render.

Current state:

```text
static object vocabulary        17 roles / 8.1.4.4
object format                   mono float32 / 48 kHz
max frame count                 480 frames
max dynamic objects             0
internal static stream          implemented
VT_BLOB activation marshalling  implemented
provider render stream available no
provider render stream activation no
downstream Omniphony transport  not yet implemented
```

`GetMaxDynamicObjectCount` intentionally returns `0`. Dynamic capacity will increase only when a real dynamic-object path exists.

`IsSpatialAudioStreamAvailable` and `ActivateSpatialAudioStream` intentionally return `SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE`. The internal stream is preparation behind that gate, not permission to make an end-to-end claim early.

## Safety boundary

The probe remains smaller than the product path:

- it writes only two project-owned registry subtrees;
- it does not write `MMDevices` state;
- it does not change the default playback endpoint;
- it does not install a virtual audio device;
- it does not restart Windows Audio;
- it does not hook or inject into applications;
- it does not replace Windows system files or HRTFs;
- it performs no production audio processing;
- the public provider cannot activate a spatial render stream yet;
- `unregister` deletes only the two Omniphony probe keys.

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

The same `Spatial\Encoder` surface has also been independently explored by the open-source MSSOAL project. That is useful corroborating implementation evidence, not a substitute for a real-machine Omniphony result.

## Registry-free static-stream contract

`OmniphonySpatialStaticStreamSmoke` exercises the internal stream without registry writes or Windows provider selection.

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

This proves an internal COM-shaped stream contract only. It does not prove provider enumeration, Windows stream activation, object delivery from another process, downstream rendering, or audible output.

## Run the experiment

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

That is a meaningful success. The provider intentionally still reports render-stream activation unavailable.

The next source change is no longer "invent a static stream from scratch." The internal static lifecycle exists. The next engineering frontier is a bounded downstream transport that preserves static role identity, PCM, update cadence, and source authority into the existing Omniphony realtime renderer. Only after that transport exists should the public provider delegate `ActivateSpatialAudioStream` to the internal stream factory.

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

## Future installer integration contract

The spatial provider must eventually join `OmniphonySetup.exe` as a transaction, not as an optimistic registry side effect.

The installer path should be:

```text
stage provider binaries
→ verify files and COM exports
→ record prior provider/selection state
→ register only Omniphony-owned keys
→ verify COM activation and capability contract
→ enable/select only after end-to-end stream transport is proven
→ verify ordinary stereo/non-spatial audio still works
→ commit installation
```

On failure or uninstall:

```text
stop using Omniphony provider if selected
→ restore prior provider/selection state when Omniphony changed it
→ unregister only Omniphony-owned keys
→ remove provider files after COM users release them
→ leave the physical audio driver untouched
```

The installer must never leave Windows pointing at a provider that accepts a spatial stream but drops its audio. Keeping the public stream gate closed until downstream transport exists is part of installation safety, not merely experimental caution.

## Clean removal

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
build succeeds
≠ registry-free static stream lifecycle succeeds
≠ VT_BLOB activation marshalling succeeds
≠ read-only provider/endpoint snapshot succeeds
≠ registry registration succeeds
≠ COM IUnknown activation succeeds
≠ ISpatialAudioClient capability query succeeds
≠ Windows Settings enumerates Omniphony
≠ Windows accepts Omniphony as selected provider
≠ public spatial render stream activates
≠ static objects arrive from a real application
≠ object PCM reaches Omniphony Current
≠ dynamic XYZ objects arrive
≠ Omniphony renders them correctly
```

The current probe is complete when enumeration and capability negotiation have real-machine results. The internal stream machinery reduces the amount of code needed after that gate; it does not promote the experiment to a later evidence state.

## Primary platform references

- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `ISpatialAudioClient::ActivateSpatialAudioStream`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioclient-activatespatialaudiostream
- `SpatialAudioObjectRenderStreamActivationParams`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ns-spatialaudioclient-spatialaudioobjectrenderstreamactivationparams
- `AudioObjectType`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ne-spatialaudioclient-audioobjecttype
- Spatial object rendering: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects

Open-source comparison implementation:

- MSSOAL: https://github.com/ThreeDeeJay/MSSOAL
