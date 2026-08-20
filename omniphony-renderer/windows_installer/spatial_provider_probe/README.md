# Omniphony Windows Spatial Sound provider probe

This is a bounded Windows experiment for the platform seam Omniphony ultimately needs to occupy as a system spatial renderer.

The current questions remain deliberately separate:

1. Can an independently registered Omniphony format be enumerated by the Windows **Spatial sound** selector?
2. If Windows activates the registered COM class, will it accept a standards-shaped `ISpatialAudioClient` capability object?
3. Can a real static Spatial Audio object cross the COM stream, Current, a single-render RAW egress path, and the final physical endpoint without another headphone renderer touching it?

This probe is **not yet an audible Windows spatial renderer** and it does not change Omniphony's production signal path.

## Current capability stage

The COM DLL implements `ISpatialAudioClient`. The repository also contains an internal static-only `ISpatialAudioObjectRenderStream` lifecycle that accepts the documented `VT_BLOB` activation shape and validates the structure size, requested interface, object format, static mask, and zero dynamic-object capacity.

The closed-gate source path is now:

```text
Windows static role + authored position
        ↓
COM-shaped fixed-topology static stream
        ↓
immutable planar object quantum
        ↓
C++ realtime bridge
        ↓
omniphony_realtime.dll
        ↓
WindowsStaticObjectPipeline worker
        ↓
existing source-aware Current renderer
        ↓
480-frame binaural stereo quantum
        ↓
preallocated SPSC stereo queue
        ↓
endpoint-owned event cadence                 not active yet
        ↓
RAW stereo physical endpoint                 not physically proven yet
```

At each completed internal COM update pass, object buffers are snapshotted into one descriptor order derived at activation. Per-object volume and partial end-of-stream semantics are applied, inactive roles remain silence rather than changing topology, and the quantum is handed to the pre-opened realtime transport.

The realtime transport can now optionally submit each complete Current stereo quantum directly into a pre-opened `OmniphonySpatialStereoQueue`. Queue submission is allocation-free and non-blocking. It never overwrites unread endpoint audio; a full queue rejects the producer block and exposes the overflow instead.

The physical-output control path is also farther along. `OmniphonySpatialRawOutputSink` can initialize one exact physical endpoint as shared event-driven RAW float32 stereo, select the endpoint's own legal default engine period, obtain `IAudioRenderClient`, bind the sample-ready event, and then remain deliberately **unstarted**.

The 480-frame object/render quantum is therefore no longer treated as a physical endpoint requirement. A preallocated SPSC queue is the clock-domain adapter between Omniphony's fixed producer quantum and whatever legal shared-engine period the selected endpoint owns.

The public provider deliberately does **not** expose the static stream yet. The active endpoint-event consumer has not been completed or physically verified. Accepting a public Spatial Audio stream before that boundary works could create a silent sink, so the gate remains closed.

Current state:

```text
static object vocabulary             17 roles / 8.1.4.4
object format                        mono float32 / 48 kHz
Current render quantum               480 frames
max dynamic objects                  0
internal static COM stream           implemented
VT_BLOB activation marshalling       implemented
static object -> Current worker      implemented
C++ realtime ABI bridge              implemented
COM quantum -> Current               implemented registry-free
Current stereo -> SPSC queue         implemented behind closed gate
RAW APO single-render bypass         implemented in source
RAW endpoint capability probe        implemented read-only
RAW output sink initialization       implemented, deliberately unstarted
480 -> endpoint-period queue         implemented in source
immutable provider staging           implemented as inert primitive
active endpoint-event queue drain    pending
real provider enumeration            pending physical proof
provider render stream available     no
provider render stream activation    no
```

`GetMaxDynamicObjectCount` intentionally returns `0`. Dynamic capacity will increase only when a real dynamic-object path exists.

`IsSpatialAudioStreamAvailable` and `ActivateSpatialAudioStream` intentionally return `SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE`. The internal machinery behind that gate is preparation, not permission to make an end-to-end claim early.

## Safety boundary

The experiment remains smaller than the product path:

