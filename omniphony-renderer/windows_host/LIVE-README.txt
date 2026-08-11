Omniphony for Headphones - Windows prototype

NORMAL USE

1. Extract this entire folder to a stable location. Do not run it permanently from a temporary ZIP/extraction folder because Windows autostart points to the exact Omniphony.exe path.
2. Keep normal Windows / foobar playback routed to the virtual sink you already use (currently Hi-Fi Cable) so the unprocessed signal does not also play directly from the physical headphones.
3. Disable HeSuVi / DTS Virtual:X and stop ASIO Bridge or any other old route that independently forwards the virtual sink to the FiiO.
4. During development, double-click START-OMNIPHONY.cmd. It simply launches Omniphony.exe from the extracted folder.

Omniphony.exe opens no normal window. It lives in the Windows notification area and registers itself for per-user Windows autostart on first run. After that, normal use should require no manual launch after login.

There is only one shipped Omniphony runtime executable. The normal Omniphony.exe instance is the invisible supervisor. For crash isolation it launches a second instance of the same Omniphony.exe in a private internal audio-engine mode. This preserves watchdog/recovery behavior without shipping a separate worker executable.

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
  -> inherited Omniphony cascaded speaker-room renderer
  -> HRTF / ITD / metric distance / first-order reflections / air cues
  -> fixed linear final summing
  -> physical headphones

OFF:

  clean captured stereo reference through the same fixed output headroom

The finished stereo master never becomes a full-wet virtual-room replacement. Bass/body remains direct-dominant. Full spatial support starts above 320 Hz. Current upper support is deliberately restrained to reduce fatigue while height and scale come from geometry rather than treble gain.

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

DEVELOPMENT PACKAGE

  START-OMNIPHONY.cmd  easy double-click launch during rapid build iteration
  Omniphony.exe         supervisor + internal crash-isolated audio engine
  omniphony_live.exe    diagnostic/reference tool only, not part of normal runtime
  reference_bridge.dll  current native bridge dependency
  reference-demo\       current renderer configs/layout

DIAGNOSTICS

  omniphony.log       supervisor + internal engine output
  LIST-DEVICES.cmd    list WASAPI devices

If the audio engine repeatedly restarts, inspect omniphony.log first. The engine also reports playback-queue underruns so CPU scheduling crackle can be separated from DSP artifacts.
