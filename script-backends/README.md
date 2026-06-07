# Scriptable render backends (Lua)

Omniphony's **Script** render backend evaluates a user-supplied Lua function to
turn an object position into one gain per speaker. The renderer samples this
function only while building its precomputed gain table — never per audio
sample — so an embedded scripting language is fast enough, and you can iterate
on a panning law without recompiling.

## Selecting a script

In Studio, pick **Script** as the render backend; a **Script file** field
appears (it is a normal backend parameter). Point it at a `.lua` file. The
parameters your script declares via `params()` then appear as sliders.

In a config YAML the same lives in the generic backend-param store:

```yaml
render:
  render_backend: script
  backend_params:
    script:
      path: /path/to/your_backend.lua
      falloff: 0.1
```

The backend runs only in a precomputed evaluation mode; a realtime request is
forced to precomputed (calling Lua per sample is not viable).

## The contract

```lua
-- REQUIRED: gains per speaker for one position.
function gains(pos, speakers, state, params)
  -- pos      = { x=, y=, z= }            (raw ADM position)
  -- speakers = { {x=,y=,z=}, ... }       (unit speaker directions)
  -- state    = value returned by setup(), or nil
  -- params   = { key = number, ... }     (values for params() below)
  -- return an array of #speakers finite numbers, in speaker order.
end

-- OPTIONAL: declare tunable params so Studio renders sliders.
function params()
  return { { key="falloff", label="Falloff", min=0, max=1, step=0.01,
             default=0.1, help="..." } }
end

-- OPTIONAL: one-time per-VM setup; its return value is passed as `state`.
function setup(speakers, params) return {} end
```

Distance attenuation and distance-diffuse are applied by Omniphony *around* your
script, so you only write the directional panning law.

## Sandbox & limits

Scripts run sandboxed: only `math`, `table` and `string` are available (no
`io`, `os`, `require`, …). Each VM is memory-capped and each call is bounded by
an instruction budget, so an infinite loop fails the build instead of hanging
the renderer. A script that errors or returns non-finite / wrong-count gains is
rejected at build time (the engine smoke-tests every freshly built backend).

## Examples

- [`nearest_inverse_distance.lua`](nearest_inverse_distance.lua) — inverse-distance,
  constant-power panning, with `falloff`/`sharpness` params. A good template.
