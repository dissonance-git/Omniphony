# Omniphony roadmap

Omniphony's target is one free and open-source spatial renderer for Windows that enhances ordinary stereo, preserves authored surround and height, accepts true spatial scenes when the platform exposes them, and performs the final headphone render itself.

> **Preserve the richest source representation available and invent only what is missing.**

This roadmap tracks product capability, not individual experiments. Renderer fixtures, Windows graph proof, application proof, and listening proof remain distinct evidence states.

## Product invariant

All source types converge on one source-authority model and one final binaural renderer.

```text
stereo
→ preserve finished master
→ infer bounded missing spatial structure
→ Omniphony

5.1 / 7.1 / conventional height PCM
→ preserve authored speaker identity and position
→ infer less
→ same Omniphony

Windows Spatial Audio static objects
→ preserve supplied fixed spatial roles
→ same Omniphony

Windows Spatial Audio dynamic objects
→ preserve object identity + continuous XYZ
→ same Omniphony

all paths
→ one final binaural render
→ stereo headphone endpoint
```

`AUTHORED`, `DERIVED`, and `EMPTY` remain the source-authority states. Richer input increases source authority rather than selecting a different product mode.

---

## Phase 0: Windows stereo and conventional surround baseline

**State: accepted baseline**

Implemented and accepted:

- headless Windows endpoint deployment;
- protected stereo Current path;
- format-changing stream SFX;
- authored 7.1 shared-client ingress;
- authored channel-mask mapping into the source scene;
- one source-aware binaural renderer;
- exact stereo physical endpoint output;
- endpoint rollback and recovery floor;
- conventional 7.1 game use accepted as the production baseline.

Reference path:

```text
Windows stereo or authored 7.1 PCM
        ↓
Omniphony stream SFX
        ↓
source-authority mapping
        ↓
Current source-aware renderer
        ↓
stereo headphone endpoint
```

Do not reopen this phase merely to gather redundant telemetry unless a later regression makes the conventional path uncertain.

---

## Phase 1: Conventional authored height PCM

**State: renderer/APO support exists; application-level proof remains secondary**

The native-bed path already regression-tests authored 7.1.4 processing. Preserve useful conventional height layouts when an application actually opens them.

Important boundary:

> **Do not treat 8.1.4.4 as the next conventional WAVEFORMATEXTENSIBLE PCM milestone.**

Microsoft exposes the complete 17-role 8.1.4.4 vocabulary through Windows Spatial Audio static `AudioObjectType` objects. Conventional PCM remains a compatibility path; the full spatial target belongs to the object API.

Phase 1 therefore remains bounded:

```text
5.1 / 7.1 / supported height PCM
→ authored mask identity
→ canonical source scene
→ Omniphony
```

No synthetic lower speakers or back-center channels may be promoted to `AUTHORED` simply to fill the canonical frame.

---

## Phase 2: Windows Spatial Sound provider seam

**State: immediate frontier. Capability, internal static stream, COM-shaped static quantum transport into Current, C++ realtime bridge, RAW-mode APO single-render bypass, read-only RAW endpoint preflight, and hardened immutable package staging are implemented in source behind a closed public provider gate. Real Windows enumeration/selection and an event-driven final output/cadence boundary remain pending.**

This is the critical platform milestone.

Microsoft's public application contract is `ISpatialAudioClient`. Spatial applications can submit:

- up to 17 predefined static spatial roles, forming an 8.1.4.4 vocabulary;
- dynamic objects with arbitrary positions that can change over time;
- object PCM independently of the final headphone/speaker render format.

The active Windows spatial renderer is abstracted from the application, which is exactly the product position Omniphony ultimately wants to occupy.

Target topology:

```text
spatial application
        ↓
Windows Spatial Audio object API
        ↓
Omniphony spatial provider
        ↓
canonical static scene + dynamic object layer
        ↓
Omniphony binaural renderer
        ↓
RAW stereo egress with Omniphony APOs transparent
        ↓
physical headphone endpoint
```

