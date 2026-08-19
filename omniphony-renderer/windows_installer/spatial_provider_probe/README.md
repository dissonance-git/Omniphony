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

Extract `OmniphonySpatialProbeCtl.exe` and `OmniphonySpatialProbe.dll` into the same directory and leave them there while the probe is registered.

First capture the current state from a normal terminal:

```powershell
.\OmniphonySpatialProbeCtl.exe contract
.\OmniphonySpatialProbeCtl.exe list
.\OmniphonySpatialProbeCtl.exe status
```

`status` returns exit code 3 before registration by design.

Then open **PowerShell as Administrator** in that directory and run:

```powershell
.\OmniphonySpatialProbeCtl.exe register .\OmniphonySpatialProbe.dll
.\OmniphonySpatialProbeCtl.exe diagnose
```

The registration command verifies that the COM DLL can be activated through the newly written CLSID before leaving the registry state in place.

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

Preserve the `list`, `status`, and `diagnose` output. The next step is read-only Process Monitor / registry-delta observation around a known provider rather than progressively broader registry writes.

### Result B — `Omniphony` appears but cannot be selected

This proves **provider enumeration** and advances the experiment to COM/provider capability negotiation.

That is a meaningful success. The current object intentionally has no render stream, so selection may reasonably stop here.

The next source change is the smallest valid `ISpatialAudioObjectRenderStream` implementation, initially static-object-only and silence-safe.

### Result C — Windows accepts/selects the capability-only provider

Record this separately. It would show that provider selection can precede stream creation, but it would still not prove any spatial application can render through Omniphony.

Do not leave the probe selected for normal listening because this build cannot activate a render stream.

## Clean removal

From an elevated terminal:

```powershell
.\OmniphonySpatialProbeCtl.exe unregister
.\OmniphonySpatialProbeCtl.exe status
```

After `unregister`, `status` should again return exit code 3 and report both owned keys absent.

## Evidence states

Keep these claims separate:

```text
build succeeds
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

The current probe is complete when enumeration and capability negotiation have real-machine results.

## Primary platform references

- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `AudioObjectType`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ne-spatialaudioclient-audioobjecttype
- Spatial object rendering: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects

Open-source comparison implementation:

- MSSOAL: https://github.com/ThreeDeeJay/MSSOAL
