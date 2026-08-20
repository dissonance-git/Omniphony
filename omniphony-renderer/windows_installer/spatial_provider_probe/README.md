# Omniphony Windows Spatial Sound provider probe

This is a bounded Windows experiment for the platform seam Omniphony ultimately needs to occupy as a system spatial renderer.

The current questions remain deliberately separate:

1. Can an independently registered Omniphony format be enumerated by the Windows **Spatial sound** selector?
2. If Windows activates the registered COM class, will it accept a standards-shaped `ISpatialAudioClient` capability object?
3. Can a real static Spatial Audio object cross the COM stream, Current, a single-render RAW egress path, and the final physical endpoint without another headphone renderer touching it?

This probe is **not yet a proven Windows spatial provider** and it does not change Omniphony's production signal path.

## Current source frontier

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
exact physical endpoint event cadence
        ↓
RAW IAudioRenderClient egress
```

Everything through the queue is implemented, compiled, and registry-free-smoked. The endpoint-event output pump is also implemented and compiled, with a no-device fail-closed smoke. The final line has **not yet been physically proven** on the user's endpoint.

The COM DLL implements `ISpatialAudioClient`. The repository contains an internal static-only `ISpatialAudioObjectRenderStream` lifecycle that accepts the documented `VT_BLOB` activation shape and validates the structure size, requested interface, object format, static mask, and zero dynamic-object capacity.

At each completed internal COM update pass, object buffers are snapshotted into one descriptor order derived at activation. Per-object volume and partial end-of-stream semantics are applied, inactive roles remain silence rather than changing topology, and the quantum is handed to the pre-opened realtime transport.

The realtime transport can submit each complete Current stereo quantum directly into a pre-opened `OmniphonySpatialStereoQueue`. Queue submission is allocation-free and non-blocking. It never overwrites unread endpoint audio; a full queue rejects the producer block and exposes the overflow instead.

`OmniphonySpatialRawOutputSink` initializes one exact physical endpoint as shared event-driven RAW float32 stereo, selects the endpoint's own legal default engine period, obtains `IAudioRenderClient`, binds the sample-ready event, and can remain deliberately unstarted for install preflight.

`OmniphonySpatialRawOutputPump` is the separate closed-gate active consumer. It pre-rolls silence, calls `IAudioClient::Start`, waits on the endpoint's sample-ready event, queries `GetCurrentPadding`, drains exactly the writable frames from the SPSC queue into `IAudioRenderClient`, releases the buffer, and exposes real/silence/drain counters. It introduces no independent playback timer.

The 480-frame object/render quantum is therefore not a physical endpoint requirement. The SPSC queue is the clock-domain adapter between Omniphony's fixed producer quantum and whatever legal shared-engine period the selected endpoint owns.

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
COM quantum -> Current               compiled + registry-free smoke
Current stereo -> SPSC queue         compiled + registry-free smoke
RAW APO single-render bypass         implemented in source
RAW endpoint capability probe        implemented read-only
RAW output sink initialization       implemented + compiled
480 -> endpoint-period queue         compiled + smoke-tested
endpoint-event RAW output pump       compiled + fail-closed smoke
finite physical egress diagnostic    compiled; real endpoint run pending
immutable provider staging           implemented as inert primitive
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
- the read-only RAW capability probe never initializes playback;
- the install-preflight RAW sink may initialize an exact endpoint but exposes no public `Start()` operation;
- the active pump is a separate development-only closed-gate component;
- the clock-domain queue allocates only during control-path `Open`;
- the finite physical egress probe requires an explicit endpoint, runs for a bounded duration, and performs no provider registration or selection;
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

MSSOAL independently explored the same `Spatial\Encoder` surface. It remains a mechanism quarry, not proof. Its stream source is useful as a warning against an unrelated free-running render clock: that project documents moving uploads back onto the Spatial Audio update cadence after a prior two-clock design produced drift/stutter.

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

This is registry-free evidence only.

## Registry-free realtime bridge and queued output

`OmniphonySpatialRealtimeBridge` is the narrow C++ boundary to the existing Rust realtime ABI. It requires an explicit absolute `omniphony_realtime.dll` path, resolves all static-object exports before processing, validates ABI compatibility, and destroys the processor before unloading its supplying module.

`OmniphonySpatialRealtimeBridgeSmoke` exercises both the narrow ABI and the composed COM-shaped path. The composed path additionally proves that every successful 480-frame Current stereo result can enter the downstream clock-domain queue and be read back without producer drops:

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

The intended clock law is:

> **Current owns its render quantum. The physical endpoint owns downstream playback cadence. The queue crosses the boundary without forcing those periods to be equal.**

The exact production queue depth is not frozen. It should be selected and measured as an explicit latency-versus-resilience parameter rather than hidden as a magic constant.

## Physical-output capability, inert lifecycle, and active pump

`OmniphonySpatialRawOutputProbe.exe` is read-only. Given an explicit endpoint ID it inspects RAW client support, stereo float32 / 48 kHz support, and default/fundamental/minimum/maximum shared-engine periods. It reports whether 480 frames happens to be legal, but that result is diagnostic rather than an installation gate.

`OmniphonySpatialRawOutputSinkProbe.exe` initializes the same exact endpoint as an event-driven shared RAW stereo stream, obtains `IAudioRenderClient`, binds the sample-ready event, records the endpoint-selected period and buffer size, then closes without calling `Start()`.

`OmniphonySpatialRawOutputPump` goes one source step farther behind the closed gate. The Windows diagnostic workflow compiles it and runs a no-endpoint lifecycle smoke. That smoke proves fail-closed state handling without opening a physical device or starting playback.

Microsoft's Windows Audio Session renderer sample motivates the consumer shape used here:

```text
pre-roll endpoint buffer
→ Start
→ wait for sample-ready event
→ GetCurrentPadding
→ compute writable frames
→ GetBuffer
→ fill exactly writable frames
→ ReleaseBuffer
```

Omniphony follows that shape while sourcing frames from its preallocated SPSC queue.

## Finite closed-gate physical egress diagnostic

`OmniphonySpatialClosedGateEgressProbe.exe` is the next physical test. CI compiles it but **must never run it** because it intentionally produces a short low-level audible signal on a named real endpoint.

Usage after building:

```powershell
.\OmniphonySpatialClosedGateEgressProbe.exe `
  C:\absolute\path\omniphony_realtime.dll `
  '<physical-endpoint-id>' `
  1500
```