### P0: endpoint capability

Already implemented as read-only probing:

- activate `ISpatialAudioClient` on the endpoint;
- query native static-object mask;
- query object formats;
- query dynamic-object capacity;
- inspect static object positions.

This proves endpoint capabilities only. It does not prove that Omniphony can become the selected provider.

A separate read-only `IAudioClient3` RAW output probe now inspects the intended physical endpoint without initializing or starting a render stream. It records endpoint identity, stereo float32 / 48 kHz support, shared-engine period constraints, and whether the current 480-frame quantum is legal. This is preflight evidence for the future output transaction, not output proof.

The installed APO side now has an explicit RAW contract as well. Both the endpoint and stream APO capture the Windows audio processing mode at initialization. In `AUDIO_SIGNALPROCESSINGMODE_RAW`, they do not load Current. The endpoint APO becomes an identity effect, while the stream APO also suppresses its normal 7.1 preferred-input expansion and accepts only an identity stereo float32 pair. A dedicated smoke encodes bit transparency, silent-buffer transparency, zero Omniphony latency, no `omniphony_realtime.dll` load, and 7.1 rejection in RAW mode.

This is the source-level condition required before a future provider may place already-rendered binaural stereo back onto the physical endpoint without double-rendering it.

### P1: provider enumeration

The repository contains a bounded provider-registration probe under:

```text
omniphony-renderer/windows_installer/spatial_provider_probe/
```

Its first real-machine question remains deliberately small:

> Can an independently registered Omniphony spatial format appear in the Windows Spatial sound selector?

The current candidate registration seam is:

```text
HKLM\SOFTWARE\Microsoft\Multimedia\Audio\Spatial\Encoder\{format-guid}
```

This is not a Microsoft-documented third-party provider API. It is an experimental boundary and must remain labeled as such.

### P2: provider activation

**Implementation state:** the COM provider exposes a standards-shaped `ISpatialAudioClient` capability object. It advertises the complete 17-role static vocabulary, one mono float32 / 48 kHz object format, a 480-frame capability quantum, and zero dynamic-object capacity.

The repository also contains an internal static-only `ISpatialAudioObjectRenderStream` lifecycle. Its activation helper accepts the documented `VT_BLOB` form used by `ISpatialAudioClient::ActivateSpatialAudioStream` and validates:

- exact activation-structure size;
- `ISpatialAudioObjectRenderStream` interface identity;
- object format;
- requested static mask;
- zero dynamic-object capacity.

Behind that COM-shaped stream, `omniphony_realtime.dll` exposes a fixed-topology static-object ABI. The Windows-facing side copies planar mono object quanta through preallocated rings while a dedicated worker owns `WindowsStaticObjectPipeline` and the existing source-aware Current renderer. Directional Windows positions remain authored geometry; LFE remains non-directional.

The C++ bridge dynamically loads that ABI from an explicit absolute DLL path, verifies the realtime ABI before processor creation, keeps the processor alive only while its supplying module remains loaded, and now composes directly with the internal COM-shaped stream. Each completed update quantum is snapshotted into the immutable role order, with object volume and partial end-of-stream semantics applied before transport:

```text
COM-shaped static object quantum
→ immutable static role order
→ C++ realtime bridge
→ omniphony_realtime.dll
→ WindowsStaticObjectPipeline
→ existing source-aware Current renderer
→ binaural stereo
```

The composed path is registry-free and remains behind the public gate. It proves source-side wiring in isolation; it does not prove that Windows will enumerate/select the provider or that returned stereo reaches the physical endpoint.

The public provider is therefore still intentionally closed. `IsSpatialAudioStreamAvailable` and `ActivateSpatialAudioStream` continue to return `SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE` because the provider still lacks the proven Windows output/cadence boundary that owns the resulting binaural stereo. The provider must not accept application audio until it can carry that audio all the way to the headphones.

