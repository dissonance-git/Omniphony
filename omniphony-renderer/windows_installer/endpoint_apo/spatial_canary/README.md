# Omniphony Spatial Provider Phase 1 Canary

Status: **experimental interface-discovery gate**

This directory answers one question before Omniphony implements a Windows Spatial Sound provider:

> When Windows selects a third-party entry from the observed Spatial `Encoder` registry surface, what COM interface does Windows actually request from that provider CLSID?

The canary intentionally does **not** implement `ISpatialAudioClient` or any guessed private encoder interface. Its `IClassFactory::CreateInstance` records the requested IID and returns `E_NOINTERFACE`.

That failure is deliberate. Phase 1 is interface discovery, not audio rendering.

## Why this experiment changed shape

The GitHub + SciSpace research pass found two distinct layers that must not be conflated.

### Public application-side contract

Microsoft's public `ISpatialAudioClient` documentation describes an application client creating spatial streams, activating static/dynamic objects, supplying object PCM, and updating dynamic positions. It does not document third-party Spatial Sound provider registration or a provider-side COM ABI.

### Undocumented provider / cross-process machinery

The experimental `ThreeDeeJay/MSSOAL` repository registers a format GUID under:

```text
HKLM\SOFTWARE\Microsoft\Multimedia\Audio\Spatial\Encoder\{format-guid}
```

and maps that format to a COM CLSID. Its implementation assumes Windows will instantiate an `ISpatialAudioClient` implementation. The repository explicitly describes itself as a non-working AI proof of concept, so that assumption is not implementation authority.

Independent Windows binary-string snapshots on GitHub contain stronger clues. Windows 10 `AudioHandlers.dll` exposes names including:

```text
Create_SpatialAudioEncoderProperties
SpatialAudioConfigureDevice::ConfigureForSpatialAudioEncoder
SpatialAudioDevicePropertyReader::GetCurrentSpatialAudioEncoderId
SpatialAudioEncoderProperties::GetEncoderProperties
Software\Microsoft\Multimedia\Audio\Spatial\Encoder
```

`AudioSes.dll` also exposes a substantial cross-process spatial path, including:

```text
CreateSpatialCrossProcessEndpoint
CSpatialCrossProcessClientOutputEndpoint
ActivateSpatialAudioMetadataReader
ActivateSpatialAudioMetadataWriter
proxyspatialaudioclient.cpp
guestspatialaudioclient.cpp
spatialcpclientendpoint.cpp
spatialcpserverendpoint.cpp
```

Equivalent `SpatialAudioEncoderProperties` and spatial cross-process symbols are present in Windows 11 build 22621/22622 snapshots as well.

This does **not** reveal the private ABI, but it makes one thing clear: copying MSSOAL's `ISpatialAudioClient` assumption before observing the requested IID would be premature.

## SciSpace consequences

The literature pass supports preserving a renderer-independent object scene until the final binaural stage:

- Landschoot & Jot, *Binaural externalization processing method for object-based audio rendering* (2023), DOI `10.1121/10.0018389`: object-aware externalization remains a separate rendering problem rather than a reason to destroy source geometry.
- Jot et al., *Rendering Spatial Sound for Interoperable Experiences in the Audio Metaverse* (2021): practical interactive object engines retain parametric source properties such as position, distance, size/orientation and acoustic behavior across reproduction systems.
- Yuan et al., *Externalization improvement in a real-time binaural sound image rendering system* (2015), DOI `10.1109/ICOT.2015.7498514`: dynamic relative source direction plus bounded early/late environmental cues can improve externalization while preserving direction.

These findings affect later rendering phases. They do not establish any Windows provider registration contract.

## Components

```text
OmniphonySpatialProviderCanary.dll
    COM in-process class factory only
    records the IID requested by CreateInstance
    returns E_NOINTERFACE

OmniphonySpatialProviderCanaryCtl.exe
    status
    register <dll>
    listen
    selftest <dll>
    unregister
```

Provider CLSID:

```text
{7AEE0F13-1F6B-4D83-9F6D-6C9C0E33A151}
```

Experimental format GUID:

```text
{3DBFF1AF-0FC6-4A32-8289-5E652C987D92}
```

The controller writes only the observed 64-bit COM CLSID and Spatial `Encoder` keys. It does **not** write `MMDevices\SpatialAudioEndpoint`, change ACLs, create services, take ownership, or programmatically select the format.

## CI gate

The focused workflow builds only this disposable canary and runs:

```powershell
OmniphonySpatialProviderCanaryCtl.exe selftest .\OmniphonySpatialProviderCanary.dll
```

The selftest proves the DLL exports a working class factory and that an arbitrary requested IID is recorded/rejected. It does not prove Windows activation.

## Physical Phase 1 experiment

Use an elevated terminal only for registration/unregistration.

```powershell
.\OmniphonySpatialProviderCanaryCtl.exe register .\OmniphonySpatialProviderCanary.dll
.\OmniphonySpatialProviderCanaryCtl.exe status
```

In a second normal terminal, start the one-shot listener:

```powershell
.\OmniphonySpatialProviderCanaryCtl.exe listen
```

Then open the ordinary Windows Spatial Sound UI and select **Omniphony Spatial Canary (EXPERIMENT)** if Windows enumerates it.

A Phase 1 activation witness looks like:

```text
phase1.activation=observed
omniphony_spatial_provider_canary ... process="..." requested_iid={...}
```

The experiment deliberately returns `E_NOINTERFACE`, so Windows may reject or immediately revert the selection. That is expected.

Before unregistering, select Windows Sonic or Spatial Sound Off in the normal Windows UI. Then:

```powershell
.\OmniphonySpatialProviderCanaryCtl.exe unregister
```

## Phase 1 verdicts

**PASS:** Windows enumerates the canary and invokes `CreateInstance`, giving us the exact requested provider IID.

**PARTIAL:** Windows enumerates the canary but never invokes the COM class. The registry surface is real UI/configuration state but not sufficient to activate a provider.

**FAIL:** Windows neither enumerates nor activates the canary on a representative Windows 11 machine. Revisit the registration hypothesis rather than adding renderer code.

No Phase 2 object-stream implementation begins until this gate produces a concrete provider-side interface witness or a different activation mechanism is independently proven.
