# Windows development transport

This document records a **temporary private-development transport**, not the target Omniphony for Windows endpoint architecture.

## Why it exists

The custom Omniphony virtual render endpoint is currently a Windows 11 WDK development/test-signed kernel driver. Stock Windows 11 with normal code-integrity / Secure Boot policy can reject that driver even when its development certificate is imported correctly.

The product must not silently weaken Secure Boot, BitLocker, driver-signature enforcement, boot integrity, or other Windows security policy merely to make a private build load.

The permanent product solution remains a Microsoft-signed Omniphony endpoint.

## Private development fallback

When Steam is installed and its official local Steam Streaming Speakers package is available, the private installer may use that already-signed endpoint as a temporary silent render sink:

```text
Windows apps / games / players
        ↓
Steam Streaming Speakers
signed temporary transport only
        ↓
endpoint-independent Windows process-loopback capture
        ↓
Omniphony renderer / personal profile
        ↓
stereo binaural output
        ↓
Dan Clark Noire X / FiiO K7
```

This restores the single-stream topology required for daily listening without Hi-Fi Cable, ASIO Bridge, HeSuVi, test mode, or disabling Windows security.

## Ownership rules

Omniphony does **not** own the Valve driver.

The installer therefore must:

- use an already-installed Steam Streaming Speakers endpoint when present;
- otherwise use only the official Steam-local driver package if it exists under the local Steam installation;
- install that package through the normal Windows driver-install API, following the same basic approach used by mature software such as Sunshine;
- never download, redistribute, rename, patch, re-sign, or otherwise modify Valve driver files;
- never remove the Valve driver during Omniphony uninstall;
- keep all Omniphony rendering, profile, lifecycle, recovery, and physical-output behavior in Omniphony itself;
- prefer the real Omniphony endpoint automatically once a production-trusted package is available.

## Product boundary remains unchanged

The temporary transport does not alter the architecture in `docs/omniphony-for-windows.md`.

The target remains:

```text
Windows
  ↓
Omniphony virtual render endpoint
  ↓
Windows host adapter
  ↓
portable Omniphony engine
  ↓
selected profile
  ↓
physical stereo headphone endpoint
```

Steam Streaming Speakers is a disposable bridge for private development only.

## Useful side effect

Steam Streaming Speakers already supports conventional stereo, 5.1, and 7.1 Windows speaker configurations. That makes it useful for validating source-layout preservation while the native Omniphony endpoint is still being productionized. It does not replace the need for Omniphony's own endpoint, richer channel-layout ABI, or future Windows spatial/object integration.