The optional duration is bounded to 250–5000 ms.

The probe:

1. creates an internal static stream with `FrontLeft` and `TopFrontLeft`;
2. generates two quiet diagnostic tones at distinct frequencies;
3. sends them through the same COM-shaped stream and Current worker;
4. submits Current's stereo result into the SPSC queue;
5. opens only the explicitly named RAW physical endpoint;
6. pre-fills several render quanta before endpoint start;
7. starts the endpoint output pump;
8. runs a dedicated MMCSS `Pro Audio` worker waiting on the endpoint sample-ready event;
9. drains `IAudioRenderClient` until the finite diagnostic ends;
10. stops and resets the endpoint and internal stream.

A successful run must report:

```text
SPATIAL_CLOSED_GATE_EGRESS_OK 1
SPATIAL_CLOSED_GATE_EGRESS_COM_TO_CURRENT 1
SPATIAL_CLOSED_GATE_EGRESS_CURRENT_TO_QUEUE 1
SPATIAL_CLOSED_GATE_EGRESS_ENDPOINT_EVENT_CLOCK 1
SPATIAL_CLOSED_GATE_EGRESS_RAW_RENDER_CLIENT 1
SPATIAL_CLOSED_GATE_EGRESS_QUEUE_DROPPED_FRAMES 0
SPATIAL_CLOSED_GATE_EGRESS_PROVIDER_REGISTERED 0
SPATIAL_CLOSED_GATE_EGRESS_PROVIDER_SELECTED 0
SPATIAL_CLOSED_GATE_EGRESS_PUBLIC_PROVIDER_GATE_OPENED 0
```

Underrun frames are reported rather than silently hidden. Their acceptable production target is not frozen until the real endpoint has been measured.

Success would prove the **output half** of the architecture independently of the undocumented provider-registration seam. It would still not prove that Windows can enumerate/select Omniphony or feed real application objects into it.

## Immutable provider generations

The future installer must never overwrite a COM DLL that Windows or an application may still have loaded. `Stage-OmniphonySpatialProvider.ps1` prepares immutable content-addressed generations before any registration transaction exists.

Required staged package members remain deliberately inert:

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

The active pump and finite audible egress probe are **not staged as ordinary installer payload yet**. They remain development evidence tools until physical output is proven.

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

Before any provider mutation it verifies:

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

The preflight report records both `renderer_quantum_frames = 480` and the actual `endpoint_period_frames`, plus whether a cadence adapter is required. A non-480 endpoint period is valid if the endpoint accepts it and the queue contract is present.

The preflight may initialize and close the endpoint stream, but it does not start playback, register/select the provider, or open the public Spatial Audio stream.

## Provider enumeration experiment

The registry experiment remains useful because the Windows provider seam itself is unproven, but it follows the closed-gate physical output experiment in the current evidence order.

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
→ separately prove finite closed-gate physical egress
→ record prior provider/selection state
→ switch only Omniphony-owned registration to candidate generation
→ verify COM activation/capability
→ verify public stream end to end over the proven egress path
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
≠ endpoint-event pump compiles and passes no-device contract smoke
≠ RAW endpoint capability preflight succeeds
≠ inert endpoint output initialization succeeds
≠ finite closed-gate physical egress reaches the real endpoint
≠ immutable provider generation stages successfully
≠ provider registration succeeds
≠ Windows Settings enumerates Omniphony
≠ Windows selects Omniphony
≠ public spatial render stream activates
≠ a real application sends static objects end to end
≠ dynamic XYZ objects arrive
≠ listening validation succeeds
```

The source machinery deliberately advances one boundary at a time. Compiled code and registry-free smokes are not promoted into physical Windows proof.

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