A useful open-source prior exists in `ThreeDeeJay/MSSOAL`, which implements a COM object shaped as `ISpatialAudioClient` plus `ISpatialAudioObjectRenderStream` and `ISpatialAudioObject`. Its registration tool independently identified the same `Spatial\Encoder` surface using Process Monitor observations. MSSOAL describes its provider work as a proof of concept rather than a working product, so it remains a mechanism quarry rather than Windows proof.

Its stream implementation is still useful for one architecture warning: it documents a move away from an independent render-thread clock toward synchronous object-buffer submission because the earlier two-clock design accumulated drift and stutter. A separate SciSpace pass over low-latency audio scheduling and clock-skew work points in the same direction: keep the callback-facing path bounded, decouple heavyweight DSP from the time-critical callback, and avoid independent free-running producer/consumer clocks unless explicit drift compensation exists.

For Omniphony, that makes the intended output law:

> **The physical endpoint render cadence should become the single downstream clock owner, rather than adding another free-running provider render clock.**

This remains an engineering hypothesis until the Windows output path is implemented and measured on the real machine.

Treat MSSOAL as a mechanism quarry, not proof:

- reuse interface shape and lifecycle ideas where correct;
- independently test every Windows boundary;
- do not inherit claims about Windows Sonic, object counts, or rendering internals without primary evidence;
- do not adopt OpenAL Soft as Omniphony's renderer merely because MSSOAL uses it.

Gate:

```text
internal static COM lifecycle exists
≠ static-object realtime ABI reaches Current in isolation
≠ C++ bridge drives that ABI in isolation
≠ COM-shaped update quanta reach Current in isolation
≠ RAW-mode Omniphony APOs are source-level transparent
≠ immutable package generation stages successfully
≠ RAW endpoint capability preflight succeeds
≠ event-driven RAW output lifecycle opens safely
≠ Windows enumerates Omniphony
≠ Windows activates its COM provider
≠ rendered stereo reaches the real endpoint with correct cadence
≠ applications feed objects through the complete path
```

Each transition needs separate evidence.

### P3: static object stream

**Implementation state:** registry-free COM lifecycle, activation marshalling, immutable static-role quantum assembly, object volume/EOS handling, realtime bridge transport, Current worker handoff, and RAW-mode double-render prevention exist in source. Final endpoint output/cadence and real Windows provider proof remain pending.**

The internal COM stream already models:

- static role activation;
- duplicate-role rejection;
- unavailable-role rejection;
- fixed static positions;
- object buffers and update ordering;
- per-object volume;
- implicit end-of-stream semantics;
- role reactivation;
- start / stop / reset lifecycle;
- zero dynamic-object capacity.

The realtime side already provides:

- a fixed immutable stream topology;
- canonical role descriptors and authoritative Windows positions;
- planar object PCM input;
- preallocated callback-facing rings;
- a dedicated Current worker;
- time-aligned safety fold-down on worker starvation;
- stereo binaural output;
- processed-block and latency observability.

The source-side static path now additionally guarantees:

1. one descriptor order is derived from the activation static mask before processing;
2. that topology remains fixed for the stream lifetime;
3. completed COM update quanta are assembled into planar PCM in that exact order;
4. inactive or ended static roles become silence rather than mutating topology;
5. per-object volume and partial end-of-stream semantics are applied before handoff;
6. DLL discovery and renderer construction happen before update processing, not inside the OS-facing update call;
7. once Current has produced binaural stereo, Windows RAW processing mode leaves the Omniphony endpoint and stream APOs transparent rather than invoking Current again.

The next engineering primitive is deliberately inert: an event-driven RAW output sink bound to the exact physical endpoint that can initialize the shared stream, obtain `IAudioRenderClient`, bind an event handle, validate the actual buffer/period contract, and then remain **unstarted**. Only after that lifecycle is stable should the provider use the endpoint event as its downstream cadence and begin moving returned stereo into the render buffer.

All of this must follow the existing Windows realtime law: no filesystem I/O, device discovery, allocation-heavy renderer graph, or research work on the OS-facing update path.

Then receive one real static object through the selected provider and preserve:

