Omniphony personal Windows bootstrap
====================================

Purpose
-------

This is a temporary personal bring-up host for hearing the retained Omniphony
Current model inside the real Windows render graph before the project ships its
own endpoint-effect APO. It uses an existing Equalizer APO installation only as
the host for a tiny VST bridge. It creates no virtual cable, virtual playback
device, loopback transport, or second renderer.

Audible path
------------

    Windows application audio
        -> existing Equalizer APO on the physical FiiO endpoint
        -> OmniphonyVstBridge.dll
        -> omniphony_realtime.dll ABI 0.2
        -> dedicated Current-model worker thread
        -> protected master + coherent foundation + full-sphere support
        -> measured-HRTF early field + retained transient law
        -> +2.8 dB fixed makeup / stereo-linked peak safety
        -> stereo FiiO / Noire X

The VST callback only moves PCM through preallocated stereo rings. The retained
renderer remains on a dedicated worker thread because the existing engine still
allocates internally. This is a bounded personal bootstrap, not the final APO
realtime architecture.

Identity mode remains in the Rust ABI as a regression oracle, and explicit VST
bypass remains bit-exact passthrough. If the Current worker faults, the C++ host
falls back to direct PCM rather than calling broken renderer state.

Install
-------

1. Extract the complete CI artifact to one directory.
2. Open PowerShell as Administrator in that directory.
3. Run:

       .\install-personal-bootstrap.ps1

The script finds Equalizer APO, copies the two DLLs under Program Files, backs
up Equalizer APO's config.txt, creates a separate Omniphony include file, and
adds one marked Include line to config.txt.

If the FiiO endpoint is already enabled in Equalizer APO because of an existing
HeSuVi setup, leave the endpoint association alone. If it is not enabled, use
Equalizer APO Configurator to enable that real render endpoint. The bootstrap
does not edit endpoint FxProperties itself.

Expected first-listen behavior
------------------------------

The first audible build is intentionally stereo-source first. It should sound
like the repository's retained Current model, not like generic 7.1 conversion.
Do not enable HeSuVi virtualization in parallel with Omniphony. Equalizer APO is
only the temporary host here; a second HRTF/virtual-room stage would invalidate
the listening comparison.

Rollback
--------

Run as Administrator:

    .\uninstall-personal-bootstrap.ps1

The uninstall script removes only the marked Omniphony include, its snippet,
and the Omniphony bootstrap DLL directory. It does not uninstall Equalizer APO
or rewrite unrelated Equalizer APO configuration.

Architecture quarantine
-----------------------

Equalizer APO is a personal bootstrap host, not the mature Omniphony product
boundary. The product contract remains: physical Windows endpoint, Omniphony
native endpoint effect / supported richer spatial ingress, one portable scene
and renderer, no virtual cable.
