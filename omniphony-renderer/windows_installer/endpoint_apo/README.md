# Omniphony native endpoint APO Current candidate

This package is the first **audible** native Windows endpoint-APO build of the retained Omniphony Current model.

## What this build is

`OmniphonyAPO.dll` is a real Windows Audio Processing Object hosted by the Windows audio engine on the physical render endpoint. It does not create or require an Omniphony playback device.

The processing boundary is:

```text
Windows audio engine / audiodg
        ↓
OmniphonyAPO.dll
        ↓ cached realtime ABI call
omniphony_realtime.dll
        ↓
bounded delayed-dry safety lane
+ dedicated Current worker
        ↓
protected master + foundation + spatial support
        ↓
primary Noire X personal output correction
        ↓
retained final linked peak guard
        ↓
physical FiiO / Noire X endpoint
```

Mode 0 remains exact identity inside the portable realtime ABI as a deterministic transport oracle. The endpoint APO now selects **mode 1 / Current** for supported stereo float32 graphs.

## Personal output correction

The Current renderer's public foundation EQ and the listener's headphone correction are separate layers.

This personal development package includes the primary Noire X correction profile after the Omniphony spatial/master sum and before the final peak guard:

```text
shared preamp     -4.0 dB
15 Hz high-pass   Q 0.6
45 Hz low shelf   +3.5 dB Q 0.5
30 Hz peak        +1.2 dB Q 0.8
85 Hz peak        +2.0 dB Q 0.65
155 Hz peak       +1.3 dB Q 0.75
240 Hz peak       -0.2 dB Q 0.9
420 Hz peak       +0.8 dB Q 0.7
700 Hz peak       +0.8 dB Q 0.8
1.2 kHz peak      +0.9 dB Q 0.7
1.9 kHz peak      +0.5 dB Q 0.8
2.8 kHz peak      -0.6 dB Q 0.6
3.8 kHz peak      -2.2 dB Q 0.9
4.8 kHz peak      -2.6 dB Q 1.1
6.2 kHz peak      -0.9 dB Q 1.3
7.2 kHz high shelf -1.8 dB Q 0.7

right only
preamp            -0.4 dB
180 Hz peak        -0.3 dB Q 0.9
3.0 kHz peak       -1.1 dB Q 1.0
6.2 kHz high shelf -0.3 dB Q 0.7
delay               0.02 ms
```

The biquad implementation independently matches the RBJ/Q/corner-frequency semantics used by the former Equalizer APO profile. `0.02 ms` rounds to one right-channel sample at 48 kHz, matching Equalizer APO's integer-sample delay behavior. Equalizer APO itself is not a dependency.

This profile is a listener-specific layer and **not** the public/default Omniphony tuning.

## Fixed-latency safety lane

Current rendering remains off the Windows realtime callback. The callback only performs bounded copies into/out of preallocated rings.

The endpoint build reports a fixed **40 ms** host delay. The same delay is continuously maintained for a dry safety lane. During startup, output is silent until that timeline is primed. After priming:

- normal case: aligned Current PCM is emitted;
- transient worker underrun: the matching delayed dry frame is emitted instead;
- late Current frames corresponding to missed deadlines are discarded before Current resumes;
- worker/ring failure: the APO remains on the aligned delayed-dry path instead of jumping to immediate dry or stale PCM.

This is intentionally conservative for the first physical Current test. Latency can be reduced only after hardware timing measurements show a smaller budget is stable.

## Safety boundary

- no virtual playback endpoint is installed by this package;
- no application hooking or injection;
- no Secure Boot, test-signing, BitLocker, or boot-policy changes;
- no `DisableProtectedAudioDG` change;
- the installer refuses to overwrite a different existing EFX APO;
- detach removes the EFX value only when it belongs to Omniphony;
- DLL loading, ABI resolution, Current-worker construction and teardown occur outside `APOProcess`;
- `APOProcess` does not allocate, log, sleep, access files, launch processes, discover devices, or touch the registry;
- identity mode remains bit-exact inside the ABI for transport regression tests;
- final Current peak safety remains after the personal EQ.

Stable APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

## Install / update on the test machine

Extract the **whole artifact** into one directory and keep that directory in place during this test. Open **PowerShell as Administrator** there and run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\Install-OmniphonyAPO.ps1
```

Before touching the endpoint, the installer runs two local fail-fast checks:

1. `OmniphonyRealtimeSmoke.exe` loads `omniphony_realtime.dll`, verifies ABI 0.3, exact identity mode, Current initialization, and the 40 ms latency contract.
2. `OmniphonyApoSmoke.exe` activates the COM APO and exercises Current-mode configure/process/unlock behavior before endpoint association.

Only after those pass does the script attach the APO to the physical output, restore the physical FiiO / Noire endpoint as Windows default, restart Windows Audio, and verify that association survived the restart.

Then play normal stereo audio through `Dan Clark Noire X (FiiO Q series)`. You should **not** select an `Omniphony` playback endpoint.

For diagnostics:

```powershell
.\OmniphonyApoCtl.exe list
.\OmniphonyApoCtl.exe status
```

Expected association:

```text
EFX     {A9333BFE-39C1-40FD-B4B0-ECC591410B47}
ENHANCEMENTS_DISABLED   0
```

If installation reports `EXISTING_EFX`, stop and preserve the output. Omniphony deliberately refuses to replace another endpoint effect.

If installation reports `FX_WRITE` with access denied, preserve that output too. Do not weaken Windows security globally.

## Remove

From elevated PowerShell:

```powershell
.\Uninstall-OmniphonyAPO.ps1
```

## Evidence states

Keep these separate:

```text
APO builds
≠ realtime ABI Current tests pass
≠ COM registration/activation succeeds
≠ physical endpoint accepts the EFX association
≠ audiodg loads it for real playback
≠ Current PCM is stable on the physical machine
≠ Current + personal EQ is preferred in physical listening
```

The endpoint association has already been physically proven on the primary FiiO/Noire endpoint. This package promotes the **audible Current renderer** to the next physical listening gate. Listening success is not claimed until the listener reports it.