- explicit registration writes only the two project-owned registry subtrees;
- it does not write `MMDevices` state;
- it does not change the default playback endpoint;
- it does not install a virtual audio device;
- it does not hook or inject into applications;
- it does not replace Windows system files or HRTFs;
- the public provider cannot activate a spatial render stream yet;
- `unregister` deletes only Omniphony-owned probe keys;
- the read-only RAW capability probe never initializes a playback stream;
- the RAW output sink probe may initialize an exact endpoint stream but deliberately has no `Start()` operation;
- the clock-domain queue allocates only during control-path `Open`, not producer/consumer processing;
- immutable package staging performs **no provider registration and no provider selection**.

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

The same `Spatial\Encoder` surface has independently been explored by MSSOAL. MSSOAL is useful as an implementation quarry, not proof. Its current stream source is especially useful as a warning against adding an unrelated free-running render clock: the project documents moving object uploads back onto the Spatial Audio update cadence after a prior two-clock design produced drift/stutter.

## Registry-free static-stream contract

`OmniphonySpatialStaticStreamSmoke` exercises the COM-shaped lifecycle without registry writes or provider selection. It covers:

- documented `VT_BLOB` activation payload shape;
- exact activation-structure size;
- supported `ISpatialAudioObjectRenderStream` IID;
- zero dynamic-object capacity;
- canonical static-mask validation;
- static object activation and duplicate/unavailable-role rejection;
- update ordering and 480-frame object buffers;
- fixed static positions;
- volume validation;
- partial and implicit end-of-stream behavior;
- static-role reactivation;
- start, stop, and reset lifecycle.

This is source/registry-free evidence only.

## Registry-free realtime bridge and queued output

`OmniphonySpatialRealtimeBridge` is the narrow C++ boundary to the existing Rust realtime ABI. It requires an explicit absolute `omniphony_realtime.dll` path, resolves all static-object exports before processing, validates ABI compatibility, and destroys the processor before unloading its supplying module.

`OmniphonySpatialRealtimeBridgeSmoke` exercises both the narrow ABI and the composed COM-shaped path. The composed path now additionally proves that every successful 480-frame Current stereo result can enter the downstream clock-domain queue and be read back without producer drops:

```text
FL + TFL object PCM
→ internal ISpatialAudioObjectRenderStream lifecycle
→ immutable planar role order
→ OmniphonySpatialRealtimeBridge
→ omniphony_realtime.dll
→ Current
→ interleaved stereo
→ OmniphonySpatialStereoQueue
```

Expected markers include:

```text
SPATIAL_COM_TO_CURRENT_OK 1
SPATIAL_COM_TO_STEREO_QUEUE_OK 1
SPATIAL_FINAL_ENDPOINT_PROVEN 0
```

That final zero is intentional.

## Clock-domain queue

`OmniphonySpatialStereoQueue` is a preallocated single-producer/single-consumer ring in **stereo frames**, not bytes or renderer blocks.

After `Open`:

- producer `TryWrite` is non-blocking and allocation-free;
- producer writes a complete block or rejects it as a whole;
- consumer `Read` accepts any frame count, including periods different from 480;
- underrun tails are explicitly zero-filled;
- overflow and underrun frame counts are observable;
- wraparound preserves frame order;
- no second rendering operation occurs.

`OmniphonySpatialStereoQueueSmoke` deliberately tests a 480-frame producer against `128 + 224 + 128` consumer requests, plus wraparound, underrun, and overflow behavior.

This is the intended clock law:

> **Current owns its render quantum. The physical endpoint owns downstream playback cadence. The queue crosses the boundary without forcing those periods to be equal.**

The exact production queue depth is not frozen yet. It should be selected and measured as an explicit latency-versus-resilience parameter rather than smuggled in as a magic constant.

## Physical-output capability and inert lifecycle

`OmniphonySpatialRawOutputProbe.exe` is read-only. Given an explicit endpoint ID it inspects RAW client support, stereo float32 / 48 kHz support, and default/fundamental/minimum/maximum shared-engine periods. It reports whether 480 frames happens to be legal, but that result is now diagnostic rather than an installation gate.

