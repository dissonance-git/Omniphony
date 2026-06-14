# Channel content render modes

Some streams carry **spatial objects** (Atmos / JOC E-AC3, object TrueHD): orender
renders those through VBAP as designed. Others are plain **channel beds** — a 5.1
or 7.1 mix with no objects (non-JOC E-AC3, channel TrueHD, AC-3, multichannel
PCM). This page covers how that *channel-based (non-object)* content is rendered.

There are three modes, applied **identically** by the CLI/spdif decode path and
by the embedded mpv decoder (`--ad=orender`):

| Mode      | What happens                                                                                   |
|-----------|------------------------------------------------------------------------------------------------|
| `virtual` | **(default)** Each input channel becomes a virtual object placed at its theoretical speaker angle and is rendered through VBAP across your whole layout. The flagship behaviour. |
| `direct`  | Each bed channel is routed straight to the matching speaker of your layout, with no virtualization. Channels with no matching speaker are dropped. |
| `host`    | No spatialization. In **mpv** the orender decoder declines and mpv falls back to its native decoder (`ad_lavc`) — its own downmix. In the **CLI** the decoded channels are written straight to the output sink. |

Object-based content is **never** affected by this setting.

## How to choose the mode

### Studio
Renderer panel → **Channel content** selector (next to *Ramp mode*). The change is
live and is saved to the config.

### Config file (`render.channel_render_mode`)
The shared config (`~/.config/omniphony/config.yaml`, or
`%ProgramData%\omniphony\config.yaml` on Windows) is the source of truth for both
the CLI and mpv:

```yaml
render:
  channel_render_mode: virtual   # host | direct | virtual (default: virtual)
```

### CLI
```
orender decode --channel-render-mode virtual|direct|host …
```

### mpv
By default mpv follows the shared config. To override for one invocation:
```
mpv --ad=orender --ad-orender-channel-mode=host|direct|virtual …
```

### OSC
Live control: `/omniphony/control/channel_render_mode` with a string argument
(`host` / `direct` / `virtual`). The current value is broadcast in the live-state
snapshot as `channelRenderMode`.

## Output speaker layout (5.1.4, 7.1.4, …)

The *output* layout — how many speakers you have and where they sit — is a
separate setting from the modes above. Configure it in Studio (it is embedded in
the config as `render.current_layout`) or point `render.speaker_layout` at a
layout YAML. Bundled layouts live in the `layouts/` folder (height-less 5.1/7.1
beds are under `layouts/legacy/`). On Windows, layout files are also discovered
under `%ProgramData%\omniphony\layouts`.

In `virtual` mode every input channel is spread across whatever output layout you
configure (e.g. a 5.1 bed virtualized over a 7.1.4 rig). In `direct` mode only
the input channels that have a matching speaker in your layout are heard.

## Troubleshooting

- **Non-Atmos E-AC3 plays but you want your player's normal downmix** → set the
  mode to `host`.
- **Playback froze on a non-object track (older builds)** → if the renderer cannot
  resolve an output layout it now hands the track back to the native decoder
  instead of stalling. Make sure a speaker layout is configured (see above).