- object/static role identity;
- PCM;
- source authority;
- update timing;
- exact role position.

Scale to the complete 17-role vocabulary:

```text
horizontal: FL FR C LFE SL SR BL BR BC
upper:      TFL TFR TBL TBR
lower:      BFL BFR BBL BBR
```

Gate:

> A static source placed above or below the listener reaches Omniphony as authored spatial truth, survives the provider-to-Current transport, and is rendered once by Omniphony to the real headphone endpoint.

### P4: dynamic XYZ object stream

Receive a real dynamic object and preserve:

```text
object identity
audio buffer
x / y / z
volume
lifetime
motion trajectory
other supplied authoritative metadata
```

Do not snap dynamic objects to the 17 static anchors. The static frame and dynamic layer are parallel source representations.

Dynamic capacity remains truthfully zero until this path exists.

Gate:

> A moving object can cross arbitrary 3-D space while its identity and continuous position survive into the Omniphony renderer.

### P5: real spatial application/game

Prove the path with a real application using Windows Spatial Audio.

No game injection, hooks, anti-cheat-sensitive methods, or reconstruction of metadata from already-binaural stereo.

The result must distinguish:

```text
application produced static objects
application produced dynamic objects
Windows selected Omniphony
Omniphony received those objects
object PCM reached Current
Current's binaural stereo reached the real endpoint
Omniphony performed the single final binaural render
```

### Provider installation gate

Spatial-provider deployment must join the existing installer as a transaction rather than as an optimistic registry write.

The staging half of that future transaction now exists as an inert primitive. `Stage-OmniphonySpatialProvider.ps1` creates immutable content-addressed generations beneath the Omniphony install root. Generation identity is derived from the complete sorted package hash set, not only the provider/runtime DLLs.

For a new candidate the staging primitive now:

- requires a 64-bit PowerShell host on 64-bit Windows so Program Files and future registry views cannot silently redirect;
- rejects unsafe source/managed-tree path nesting;
- copies the exact package into a temporary generation directory;
- verifies that the immutable generation contains exactly the expected files and no unexpected subdirectories;
- SHA-256 verifies every copied file;
- runs provider capability, static-stream, and realtime-bridge smokes from the temporary candidate;
- moves the verified candidate into its final immutable generation path;
- re-verifies the exact file set and hashes from the final path;
- re-runs provider capability, static-stream, and realtime-bridge smokes from the final path;
- stages the read-only `OmniphonySpatialRawOutputProbe.exe` for later physical-endpoint preflight;
- atomically writes a `staged-generation.json` manifest with the full package digest, per-file hashes, architecture state, and verification state;
- records explicitly that provider registration and selection were not mutated.

An existing generation is verified rather than modified.

This generation model is intentional:

- never overwrite an in-use COM provider DLL;
- never mutate a generation that has already been verified;
- let upgrades stage beside the current provider;
- preserve the previous generation for rollback while any process may still hold it loaded;
- make repair idempotent by re-verifying an identical generation rather than recopying it.

Required activation order once end-to-end provider transport is ready:

```text
stage immutable generation                         source primitive exists
→ verify exact file set + hashes + final smokes   source primitive exists
→ verify installed APO RAW bypass contract        source smoke exists
→ run read-only RAW endpoint preflight            source primitive exists
→ open event-driven RAW output lifecycle          next source primitive; do not start
→ capture prior provider and selection state
→ switch only Omniphony-owned registration to the new generation
→ verify COM activation and capability contract
→ verify public stream activation and output path
→ enable/select only after end-to-end transport is proven
→ verify ordinary stereo/non-spatial audio still works
→ commit active-generation state
```

Failure and uninstall must restore any provider state Omniphony changed, restore the previous Omniphony generation when an activation fails, unregister only Omniphony-owned keys, retain in-use old generations until they can be retired safely, and leave the physical audio driver untouched.

The installer must never leave Windows selected on a provider that accepts a stream but cannot render it.

---

## Phase 3: Canonical object scene and source authority