`OmniphonySpatialRawOutputSinkProbe.exe` goes one bounded step farther. It initializes the same exact endpoint as an event-driven shared RAW stereo stream, obtains `IAudioRenderClient`, binds the sample-ready event, records the endpoint-selected period and buffer size, then closes without ever calling `Start()`.

Examples after building:

```powershell
.\OmniphonySpatialRawOutputProbe.exe '<physical-endpoint-id>'
.\OmniphonySpatialRawOutputSinkProbe.exe '<physical-endpoint-id>'
```

An optional second sink-probe argument can request an exact legal period for diagnostics. Omitting it uses the endpoint-reported default period, which is the normal preflight behavior.

Neither tool proves audible provider output.

## Immutable provider generations

The future installer must never overwrite a COM DLL that Windows or an application may still have loaded. `Stage-OmniphonySpatialProvider.ps1` prepares immutable content-addressed generations before any registration transaction exists.

Required package members are currently:

```text
OmniphonySpatialProbe.dll
omniphony_realtime.dll
OmniphonySpatialProbeCtl.exe
OmniphonySpatialProbeSmoke.exe
OmniphonySpatialStaticStreamSmoke.exe
OmniphonySpatialRealtimeBridgeSmoke.exe
OmniphonySpatialStereoQueueSmoke.exe
OmniphonySpatialRawOutputProbe.exe
OmniphonySpatialRawOutputSinkProbe.exe
CaptureSpatialProviderState.ps1
```

Staging:

- requires 64-bit PowerShell on 64-bit Windows;
- rejects unsafe source/managed-tree nesting;
- hashes every package member;
- derives generation identity from the complete sorted package hash set;
- copies into a temporary candidate generation;
- verifies the exact file set and every hash;
- runs capability, static-stream, realtime-bridge, and clock-domain queue smokes;
- moves the candidate into its immutable final generation path;
- repeats exact-file/hash/path-sensitive smoke verification from the final path;
- records paths for the queue smoke, RAW capability probe, and inert RAW output-sink probe;
- atomically writes `staged-generation.json`;
- explicitly records `registry_mutated=false` and `provider_selected=false`.

An existing generation is verified rather than modified.

## Activation preflight

`Preflight-OmniphonySpatialProvider.ps1` consumes a staged-generation manifest and one exact physical endpoint ID.

Before any provider mutation it now verifies:

```text
immutable package still exact
→ all hashes still exact
→ provider/static/Current smokes
→ COM quantum reaches Current
→ Current stereo reaches SPSC queue in registry-free smoke
→ standalone 480→variable-period queue contract
→ RAW stereo format support on exact endpoint
→ endpoint period constraints
→ inert event-driven RAW sink initialization
→ IAudioRenderClient + event ownership
→ sink remains unstarted
→ registry/provider state remains untouched
```

The preflight report records both:

- `renderer_quantum_frames = 480`
- the actual `endpoint_period_frames`

and whether a cadence adapter is required. A non-480 endpoint period is valid if the endpoint itself accepts that period and the queue contract is present.

The preflight may initialize and close the endpoint stream, but it does not start playback, register/select the provider, or open the public Spatial Audio stream.

## Next source frontier: endpoint event drain

The next code boundary is intentionally narrow:

```text
Current 480-frame stereo quantum
→ queue.TryWrite(...)
→ physical endpoint sample-ready event
→ IAudioClient::GetCurrentPadding
→ writable = endpointBufferFrames - padding
→ IAudioRenderClient::GetBuffer(writable)
→ queue.Read(..., writable)
→ zero-fill any underrun tail
→ IAudioRenderClient::ReleaseBuffer(writable, flags)
```

Microsoft's WASAPI renderer sample follows the same endpoint-owned consumption pattern: initialize event-driven output, pre-roll before start, react to the endpoint event, query current padding, and write only the available frames.

This drain should be built behind the closed provider gate first. Required properties:

- no filesystem/device discovery on the event path;
- no allocation on the event path;
- no second free-running playback clock;
- deliberate startup pre-roll;
- observable queue overflow/underrun;
- device invalidation fails closed;
- stop/close are idempotent;
- public `ActivateSpatialAudioStream` remains unavailable until physical end-to-end proof exists.

