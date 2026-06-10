# Binaural Headphone Output

The renderer has an independent **binaural output stage** for headphones: when
selected, the whole VBAP / crossover / speaker chain is bypassed and every
input channel (beds and objects) is rendered straight to 2-channel stereo
through an HRTF, with interaural time difference (ITD), shoebox early
reflections and live head tracking.

Per channel, per block:

```
position → rotate(head pose) → (azimuth, elevation, distance)
         → 1/d gain → per-ear ITD delay → per-ear HRIR convolution
         → + 6 first-order shoebox reflections (delay + ILD pan per ear)
         → mix into [L, R]
```

Measured cost: ~0.09 ms per 40-sample block for a 16-channel Atmos stream
(~11 % of the realtime budget), reflections included.

## Enabling it

Set the output mode in `~/.config/omniphony/config.yaml`:

```yaml
render:
  binaural:
    output_mode: binaural      # "speaker" (default) restores the VBAP path
    unit_scale_m: 1.0
    hrir_source: saf
    head_tracking:
      osc_address: /gamerotationvector
      format: auto
```

> **mpv host**: `ad_orender` fixes the channel count when the decoder
> initialises, so the binaural mode must be **active at boot** (in the config)
> — toggling it during playback changes the render but not the negotiated
> channel layout. Restart mpv after switching modes.

Everything below is also live-tunable from the **Binaural / Headphones** panel
in Studio and over OSC (addresses listed at the end).

## Configuration reference (`render.binaural`)

| Key | Default | Meaning |
|---|---|---|
| `output_mode` | `speaker` | `binaural` enables the headphone stage |
| `unit_scale_m` | `1.0` | metres per ADM unit — isotropic distance scale (the anisotropic `room_ratio` is deliberately not used here) |
| `head_radius_m` | `0.0875` | effective head radius (half the inter-ear distance) for the Woodworth ITD model; fit it to the listener (clamped 0.05–0.15) |
| `hrir_source` | `saf` | `saf`/`kemar` (embedded measured KEMAR), `synthetic` (analytic head shadow), `sofa` (personalised set, needs the `sofa` build feature) |
| `hrtf_sofa_path` | — | SOFA file used when `hrir_source: sofa` |
| `head_tracking.osc_address` | — | OSC address carrying the orientation (empty disables tracking) |
| `head_tracking.format` | `auto` | `auto` / `quat` / `rotvec` / `euler` |
| `reflections.enabled` | `true` | shoebox early reflections (externalization) |
| `reflections.room_width_m` | `4.0` | room extent, x (clamped 1–20 m) |
| `reflections.room_depth_m` | `5.0` | room extent, y |
| `reflections.room_height_m` | `2.7` | room extent, z |
| `reflections.level` | `0.5` | per-reflection wall gain (0–1) |

## Head tracking

Any app or device that sends an orientation over OSC works; the address and
format are free. The reference setup is the Android app **Sensors2OSC** with
the phone strapped to the headband:

1. In Sensors2OSC, enable the **Game Rotation Vector** sensor — *not* the
   plain Rotation Vector. The standard sensor fuses the magnetometer, whose
   filtering adds 20–50 ms of latency and drifts near magnets (headphone
   drivers qualify). Game Rotation Vector is gyro+accelerometer only and
   tracks with no perceptible lag.
2. Point it at the renderer's OSC port (default `9000`) and set
   `head_tracking.osc_address: /gamerotationvector` (`format: auto` handles
   the 4/5-float quaternion payload).
3. If the renderer sees nothing while `tcpdump` does, check the host
   firewall: incoming UDP on the OSC port must be allowed.
4. Put the headphones on, look at the screen, press **Recenter** (Studio
   panel or `/omniphony/control/head/recenter`). That direction becomes
   "front".
5. If the scene rotates the wrong way, toggle **Invert rotation**.

`smoothing` (0–0.99, default 0.2) trades a little latency for pose stability;
with Game Rotation Vector you can usually lower it.

## Usage tips

- **Externalization / "inside the head" feeling**: that cue comes almost
  entirely from the early reflections. Start from the defaults and adjust
  **Reflection level** by ear — too high colours dialogue and sounds echoey,
  too low collapses back into the head. Set the room dimensions roughly to
  your actual listening room; they do not need to be exact.
- **Distance**: `unit_scale_m` sets how far "1 ADM unit" is in metres. Raising
  it pushes the whole mix further away (quieter directs, relatively stronger
  reflections); the direct/reflected ratio is the main distance cue.
- **ITD fit**: `head_radius_m` defaults to a KEMAR-ish 8.75 cm. If
  localisation feels smeared, measure ear-to-ear width and set half of it.
- **HRTF**: the embedded measured KEMAR (`saf`) is the best generic default.
  `sofa` lets you load a personalised or alternative measured set (build with
  `--features sofa`). `synthetic` is a lightweight fallback.
- **Head-tracking reaction latency under mpv**: rendered audio waits in mpv's
  output queue, so rotation is only audible once that queue drains. Set
  `audio-buffer=0.05` in `mpv.conf` (default is 0.2 s) to cut the dominant
  term. The Studio 3D head has its own low-latency pose channel and is not
  affected by the audio buffer.
- The output is plain stereo FL/FR — no special player-side configuration
  beyond a stereo sink.

## OSC control surface

| Address | Args | Meaning |
|---|---|---|
| `/omniphony/control/output_mode` | `s: speaker\|binaural` | select the output stage |
| `/omniphony/control/binaural/hrir_source` | `s: synthetic\|saf\|sofa:<path>` | HRIR set |
| `/omniphony/control/binaural/unit_scale` | `f` (m/unit) | distance scale |
| `/omniphony/control/binaural/head_radius` | `f` (m) | ITD head radius |
| `/omniphony/control/binaural/reflections/enabled` | `i\|f` (bool) | reflections on/off |
| `/omniphony/control/binaural/reflections/level` | `f` (0–1) | reflection gain |
| `/omniphony/control/binaural/reflections/room_width` | `f` (m) | room x |
| `/omniphony/control/binaural/reflections/room_depth` | `f` (m) | room y |
| `/omniphony/control/binaural/reflections/room_height` | `f` (m) | room z |
| `/omniphony/control/head/orientation` | `fff` (euler) | set pose directly |
| `/omniphony/control/head/quat` | `ffff` | set pose directly |
| `/omniphony/control/head/recenter` | — | current orientation becomes "front" |
| `/omniphony/control/head/tracking/address` | `s` | tracking OSC address ("" disables) |
| `/omniphony/control/head/tracking/format` | `s` | `auto\|quat\|rotvec\|euler` |
| `/omniphony/control/head/tracking/smoothing` | `f` (0–0.99) | pose smoothing |
| `/omniphony/control/head/tracking/invert` | `i` (bool) | mirror the rotation |

State broadcast: the `binaural` object inside `/omniphony/state/renderer`
(10 Hz when the pose moves), plus a dedicated lightweight
`/omniphony/state/head_pose` (`ffff` = w x y z, ~30 Hz) for low-latency pose
consumers such as the Studio 3D head.
