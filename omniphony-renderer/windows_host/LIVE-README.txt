Omniphony for Headphones - Windows prototype

NORMAL USE

1. Extract this entire folder to a stable location. Do not run it permanently from a temporary ZIP/extraction folder because Windows autostart points to the exact Omniphony.exe path.
2. Keep normal Windows / foobar playback routed to the virtual sink you already use (currently Hi-Fi Cable) so the unprocessed signal does not also play directly from the physical headphones.
3. Disable HeSuVi / DTS Virtual:X and stop ASIO Bridge or any other old route that independently forwards the virtual sink to the FiiO.
4. During development, double-click START-OMNIPHONY.cmd. It simply launches Omniphony.exe from the extracted folder.

Omniphony.exe opens no normal window. It lives in the Windows notification area and registers itself for per-user Windows autostart on first run. After that, normal use should require no manual launch after login.

There is only one shipped Omniphony runtime executable. The normal Omniphony.exe instance is the invisible supervisor. For crash isolation it launches a second instance of the same Omniphony.exe in a private internal audio-engine mode. This preserves watchdog/recovery behavior without shipping a separate worker executable.

NORMAL PLAYBACK MODEL

There is one normal listening model: Current model.

The former research profile selector has been retired. The measured-HRTF early-reflection mechanism previously tested under the label Externalization is now part of Current model. Old profile environment variables or preferences are not used by the normal supervisor path.

TRAY CONTROLS

Left- or right-click the Omniphony tray icon:

  Omniphony: ON / clean bypass
  Turn Omniphony on/off
  Restart audio engine
  Start with Windows
  Exit Omniphony

The supervisor is single-instance. If the internal engine exits or cannot start because the output device is temporarily unavailable, the supervisor retries automatically. If WASAPI reports that the physical playback stream has failed, the engine exits cleanly and the supervisor relaunches it. Closing/restarting Windows Explorer also recreates the tray icon.

AUDIO ROUTING

The internal engine captures Windows process audio while excluding its own output, then sends the processed stereo result directly to the physical WASAPI endpoint. It prefers a device containing "FiiO" and otherwise uses a non-virtual Windows default output.

This removes the old requirement to manually start an ASIO bridge after reboot. Hi-Fi Cable is still useful as the silent source/sink boundary until Omniphony owns a native virtual endpoint or APO host.

ON:

  authoritative captured stereo master
  + coherent music foundation/body delta
  + frequency-derived 7.1.4 support
  -> inherited Omniphony cascaded speaker-world renderer
  -> measured SAF/KEMAR HRTF / ITD / metric distance / air cues
  -> six directional first-order reflection buses through measured HRTF
  -> short restrained late room field
  -> fixed linear final summing
  -> physical headphones

OFF:

  clean captured stereo reference through the same fixed output headroom

The finished stereo master never becomes a full-wet virtual-room replacement. Bass/body remains direct-dominant. Full spatial support starts above 320 Hz. Current upper support is deliberately restrained to reduce fatigue while height and scale come from geometry rather than treble gain.

The current coherent foundation includes stronger narrow kick support around 110 Hz. Dense-guitar fatigue correction remains confined to the additive HRTF support branch rather than darkening the protected master.

EARLY REFLECTIONS

Current model does not run a full HRTF convolution for every virtual speaker multiplied by every wall reflection.

Instead:

  support contributions
  -> physically timed first-order image paths
  -> grouped into six wall-direction buses
  -> six measured SAF/KEMAR HRTF renders
  -> linear sum with the primary support field

This gives the early room directional HRTF structure at bounded realtime cost. The old analytic first-order reflection bank is disabled on this path so the same early energy is not duplicated.

CURRENT WINDOWS ROLE

Windows owns only:

  lifecycle
  system/process loopback capture
  physical output selection
  realtime scheduling
  queueing / underrun telemetry
  autostart
  watchdog recovery
  tray control

The portable renderer owns music inference, foundation, spatial rendering and binaural behavior. Win32/WASAPI APIs do not define the Omniphony core.

REALTIME CONTINUITY

The playback path uses a bounded producer/callback queue and MMCSS scheduling. The final stereo-linked peak guard keeps its 5 ms look-ahead but maintains the future maximum incrementally rather than rescanning the full window for every sample.

If the playback queue briefly starves, Omniphony uses a short continuity concealment/recovery ramp instead of jumping directly from the last waveform sample to digital zero. The engine logs underrun duration so a host scheduling problem can be distinguished from a DSP artifact.

DEVELOPMENT PACKAGE

  START-OMNIPHONY.cmd  easy double-click launch during rapid build iteration
  Omniphony.exe         supervisor + internal crash-isolated audio engine

The normal release artifact intentionally contains only those two files.

DIAGNOSTICS

  omniphony.log       supervisor + internal engine output

If the audio engine repeatedly restarts or crackle reproduces, inspect omniphony.log first. Playback-queue underrun warnings identify scheduling starvation; absence of those warnings points investigation toward capture/device discontinuity or DSP behavior instead.

RESEARCH HISTORY

Retired listening controls and their physical listening results are preserved in the repository at:

  docs/listening-history.md

They are research evidence, not current product modes.
