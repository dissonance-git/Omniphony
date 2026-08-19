# Omniphony Windows Spatial Sound provider probe

This is a bounded Windows experiment for the platform seam Omniphony ultimately needs to occupy as a system spatial renderer.

The current questions are deliberately separated:

1. Can an independently registered Omniphony format be enumerated by the Windows **Spatial sound** selector?
2. If Windows activates the registered COM class, will it accept a standards-shaped `ISpatialAudioClient` capability object?
3. Which additional stream behavior is required before Windows accepts it as a functional spatial renderer?

This probe is **not yet an audible spatial renderer** and it does not change Omniphony's production signal path.

## Current capability stage

The COM DLL now implements `ISpatialAudioClient` rather than only `IUnknown`.

It truthfully exposes:

```text
static object vocabulary    17 roles / 8.1.4.4
object format               mono float32 / 48 kHz
max frame count             480 frames
max dynamic objects         0
render stream available     no
render stream activation    no
```

The complete static mask is useful because it lets Windows query the semantic scene Omniphony intends to support without pretending the stream implementation already exists.

`GetMaxDynamicObjectCount` intentionally returns `0`. Dynamic capacity will increase only when a real dynamic-object stream exists.

`IsSpatialAudioStreamAvailable` and `ActivateSpatialAudioStream` intentionally return `SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE`. Returning success before an actual stream exists would be false capability advertising.

## Safety boundary

The probe remains smaller than the product path:

- it writes only two project-owned registry subtrees;
- it does not write `MMDevices` state;
- it does not change the default playback endpoint;
- it does not install a virtual audio device;
- it does not restart Windows Audio;
- it does not hook or inject into applications;
- it does not replace Windows system files or HRTFs;
- it performs no audio processing;
- it cannot activate a spatial render stream yet;
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

That is a meaningful success. The current object intentionally has no render stream, so selection may reasonably stop here.

The next source change is the smallest valid `ISpatialAudioObjectRenderStream` implementation, initially static-object-only and silence-safe.

### Result C — Windows accepts/selects the capability-only provider

Record this separately. It would show that provider selection can precede stream creation, but it would still not prove any spatial application can render through Omniphony.

Do not leave the probe selected for normal listening because this build cannot activate a render stream.

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
≠ read-only provider/endpoint snapshot succeeds
≠ registry registration succeeds
≠ COM IUnknown activation succeeds
≠ ISpatialAudioClient capability query succeeds
≠ Windows Settings enumerates Omniphony
≠ Windows accepts Omniphony as selected provider
≠ spatial render stream activates
≠ static objects arrive
≠ dynamic XYZ objects arrive
≠ Omniphony renders them correctly
```

The current probe is complete when enumeration and capability negotiation have real-machine results. The snapshot machinery makes those results reproducible; it does not promote them to a later evidence state.

## Primary platform references

- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `AudioObjectType`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ne-spatialaudioclient-audioobjecttype
- Spatial object rendering: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects

Open-source comparison implementation:

- MSSOAL: https://github.com/ThreeDeeJay/MSSOAL