**State: semantic foundation implemented; native application-to-provider transport pending**

The canonical scene is a semantic skeleton, not the renderer lattice:

```text
8.1.4.4 static source scene
        +
continuous dynamic objects
        ↓
source authority / provenance
        ↓
Omniphony rendering geometry
```

Required invariants:

- `AUTHORED` always outranks `DERIVED`;
- dynamic XYZ remains continuous;
- object identity is stable across updates;
- LFE remains non-directional unless the source representation explicitly says otherwise;
- absent source roles remain `EMPTY` rather than silently inferred;
- stereo-derived support never contaminates an authored object scene.

---

## Phase 4: Perceptual parity and superiority

**State: begins after object ingress**

Do not retune Current to compensate for spatial information that Windows has not yet delivered.

Once Omniphony receives equivalent source geometry, compare it against established headphone spatial renderers using controlled source scenes.

Primary dimensions:

- left/right accuracy;
- front/back discrimination;
- elevation certainty;
- moving-source continuity;
- externalization;
- radial distance;
- source extent;
- center solidity;
- transient localization;
- envelopment without directional smear;
- timbre, impact, groove, and bass integrity.

The goal is not to imitate another renderer's coloration. It is to determine whether any remaining perceptual deficit comes from the binaural renderer after source geometry has been equalized.

---

## Phase 5: HRTF quality and personalization

**State: research-backed future layer**

Peer-reviewed spatial-hearing work consistently separates horizontal binaural cues from sagittal/elevation spectral cues. Research also shows that listener-specific HRTFs can materially improve localization, especially where pinna-dependent spectral structure matters, while generic HRTFs often retain much of the lateral localization information.

Roadmap order:

1. establish strong generic HRTF behavior;
2. support multiple interchangeable datasets;
3. add objective/perceptual HRTF selection;
4. investigate morphology-assisted personalization;
5. support measured individualized HRTFs where available.

Do not make a personalized HRTF mandatory for Omniphony to work well.

Useful research starting points:

- Baumgartner, Majdak & Laback, *Modeling sound-source localization in sagittal planes for human listeners*, JASA (2014), DOI `10.1121/1.4887447`.
- Romigh & Simpson, *Do you hear where I hear?: isolating the individualized sound localization cues*, Frontiers in Neuroscience (2014), DOI `10.3389/FNINS.2014.00370`.
- Dick & Herre, *Predicting the Precision of Elevation Localization Based on Head Related Transfer Functions*, ICASSP (2019), DOI `10.1109/ICASSP.2019.8682313`.
- Planinec et al., *The Accuracy of Dynamic Sound Source Localization and Recognition Ability of Individual Head-Related Transfer Functions in Binaural Audio Systems with Head Tracking*, Applied Sciences (2023), DOI `10.3390/app13095254`.

---

## Phase 6: Externalization, distance, and room

**State: research-backed future layer**

Externalization is not merely stronger reverb. Research points to interactions among binaural coherence, early reflections/room information, listener HRTF, source direction, and motion.

Priorities:

- preserve direct-source localization before adding room support;
- test early-reflection and interaural-coherence effects independently;
- model near-field distance separately from far-field externalization;
- keep source extent distinct from room size;
- prevent room support from smearing transient directionality.

Useful research starting points:

- Leclère, Lavandier & Perrin, *On the externalization of sound sources with headphones without reference to a real source*, JASA (2019), DOI `10.1121/1.5128325`.
- Landschoot & Jot, *Binaural externalization processing method for object-based audio rendering*, JASA (2023), DOI `10.1121/10.0018389`.

---

## Phase 7: Optional head tracking

**State: optional future capability, not a prerequisite**

Peer-reviewed work supports head tracking as a strong externalization/localization cue. Experienced multichannel listeners also report that head tracking can improve the illusion of a fixed acoustic scene, while some dislike it for music or mobile listening.

Therefore head tracking should be:

