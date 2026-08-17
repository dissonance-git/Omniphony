# Omniphony native endpoint APO bootstrap

This package proves the Windows integration layer before the Current Omniphony model is allowed to become audible inside it.

## What this build is

`OmniphonyAPO.dll` is a real Windows Audio Processing Object hosted by the Windows audio engine. In this bootstrap it remains intentionally **identity-only**: valid float PCM is copied bit-for-bit, silent buffers remain silent, and the APO reports zero added latency.

The identity path is now split across the same boundary intended for the finished renderer:

```text
Windows audio engine / audiodg
        ↓
OmniphonyAPO.dll
        ↓ cached realtime ABI call
omniphony_realtime.dll
        ↓
identity today / Current worker later
```

The Rust realtime DLL is loaded, ABI-checked and instantiated during `LockForProcess`, never from `APOProcess`. The realtime callback uses only cached state and bounded PCM operations. If the Rust identity call becomes unavailable, the APO falls back to native in-process identity rather than dropping or corrupting the buffer.

The Current model is already present behind realtime ABI mode 1 and runs on a dedicated worker with preallocated SPSC rings. It is **not enabled by this endpoint package yet**. Keeping the first physical test identity-only separates Windows attachment failures from DSP/latency failures.

The test therefore still answers one question only:

> Can Windows load the Omniphony APO on the physical FiiO / Dan Clark playback endpoint, keep the normal physical endpoint as default, and carry real playback through the native APO path without creating another playback device?

A successful first hardware test is expected to sound unchanged.

## Safety boundary

- no virtual playback endpoint is installed by this package;
- no application hooking or injection;
- no Secure Boot, test-signing, BitLocker, or boot-policy changes;
- no `DisableProtectedAudioDG` change;
- the installer refuses to overwrite a different existing EFX APO;
- detach removes the EFX value only when it belongs to Omniphony;
- if the EFX processing-mode value did not exist before attachment, Omniphony removes only the value it created;
- DLL loading, ABI resolution, processor creation and teardown occur outside `APOProcess`;
- the identity realtime callback does not allocate, log, sleep, access files, or touch the registry;
- Current stays disabled until its worker latency/fallback contract is ready and the identity APO has been physically proven.

Stable APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

## Install on the test machine

Extract the **whole artifact** into one directory. Open **PowerShell as Administrator** in that directory and run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\Install-OmniphonyAPO.ps1
```

Before touching the endpoint, the installer now runs two local fail-fast checks:

1. `OmniphonyRealtimeSmoke.exe` dynamically loads `omniphony_realtime.dll`, checks ABI compatibility, and verifies exact identity PCM.
2. After COM registration, `OmniphonyApoSmoke.exe` activates the APO and exercises its normal configure/process lifecycle before endpoint association.

Only after those pass does the script stop the old Omniphony build, attach the APO to the physical output, restore the physical FiiO / Noire endpoint as the Windows default, restart Windows Audio, and verify the attachment survived the restart.

Then check Windows Sound. The physical FiiO / Dan Clark endpoint should be the default playback device. You should **not** need to select an `Omniphony` playback endpoint for this test.

Normal audio should still play and should sound unchanged because the APO is identity-only.

For diagnostics:

```powershell
.\OmniphonyApoCtl.exe list
.\OmniphonyApoCtl.exe status
```

If installation reports `EXISTING_EFX`, stop and preserve the output. Omniphony deliberately refuses to replace another endpoint effect.

If installation reports `FX_WRITE` with access denied, preserve that output too. Some endpoint registry keys use tighter ACLs; the bounded repair is an Omniphony-owned installer step for writable FX properties, not a reason to weaken Windows security globally.

## Remove

From elevated PowerShell:

```powershell
.\Uninstall-OmniphonyAPO.ps1
```

## Evidence states

Keep these separate:

```text
APO builds
≠ realtime ABI identity self-test succeeds
≠ COM registration/activation succeeds
≠ LockForProcess → APOProcess → UnlockForProcess succeeds
≠ physical endpoint accepts the EFX association
≠ audiodg loads it for real playback
≠ identity audio is stable on the physical machine
≠ Current Omniphony DSP is ready to become audible in the APO
```

The bootstrap is promoted only after the physical-machine attachment result is known.
