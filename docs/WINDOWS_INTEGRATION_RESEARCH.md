# Windows integration research

This is a parked technical room for **Omniphony for Headphones**. `README.md` remains the product authority and `docs/INFLUENCE_LEDGER.md` remains the broad external-research index.

The purpose of this file is to preserve Windows-specific integration findings without allowing driver/APO research to delay the first audible prototype.

## Sources reviewed

Primary Microsoft sources:

- https://github.com/MicrosoftDocs/win32/tree/docs/desktop-src/CoreAudio
- https://github.com/MicrosoftDocs/win32/blob/docs/desktop-src/CoreAudio/spatial-sound.md
- https://github.com/MicrosoftDocs/sdk-api/blob/docs/sdk-api-src/content/spatialaudioclient/nn-spatialaudioclient-ispatialaudioobject.md

Practical third-party endpoint/driver reference:

- https://github.com/shibajee/realtek-uad-dts-mod

Related implementation references are retained in `docs/INFLUENCE_LEDGER.md`, especially CamillaDSP and `wasapi-rs`.

---

## Microsoft Spatial Sound: what the public API actually provides

Microsoft Spatial Sound is a platform-level spatial-audio system. Apps can feed spatial content to Windows through `ISpatialAudioClient`.

The public application-facing model supports:

- static audio objects assigned to predefined spatial channels;
- a static channel bed up to 8.1.4.4;
- dynamic audio objects with arbitrary 3-D positions;
- object positions updated over time;
- Windows system mixing with non-spatial applications;
- output-format abstraction: the application can submit spatial intent without implementing separate paths for Windows Sonic, Dolby Atmos for Headphones, DTS Headphone:X, speakers, or home theater.

`ISpatialAudioObject` represents either:

- a static object tied to one of the defined channel locations; or
- a dynamic object that can be positioned and moved in 3-D space.

### Important distinction for Omniphony

`ISpatialAudioClient` is primarily an **input/sink API into the spatial renderer Windows has selected**.

Conceptually:

```text
GAME / APP
→ static bed and/or dynamic audio objects
→ ISpatialAudioClient
→ user-selected Windows spatial renderer
   (Sonic / Atmos / DTS / etc.)
→ endpoint
```

This is useful for understanding spatial source semantics and for future interoperability, but it is not evidence of a simple public API that lets an ordinary application register its own renderer as another entry in Windows' Spatial Sound dropdown.

Therefore do not conflate:

```text
supporting Windows spatial objects as an input
```

with:

```text
becoming the Windows system spatial renderer
```

They are separate engineering problems.

---

## Useful Windows spatial semantics for future input support

Microsoft documents three common renderer integration styles:

1. **Static spatial channel bed** such as 7.1.4.
2. **Existing direct/stereo endpoint plus a spatial bus**, when some content should remain direct-to-ears while other content is spatialized.
3. **Dynamic objects for selected voices/submixes**, with prioritization needed when object budgets are finite.

This is valuable for Omniphony because its mature input model should eventually distinguish:

```text
stereo music
channel beds
height beds
true dynamic objects
```

without flattening them into one generic multichannel stream.

The Microsoft guidance also reinforces a useful separation of responsibilities. Spatial Sound concentrates on positioning on an idealized sphere, while engines may still own other cues such as:

- distance attenuation/filtering;
- Doppler;
- occlusion/obstruction;
- environmental reverberation.

That agrees with the current Omniphony architecture: HRTF positioning, room cues, distance cues, source extent, and scene inference should remain distinguishable mechanisms.

---

## System-renderer UX remains a product target, not a P0 assumption

The ideal consumer experience remains approximately:

```text
install once
→ choose/detect headphones
→ enable Omniphony
→ all ordinary audio uses it
```

A literal Windows Spatial Sound dropdown entry would be excellent if a supported third-party registration path is eventually available to us.

Until such a path is demonstrated from official documentation or a supported SDK, the product must not depend on that assumption.

Candidate system-wide routes remain:

- endpoint/system-effect APO;
- virtual render endpoint plus background processing host;
- another supported driver/component integration discovered later;
- optional ASIO specialist route.

The UX requirement is stable even if the underlying mechanism changes.

---

## Driver/APO reality from the Realtek + DTS mod ecosystem

The `realtek-uad-dts-mod` project is not an implementation source for Omniphony. It is useful operational evidence.

Its installation flow demonstrates that seamless commercial endpoint enhancement can involve a tightly coupled bundle of:

- audio driver package;
- Windows driver-signing policy;
- endpoint-associated processing components;
- companion UWP applications;
- vendor-specific DTS processing packages;
- reboot/install lifecycle.

The project instructs users to disable driver-signature enforcement for its modified package. **Omniphony for Headphones must not normalize that as acceptable consumer installation.** A public release should use supported Windows deployment/signing paths.

Durable lesson:

```text
seamless user experience
≠ simple implementation
```

The product may eventually hide substantial endpoint/APO/driver plumbing behind one enable switch, but P0 should not absorb that deployment complexity.

---

## Immediate versus parked

### Immediate / P0

No architecture change from this research.

Continue:

```text
upstream reference scene
→ protected Omniphony binaural renderer
→ stereo PCM
→ native WASAPI playback
```

P0 exists to prove the real renderer can be heard through the native Windows path.

### P1 transport hardening

Already-supported candidate from the HEnquist/CamillaDSP pass:

- move from CPAL convenience playback toward direct event-driven `wasapi-rs` if it buys better format control, recovery, device notifications, and deterministic buffering.

### Later system integration

Research only after the audible renderer path works:

- supported APO architecture and registration;
- endpoint effect packaging;
- virtual endpoint feasibility;
- device association;
- code signing / installer requirements;
- coexistence with existing Windows Spatial Sound selections;
- default-device changes and sleep/resume recovery.

### Later game/app spatial input

Potentially support:

- 5.1 / 7.1 channel beds;
- 7.1.4 or richer height beds;
- Windows spatial-object semantics where they can be captured/bridged legitimately;
- direct object metadata from formats/engines that expose it.

Do not double-virtualize content that Windows, a game, or another renderer has already collapsed to binaural stereo.

---

## Promotion gate

Nothing in this document enters the default product merely because Windows or a commercial vendor uses it.

Promote only when it solves a demonstrated need while preserving:

- protected upstream Omniphony sound;
- low-friction installation;
- reliable ordinary Windows playback;
- single-path output with no dry+processed duplication;
- clean uninstall/recovery;
- future publishability.