- optional;
- low-latency;
- renderer-level rather than source-destructive;
- useful for games, XR, film, and stationary virtual-room listening;
- disableable for listeners/content where a head-locked presentation is preferred.

Research examples include dynamic-binaural and head-tracking studies by Mehra et al., Fallahi et al., and later localization work.

Practitioner evidence from QuadraphonicQuad is useful here because it exposes context dependence that a single lab task can miss.

---

## Phase 8: Already-binaural detection and double-render prevention

**State: required before automatic mixed-source policy; provider RAW egress now has a separate explicit bypass contract**

A two-channel signal is not automatically ordinary stereo. If another spatial renderer has already produced binaural output, Omniphony must not apply a second HRTF field blindly.

Required future policy:

```text
ordinary stereo
→ Current inference/enhancement

known already-binaural spatial stereo
→ spatial bypass
or explicitly validated non-spatial correction only
```

Automated switching requires a trustworthy signal. Channel count alone is insufficient.

The provider path is not heuristic: when Omniphony itself has just produced the final binaural stereo, RAW processing mode is an explicit provenance-bearing egress contract and both Omniphony APOs are required to remain transparent.

---

## Phase 9: Windows product hardening

Spatial-provider installation safety begins during Phase 2 rather than waiting until the end. Full product hardening follows once spatial-object ingress works:

- endpoint hotplug and DAC power cycling;
- device/default-output changes;
- stream restart and application relaunch behavior;
- sample-rate/format compatibility;
- underrun and latency hardening;
- object-capacity changes;
- static/dynamic object lifecycle abuse tests;
- RAW-mode APO bypass regression and bit-transparency checks;
- application compatibility matrix;
- clean coexistence with non-spatial applications;
- safe provider selection, rollback, upgrade, repair, and uninstall;
- immutable content-addressed provider generations;
- locked-file / in-use COM binary retirement without in-place replacement;
- stale provider-key detection without touching unrelated providers;
- active/staged/previous generation manifests with exact hashes;
- upgrade from conventional-APO-only installs without reinstalling the physical driver;
- signed deployment research where useful.

The product experience should stay small:

```text
install Omniphony
→ choose it as the spatial renderer when applicable
→ Windows audio uses it
```

The tray/UI remains configuration only; it must never become the audio host.

---

## Phase 10: Portable open spatial renderer

After the Windows provider is mature, expose a stable portable scene API so other hosts can supply the same semantics directly.

Possible future hosts:

- Linux audio systems;
- macOS;
- game engines;
- XR stacks;
- media players;
- DAWs;
- research tools.

The Windows object API is one ingress adapter, not the definition of Omniphony's renderer.

---

## Research-derived renderer priorities

The research and practitioner record suggests this order after native object ingress:

```text
1. source geometry and identity
2. ITD / ILD horizontal localization
3. pinna/spectral elevation + front/back cues
4. motion continuity
5. externalization / early reflections / coherence
6. distance and near-field behavior
7. HRTF personalization
8. optional head tracking
```

This ordering prevents a common failure mode: using room effects or synthetic width to hide weak localization.

QuadraphonicQuad discussions also reinforce two practical requirements:

- binaural playback must be judged independently from a speaker-array mix because different renderers can diverge substantially;
- head tracking and personalization are valuable tools but should remain optional and content/context aware.

---

## Critical path

The shortest route from the accepted Windows baseline to the full product is now:

