Omniphony personal Windows bootstrap
====================================

Purpose
-------

This is a temporary personal bring-up host for proving Omniphony inside the
real Windows render graph before the project ships its own endpoint-effect APO.
It uses an existing Equalizer APO installation only as the host for a tiny VST
bridge. It does not create a virtual cable, virtual playback device, loopback
transport, or second renderer.

Stage 0 is deliberately exact identity:

    Windows application audio
        -> existing Equalizer APO on the physical FiiO endpoint
        -> OmniphonyVstBridge.dll
        -> omniphony_realtime.dll ABI 0.1
        -> bit-exact stereo
        -> FiiO / headphones

A successful Stage 0 proves the native insertion/callback boundary without
changing what you hear. The next stage attaches the retained Current-model
Omniphony renderer behind this same realtime seam. After personal listening is
stable, the temporary Equalizer APO host is replaced by Omniphony's own native
Windows APO while keeping the portable renderer boundary.

Safety properties
-----------------

- x64, stereo 2-in / 2-out bootstrap only.
- The bridge dynamically loads a sibling omniphony_realtime.dll and requires
  the expected ABI before declaring itself ready.
- Audio buffers are allocated on VST lifecycle/block-size callbacks, not in the
  realtime process callback.
- Missing backend, invalid state, oversized blocks, bypass, or process errors
  fall back to direct passthrough.
- The callback performs no filesystem I/O, device discovery, network access,
  subprocess launch, or unbounded locking.
- The current Rust ABI is bit-exact identity. Any audible change at this stage
  is a failure signal, not an expected effect.

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
Equalizer APO Configurator to enable that real render endpoint. This bootstrap
does not edit endpoint FxProperties itself.

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