## Provider enumeration experiment

The registry experiment remains useful because the Windows provider seam itself is unproven.

Before registration:

```powershell
.\OmniphonySpatialProbeCtl.exe contract
.\OmniphonySpatialProbeCtl.exe list
.\OmniphonySpatialProbeCtl.exe status
.\CaptureSpatialProviderState.ps1 > before-registration.txt
```

From elevated PowerShell:

```powershell
.\OmniphonySpatialProbeCtl.exe register .\OmniphonySpatialProbe.dll
.\OmniphonySpatialProbeCtl.exe diagnose
```

Then capture again from a normal terminal and inspect Windows Settings → System → Sound → output device → Spatial sound.

Interpret results narrowly:

- not listed: current `Spatial\Encoder` enumeration hypothesis is falsified for that build;
- listed but not selectable: enumeration is proven, activation/selection remains unresolved;
- selectable capability-only provider: selection evidence only, still not object/output proof.

Do not leave this capability-only provider selected for ordinary listening.

## Future installer transaction

The spatial provider should join `OmniphonySetup.exe` only after the closed-gate output path is physically proven.

Desired transaction:

```text
stage immutable generation
→ verify package + final-path smokes
→ verify RAW APO single-render bypass
→ preflight exact physical endpoint
→ initialize and close inert RAW sink successfully
→ record prior provider/selection state
→ switch only Omniphony-owned registration to candidate generation
→ verify COM activation/capability
→ verify active endpoint-event drain and public stream end to end
→ select/enable only after transport proof
→ verify ordinary stereo/non-spatial audio still works
→ commit active-generation state
```

Failure/uninstall must restore any provider/selection state Omniphony changed, restore the previous Omniphony generation after failed activation, unregister only Omniphony-owned keys, retain in-use immutable generations until safe retirement, and leave the physical audio driver untouched.

The installer must never leave Windows selected on a provider that accepts a spatial stream but drops its audio.

## Evidence states

Keep these claims separate:

```text
source compiles
≠ static COM lifecycle works registry-free
≠ static-object ABI reaches Current
≠ COM-shaped quantum reaches Current
≠ Current stereo reaches SPSC queue
≠ queue handles variable consumer periods
≠ RAW endpoint capability preflight succeeds
≠ inert endpoint output initialization succeeds
≠ endpoint event drain runs stably
≠ immutable provider generation stages successfully
≠ provider registration succeeds
≠ Windows Settings enumerates Omniphony
≠ Windows selects Omniphony
≠ public spatial render stream activates
≠ returned stereo reaches the physical endpoint exactly once
≠ a real application sends static objects end to end
≠ dynamic XYZ objects arrive
≠ listening validation succeeds
```

The source machinery deliberately advances one boundary at a time. None of the registry-free or inert-output work is promoted into physical Windows proof.

## Primary references

Platform and implementation references:

- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft spatial-object rendering: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `ISpatialAudioClient::ActivateSpatialAudioStream`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioclient-activatespatialaudiostream
- `SpatialAudioObjectRenderStreamActivationParams`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ns-spatialaudioclient-spatialaudioobjectrenderstreamactivationparams
- Microsoft Windows Audio Session WASAPI renderer sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/WindowsAudioSession
- Microsoft SysVAD APO samples: https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO
- MSSOAL: https://github.com/ThreeDeeJay/MSSOAL

Realtime scheduling / buffering research quarry used for the cadence design:

- Burroughs, Parkin & Tzanetakis, *Flexible Scheduling for DataFlow Audio Processing* (ICMC 2006).
- Zhao et al., *Minimizing Latency and Data Memory Requirement for Real-time Chain-Structured Synchronous Dataflow* (SIES 2007), DOI `10.1109/SIES.2007.4297348`.
- Cucinotta, Faggioli & Bagnoli, *Low-Latency Audio on Linux by Means of Real-Time Scheduling* (2011).
- *ANIRA: An Architecture for Neural Network Inference in Real-Time Audio Applications* (2024/2025).

These sources support bounded realtime work and explicit buffering between timing domains. They do not prove the undocumented Windows provider seam.