```text
stereo + 7.1 Windows baseline                    ✅
        ↓
retain conventional 7.1.4 compatibility
        ↓
provider capability object                       source implemented
        ↓
static stream lifecycle + activation marshalling source implemented
        ↓
static object → Current realtime worker          source implemented
        ↓
C++ realtime DLL bridge                          source implemented
        ↓
COM-shaped quantum → bridge → Current            source implemented
        ↓
RAW-mode Omniphony APO single-render bypass      source implemented
        ↓
immutable provider generation staging            source implemented
        ↓
RAW physical-output capability preflight         source implemented
        ↓
event-driven RAW output sink lifecycle           next: initialize, bind event, do not start
        ↓
prove Omniphony Spatial Sound provider enumeration
        ↓
prove provider COM activation on real Windows
        ↓
make physical endpoint cadence the output clock owner
        ↓
prove rendered stereo reaches the real endpoint once
        ↓
open public stream activation gate
        ↓
receive one static Windows Spatial Audio object
        ↓
receive the full 17-role static vocabulary
        ↓
receive one moving dynamic XYZ object
        ↓
receive a real application's spatial scene
        ↓
render that scene once through Omniphony
        ↓
controlled A/B against established spatial renderers
        ↓
localization / HRTF / externalization refinement
        ↓
personalization + optional head tracking
        ↓
product hardening and public release
```

The next decisive source milestone is now intentionally smaller than end-to-end activation:

> **Open the exact physical endpoint as an event-driven RAW stereo sink, prove its negotiated period/buffer/render-client contract, keep it unstarted, and make no provider-selection mutation.**

The next decisive end-to-end milestone remains:

> **A source is authored above the listener, reaches Omniphony as an actual Windows spatial object rather than an inference, crosses the existing static-object Current worker path, and Omniphony alone delivers the final headphone render to the real endpoint.**

---

## Evidence sources

### Platform contract

- Microsoft Spatial Sound overview: https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound
- Microsoft spatial-object rendering: https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects
- `ISpatialAudioClient`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient
- `ISpatialAudioClient::IsSpatialAudioStreamAvailable`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioclient-isspatialaudiostreamavailable
- `ISpatialAudioClient::ActivateSpatialAudioStream`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nf-spatialaudioclient-ispatialaudioclient-activatespatialaudiostream
- `SpatialAudioObjectRenderStreamActivationParams`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ns-spatialaudioclient-spatialaudioobjectrenderstreamactivationparams
- `AudioObjectType`: https://learn.microsoft.com/windows/win32/api/spatialaudioclient/ne-spatialaudioclient-audioobjecttype
- Microsoft Windows Audio Session WASAPI renderer sample: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/WindowsAudioSession
- Microsoft SysVAD SwapAPO sample, including RAW-mode SFX bypass: https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO/SwapAPO

### Realtime scheduling / clocking research quarry

- *ANIRA: An Architecture for Neural Network Inference in Real-Time Audio Applications* (2024/2025): decouples heavyweight inference from the audio callback and manages bounded latency.
- Yonghao Wang, *Low Latency Audio Processing* (2018 dissertation): analyzes buffering and OS scheduling as core latency/predictability constraints.
- Stefan Werner, *An algorithm for audio skew compensation in low latency environments* (ICMC 2005): demonstrates that independently clocked audio paths require explicit skew compensation.

These sources support bounded callback work and single-clock design pressure. They do not by themselves prove the undocumented Windows provider/output seam.

### Open-source implementation quarry

- MSSOAL / experimental OpenAL Spatial Audio provider: https://github.com/ThreeDeeJay/MSSOAL
- OpenAL Soft: https://github.com/kcat/openal-soft
- Microsoft Windows audio samples: https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound
- Microsoft Xbox ATG Advanced Spatial Sounds: https://github.com/microsoft/Xbox-ATG-Samples/tree/main/UWPSamples/Audio/AdvancedSpatialSoundsUWP

### Practitioner listening / production evidence

- QuadraphonicQuad Object-based Surround forum: https://quadraphonicquad.com/forums/object-based-surround.178/
- Headphone spatial-listening discussion: https://quadraphonicquad.com/threads/question-about-surround-spatial-listening-using-headphones.38489/
- Spatial Audio and head tracking discussion: https://quadraphonicquad.com/threads/spatial-audio-and-head-tracking.32111/
- Binaural monitoring/head-tracking discussion: https://quadraphonicquad.com/threads/binaural-monitoring-of-surround-mixes-with-head-tracking-pick-your-budget.37048/

Forum reports are experiential evidence, not substitutes for the Windows API contract or controlled psychoacoustic studies.
