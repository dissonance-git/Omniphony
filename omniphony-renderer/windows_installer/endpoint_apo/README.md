# Omniphony native endpoint APO bootstrap

This package proves the Windows integration layer before any Omniphony DSP is moved into it.

## What this build is

`OmniphonyAPO.dll` is a real Windows Audio Processing Object hosted by the Windows audio engine. In this first bootstrap it is intentionally **identity-only**: valid PCM is copied bit-for-bit, silent buffers remain silent, and the APO reports zero added latency.

The test therefore answers one question only:

> Can Windows load an Omniphony APO on the physical FiiO / Dan Clark playback endpoint without creating another playback device?

A successful test is expected to sound unchanged. Audible Omniphony processing comes only after physical-endpoint attachment is proven.

## Safety boundary

- no virtual playback endpoint is installed;
- no application hooking or injection;
- no Secure Boot, test-signing, BitLocker, or boot-policy changes;
- no `DisableProtectedAudioDG` change;
- the installer refuses to overwrite a different existing EFX APO;
- detach removes the EFX value only when it belongs to Omniphony;
- if the EFX processing-mode value did not exist before attachment, Omniphony removes only the value it created;
- the APO realtime callback performs only bounded memory copy/zero work and does not allocate, log, sleep, access files, or touch the registry.

Stable APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

## Install on the test machine

Extract the whole artifact into one directory. Open **PowerShell as Administrator** in that directory and run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\Install-OmniphonyAPO.ps1
```

The script stops the currently running old Omniphony build, registers the APO, attaches it to the physical output, restores the physical FiiO / Noire endpoint as the Windows default, restarts Windows Audio, and verifies the attachment survived the restart.

Then check Windows Sound. The physical FiiO / Dan Clark endpoint should be the default playback device. You should **not** need to select an `Omniphony` playback endpoint for this test.

Normal audio should still play and should sound unchanged because the APO is identity-only.

For diagnostics:

```powershell
.\OmniphonyApoCtl.exe list
.\OmniphonyApoCtl.exe status
```

If installation reports `EXISTING_EFX`, stop and preserve the output. Omniphony deliberately refuses to replace another endpoint effect.

If installation reports `FX_WRITE` with access denied, preserve that output too. Some endpoint registry keys use tighter ACLs; the next bounded repair is an Omniphony-owned installer step for writable FX properties, not a reason to weaken Windows security globally.

## Remove

From elevated PowerShell:

```powershell
.\Uninstall-OmniphonyAPO.ps1
```

## Evidence states

Keep these separate:

```text
APO builds
≠ COM registration/activation succeeds
≠ physical endpoint accepts the EFX association
≠ audiodg loads it for real playback
≠ identity audio is stable on the physical machine
≠ Current Omniphony DSP is ready for the APO
```

The bootstrap is promoted only after the physical-machine attachment result is known.
