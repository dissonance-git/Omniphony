//! # Scriptable render backend (Lua)
//!
//! A [`GainModel`] whose per-position gain function is written in **Lua** and
//! editable by the user. It depends on `renderer` through its **public API
//! only** (like `example_backend`), so it doubles as proof that a non-trivial
//! backend can live out-of-tree.
//!
//! ## Why a scripting language is viable here
//!
//! A backend is geometry-only and its `compute_gains` is invoked *only while the
//! precomputed gain table is sampled* (the host runs it once per grid point on a
//! build thread), never per audio sample. So the cost of crossing into an
//! embedded interpreter is paid once, at table-build time. Accordingly the
//! backend is **not** hot-path safe: [`ScriptFactory::realtime_capable`] returns
//! `false` and [`ScriptBackend::capabilities`] sets `supports_realtime = false`,
//! so the host forces a precomputed evaluation mode.
//!
//! ## The Lua contract
//!
//! ```lua
//! -- REQUIRED: one gain per speaker for a position.
//! function gains(pos, speakers, state, params)
//!   -- pos      = { x=, y=, z= }            (raw ADM position)
//!   -- speakers = { {x=,y=,z=}, ... }       (unit speaker directions)
//!   -- state    = value returned by setup(), or nil
//!   -- params   = { key = number, ... }     (resolved from the schema below)
//!   -- return an array of #speakers finite numbers, in speaker order.
//! end
//!
//! -- OPTIONAL: one-time per-VM setup; its return value is passed as `state`.
//! function setup(speakers, params) return {} end
//!
//! -- OPTIONAL: declare tunable params so Studio renders controls for them.
//! function params()
//!   return {
//!     { key = "falloff", label = "Falloff", min = 0.0, max = 1.0,
//!       step = 0.01, default = 0.1, help = "Softening near a speaker" },
//!   }
//! end
//! ```
//!
//! Distance attenuation and distance-diffuse are applied by the host *around*
//! this model, so the script only writes the directional panning law.
//!
//! ## Sandbox
//!
//! Each VM exposes only `math`/`table`/`string` (no `io`/`os`/`require`/
//! `debug`), is memory-capped, and each call is bounded by an instruction
//! budget so a runaway script fails the build instead of hanging it.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow, bail};
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};

use renderer::backend_params::{ParamSpec, ParamValue};
use renderer::backend_registry::{
    BackendBuildCtx, BackendBuildPlan, BackendFactory, DynamicBackendPlan,
};
use renderer::render_backend::{BackendCapabilities, GainModel, RenderRequest, RenderResponse};
use renderer::spatial_vbap::{Gains, spherical_to_adm};
use renderer::speaker_layout::SpeakerLayout;

/// Per-VM heap cap: generous for honest scripts, low enough that a runaway
/// allocation aborts the build instead of exhausting the machine.
const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
/// Instruction budget per `gains` call. A normal call is a few thousand
/// instructions; this only trips on an infinite loop.
const INSTRUCTION_BUDGET: u64 = 5_000_000;
/// How often the debug hook fires (VM instructions).
const HOOK_INTERVAL: u32 = 4096;
/// Param key holding the path to the `.lua` file.
const PATH_KEY: &str = "path";

static SCRIPT_GENERATION: AtomicU64 = AtomicU64::new(1);

fn next_generation() -> u64 {
    SCRIPT_GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// `mlua::Error` is not `Sync` without the `send` feature (the VM is kept
/// thread-local on purpose), so it can't flow through `?` into `anyhow`.
fn lua_err(e: mlua::Error) -> anyhow::Error {
    anyhow!("{e}")
}

// ===========================================================================
// The gain model
// ===========================================================================

/// A Lua-scripted gain model. `Send + Sync`: it holds only plain data; the
/// non-`Send` [`Lua`] VM lives in a `thread_local` cache keyed by `generation`,
/// so the host's parallel sampling gets one VM per worker with no locking.
pub struct ScriptBackend {
    source: Arc<str>,
    /// Unit speaker direction vectors, one per spatializable speaker.
    speakers: Vec<[f32; 3]>,
    /// Resolved numeric params handed to the script as a Lua table.
    params: Vec<(String, f64)>,
    generation: u64,
}

impl ScriptBackend {
    /// Build and **eagerly validate** the backend: compile the source, run
    /// `setup`, and probe `gains` once so a broken script fails the build with a
    /// precise message instead of mid-sampling.
    pub fn new(
        source: impl Into<Arc<str>>,
        speakers: Vec<[f32; 3]>,
        params: Vec<(String, f64)>,
    ) -> Result<Self> {
        let backend = Self {
            source: source.into(),
            speakers,
            params,
            generation: next_generation(),
        };
        // Eager smoke: compile + setup + one probe call.
        let vm = backend.build_vm()?;
        vm.eval_gains([0.0, 0.0, 0.0])
            .map_err(|e| anyhow!("script `gains` failed on probe: {e}"))?;
        Ok(backend)
    }

    fn build_vm(&self) -> Result<ScriptVm> {
        ScriptVm::new(&self.source, &self.speakers, &self.params)
    }
}

thread_local! {
    static VM_CACHE: RefCell<Option<CachedVm>> = const { RefCell::new(None) };
    static INSTRUCTIONS: Cell<u64> = const { Cell::new(0) };
}

struct CachedVm {
    generation: u64,
    vm: ScriptVm,
}

impl GainModel for ScriptBackend {
    fn backend_id(&self) -> &'static str {
        "script"
    }

    fn backend_label(&self) -> &'static str {
        "Script"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            // Realtime would call Lua per audio sample — forbidden.
            supports_realtime: false,
            supports_precomputed_polar: true,
            supports_precomputed_cartesian: true,
            supports_position_interpolation: true,
            // Distance attenuation / diffuse are applied by the host decorators.
            supports_distance_model: true,
            supports_spread: false,
            supports_spread_from_distance: false,
            supports_event_size: false,
            supports_distance_diffuse: true,
            supports_table_export: true,
        }
    }

    fn speaker_count(&self) -> usize {
        self.speakers.len()
    }

    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        let pos = [
            req.adm_position[0] as f32,
            req.adm_position[1] as f32,
            req.adm_position[2] as f32,
        ];
        let result = VM_CACHE.with(|cell| -> Result<Gains> {
            let mut slot = cell.borrow_mut();
            if slot.as_ref().map(|c| c.generation) != Some(self.generation) {
                *slot = Some(CachedVm {
                    generation: self.generation,
                    vm: self.build_vm()?,
                });
            }
            let vm = &slot.as_ref().expect("vm just inserted").vm;
            let values = vm.eval_gains(pos)?;
            let mut gains = Gains::zeroed(self.speakers.len());
            for (i, g) in values.into_iter().enumerate() {
                gains.set(i, g);
            }
            Ok(gains)
        });

        match result {
            Ok(gains) => RenderResponse { gains },
            Err(_) => {
                // A sampling-time error: emit non-finite gains so the host's
                // build-time smoke test rejects this backend (the eager probe in
                // `new` already caught the common cases with a precise message).
                let mut gains = Gains::zeroed(self.speakers.len());
                for i in 0..self.speakers.len() {
                    gains.set(i, f32::NAN);
                }
                RenderResponse { gains }
            }
        }
    }

    fn save_to_file(&self, _path: &std::path::Path, _layout: &SpeakerLayout) -> Result<()> {
        bail!("the scriptable backend does not support table export")
    }
}

// ===========================================================================
// The VM
// ===========================================================================

struct ScriptVm {
    _lua: Lua,
    gains_fn: Function,
    speaker_count: usize,
    speakers_tbl: Table,
    state: Value,
    params_tbl: Table,
    pos_tbl: Table,
}

impl ScriptVm {
    fn new(source: &str, speakers: &[[f32; 3]], params: &[(String, f64)]) -> Result<Self> {
        let lua = sandboxed_lua()?;
        lua.load(source)
            .exec()
            .map_err(|e| anyhow!("failed to load script: {e}"))?;
        let globals = lua.globals();

        let speakers_tbl = speakers_table(&lua, speakers).map_err(lua_err)?;
        let params_tbl = params_table(&lua, params).map_err(lua_err)?;

        let state = match globals.get::<Value>("setup").map_err(lua_err)? {
            Value::Function(setup) => {
                reset_instructions();
                setup
                    .call((speakers_tbl.clone(), params_tbl.clone()))
                    .map_err(|e| anyhow!("script `setup` failed: {e}"))?
            }
            Value::Nil => Value::Nil,
            other => bail!("`setup` must be a function, got {}", other.type_name()),
        };

        let gains_fn = match globals.get::<Value>("gains").map_err(lua_err)? {
            Value::Function(f) => f,
            Value::Nil => {
                bail!("script must define a `gains(pos, speakers, state, params)` function")
            }
            other => bail!("`gains` must be a function, got {}", other.type_name()),
        };

        let pos_tbl = lua.create_table().map_err(lua_err)?;

        Ok(Self {
            _lua: lua,
            gains_fn,
            speaker_count: speakers.len(),
            speakers_tbl,
            state,
            params_tbl,
            pos_tbl,
        })
    }

    fn eval_gains(&self, pos: [f32; 3]) -> Result<Vec<f32>> {
        self.pos_tbl.set("x", pos[0]).map_err(lua_err)?;
        self.pos_tbl.set("y", pos[1]).map_err(lua_err)?;
        self.pos_tbl.set("z", pos[2]).map_err(lua_err)?;

        reset_instructions();
        let result: Table = self
            .gains_fn
            .call((
                self.pos_tbl.clone(),
                self.speakers_tbl.clone(),
                self.state.clone(),
                self.params_tbl.clone(),
            ))
            .map_err(lua_err)?;

        let len = result.raw_len();
        if len != self.speaker_count {
            bail!(
                "`gains` returned {len} values but the layout has {} speakers",
                self.speaker_count
            );
        }
        let mut out = Vec::with_capacity(self.speaker_count);
        for i in 1..=self.speaker_count {
            let g: f32 = result.get(i).map_err(lua_err)?;
            if !g.is_finite() {
                bail!("`gains[{i}]` is not finite ({g})");
            }
            out.push(g);
        }
        Ok(out)
    }
}

fn sandboxed_lua() -> Result<Lua> {
    let lua = Lua::new_with(
        StdLib::MATH | StdLib::TABLE | StdLib::STRING,
        LuaOptions::default(),
    )
    .map_err(|e| anyhow!("failed to create Lua VM: {e}"))?;
    let _ = lua.set_memory_limit(MEMORY_LIMIT_BYTES);
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(HOOK_INTERVAL),
        |_lua, _debug| {
            let used = INSTRUCTIONS.with(|c| {
                let v = c.get() + HOOK_INTERVAL as u64;
                c.set(v);
                v
            });
            if used > INSTRUCTION_BUDGET {
                Err(mlua::Error::RuntimeError(
                    "script exceeded its instruction budget (possible infinite loop)".into(),
                ))
            } else {
                Ok(VmState::Continue)
            }
        },
    );
    Ok(lua)
}

fn reset_instructions() {
    INSTRUCTIONS.with(|c| c.set(0));
}

fn speakers_table(lua: &Lua, speakers: &[[f32; 3]]) -> mlua::Result<Table> {
    let table = lua.create_table_with_capacity(speakers.len(), 0)?;
    for (i, s) in speakers.iter().enumerate() {
        let entry = lua.create_table_with_capacity(0, 3)?;
        entry.set("x", s[0])?;
        entry.set("y", s[1])?;
        entry.set("z", s[2])?;
        table.set(i + 1, entry)?;
    }
    Ok(table)
}

fn params_table(lua: &Lua, params: &[(String, f64)]) -> mlua::Result<Table> {
    let table = lua.create_table_with_capacity(0, params.len())?;
    for (key, value) in params {
        table.set(key.as_str(), *value)?;
    }
    Ok(table)
}

// ===========================================================================
// The factory
// ===========================================================================

/// Registers [`ScriptBackend`] under the id `"script"`.
pub struct ScriptFactory;

impl BackendFactory for ScriptFactory {
    fn id(&self) -> &'static str {
        "script"
    }

    fn label(&self) -> &'static str {
        "Script"
    }

    fn realtime_capable(&self) -> bool {
        false
    }

    fn param_schema(&self) -> Vec<ParamSpec> {
        vec![
            ParamSpec::path(PATH_KEY, "Script file", "")
                .help("Path to a .lua file implementing gains(pos, speakers, state, params)."),
        ]
    }

    fn param_schema_for(
        &self,
        params: &std::collections::HashMap<String, ParamValue>,
    ) -> Vec<ParamSpec> {
        let mut schema = self.param_schema();
        // If a file is selected, ask it which params it declares and surface
        // those controls too.
        if let Some(path) = params.get(PATH_KEY).and_then(ParamValue::as_str) {
            if !path.trim().is_empty() {
                if let Ok(source) = std::fs::read_to_string(path) {
                    schema.extend(script_declared_params(&source));
                }
            }
        }
        schema
    }

    fn build_plan(&self, ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan> {
        // Unit speaker directions from the layout (captured on the build thread).
        let (azimuth_elevation, _) = ctx.layout.spatializable_positions();
        let speakers: Vec<[f32; 3]> = azimuth_elevation
            .iter()
            .map(|[az, el]| {
                let (x, y, z) = spherical_to_adm(*az, *el, 1.0);
                [x, y, z]
            })
            .collect();

        let path = ctx
            .backend_param(self.id(), PATH_KEY)
            .and_then(ParamValue::as_str)
            .unwrap_or("")
            .trim()
            .to_string();

        // Resolve the path → source, and the declared params → values, now. A
        // missing/unreadable file produces a builder that fails with a clear
        // message (surfaced via the recompute error banner), rather than `None`
        // (which would silently keep the previous backend).
        let load = (!path.is_empty())
            .then(|| std::fs::read_to_string(&path))
            .transpose();

        let (source, params) = match load {
            Ok(Some(source)) => {
                let values = script_declared_params(&source)
                    .into_iter()
                    .filter_map(|spec| {
                        let v = ctx
                            .backend_param(self.id(), spec.key)
                            .and_then(ParamValue::as_f32)
                            .or_else(|| spec.default.as_f32())?;
                        Some((spec.key.to_string(), v as f64))
                    })
                    .collect::<Vec<_>>();
                (Some(source), values)
            }
            Ok(None) => (None, Vec::new()),
            Err(e) => {
                let msg = format!("script backend: cannot read '{path}': {e}");
                return Some(BackendBuildPlan::Dynamic(DynamicBackendPlan::new(
                    "script",
                    move || bail!("{msg}"),
                )));
            }
        };

        let builder_msg = "script backend: no script file selected".to_string();
        Some(BackendBuildPlan::Dynamic(DynamicBackendPlan::new(
            "script",
            move || match &source {
                Some(source) => Ok(Box::new(ScriptBackend::new(
                    source.as_str(),
                    speakers.clone(),
                    params.clone(),
                )?) as Box<dyn GainModel>),
                None => bail!("{builder_msg}"),
            },
        )))
    }
}

/// Run a script's optional `params()` function in a throwaway sandboxed VM and
/// map its descriptors to [`ParamSpec`]s. Any failure yields an empty list (the
/// backend still works with the script's own defaults).
fn script_declared_params(source: &str) -> Vec<ParamSpec> {
    fn try_read(source: &str) -> Result<Vec<ParamSpec>> {
        let lua = sandboxed_lua()?;
        lua.load(source).exec().map_err(lua_err)?;
        let params_fn = match lua.globals().get::<Value>("params").map_err(lua_err)? {
            Value::Function(f) => f,
            _ => return Ok(Vec::new()),
        };
        reset_instructions();
        let list: Table = params_fn.call(()).map_err(lua_err)?;
        let mut specs = Vec::new();
        for entry in list.sequence_values::<Table>() {
            let entry = entry.map_err(lua_err)?;
            let key: String = entry.get("key").map_err(lua_err)?;
            // `ParamSpec` needs a `&'static str` key; leak the (few, build-time)
            // script-declared keys to satisfy that without reworking the schema.
            let key: &'static str = Box::leak(key.into_boxed_str());
            let label: String = entry.get("label").unwrap_or_else(|_| key.to_string());
            let label: &'static str = Box::leak(label.into_boxed_str());
            let min: f32 = entry.get("min").unwrap_or(0.0);
            let max: f32 = entry.get("max").unwrap_or(1.0);
            let step: f32 = entry.get("step").unwrap_or(0.01);
            let default: f32 = entry.get("default").unwrap_or(0.0);
            let mut spec = ParamSpec::float(key, label, min, max, step, default);
            if let Ok(help) = entry.get::<String>("help") {
                let help: &'static str = Box::leak(help.into_boxed_str());
                spec = spec.help(help);
            }
            specs.push(spec);
        }
        Ok(specs)
    }
    try_read(source).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer::backend_params::ParamKind;

    const NEAREST: &str = r#"
        function gains(pos, speakers, state, params)
          local out, best, bestd = {}, 1, math.huge
          for i = 1, #speakers do
            out[i] = 0.0
            local s = speakers[i]
            local dx, dy, dz = pos.x - s.x, pos.y - s.y, pos.z - s.z
            local d = dx*dx + dy*dy + dz*dz
            if d < bestd then bestd, best = d, i end
          end
          out[best] = 1.0
          return out
        end
    "#;

    fn speakers() -> Vec<[f32; 3]> {
        vec![
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
        ]
    }

    fn request(pos: [f64; 3]) -> RenderRequest {
        RenderRequest {
            adm_position: pos,
            event_size: [0.0; 3],
            size_to_spread_mode: Default::default(),
            spread_min: 0.0,
            spread_max: 0.0,
            spread_from_distance: false,
            spread_distance_range: 1.0,
            spread_distance_curve: 1.0,
            room_ratio: [1.0, 1.0, 1.0],
            room_ratio_rear: 1.0,
            room_ratio_lower: 1.0,
            room_ratio_center_blend: 0.5,
            use_distance_diffuse: false,
            distance_diffuse_threshold: 1.0,
            distance_diffuse_curve: 1.0,
            distance_model: Default::default(),
        }
    }

    fn backend(src: &str) -> Result<ScriptBackend> {
        ScriptBackend::new(src, speakers(), Vec::new())
    }

    #[test]
    fn nearest_script_selects_closest() {
        let model = backend(NEAREST).expect("valid script");
        let gains = model.compute_gains(&request([0.9, 0.0, 0.0])).gains;
        assert_eq!(gains.len(), 4);
        assert_eq!(gains[1], 1.0);
        assert!(gains.iter().all(|g| g.is_finite()));
    }

    #[test]
    fn params_and_setup_reach_the_script() {
        let src = r#"
            function setup(speakers, params) return { n = #speakers } end
            function gains(pos, speakers, state, params)
              local out = {}
              for i = 1, #speakers do out[i] = (params.level or 0.0) + state.n end
              return out
            end
        "#;
        let model = ScriptBackend::new(src, speakers(), vec![("level".into(), 0.25)])
            .expect("valid script");
        let gains = model.compute_gains(&request([0.0, 0.0, 0.0])).gains;
        assert!(gains.iter().all(|g| (*g - 4.25).abs() < 1e-6));
    }

    #[test]
    fn broken_scripts_fail_construction() {
        assert!(backend("function gains( not lua").is_err(), "syntax");
        assert!(backend("local x = 1").is_err(), "missing gains");
        assert!(
            backend("function gains(p,s,st,pa) return {1,2} end").is_err(),
            "wrong length"
        );
        assert!(
            backend("function gains(p,s,st,pa) local o={} for i=1,#s do o[i]=1/0 end return o end")
                .is_err(),
            "non-finite"
        );
    }

    #[test]
    fn sandbox_denies_dangerous_stdlib() {
        assert!(backend("function gains(p,s,st,pa) return os.time() end").is_err());
        assert!(backend("function gains(p,s,st,pa) io.write('x') return {} end").is_err());
        assert!(backend("function gains(p,s,st,pa) require('os') return {} end").is_err());
        assert!(backend("function gains(p,s,st,pa) return { debug.getinfo(1) } end").is_err());
    }

    #[test]
    fn infinite_loop_and_runaway_allocation_are_bounded() {
        assert!(backend("function gains(p,s,st,pa) while true do end end").is_err());
        assert!(
            backend(
                "function gains(p,s,st,pa) local b=string.rep('x',256*1024*1024) return {#b} end"
            )
            .is_err()
        );
    }

    #[test]
    fn sampling_time_error_yields_non_finite_for_the_smoke_test() {
        // Passes the eager probe at [0,0,0] but errors elsewhere; compute_gains
        // then returns non-finite gains so the host smoke test rejects it.
        let src = r#"
            function gains(pos, speakers, state, params)
              if pos.x < -0.5 then error("boom") end
              local out = {}
              for i = 1, #speakers do out[i] = 0.0 end
              out[1] = 1.0
              return out
            end
        "#;
        let model = backend(src).expect("eager probe at origin passes");
        let bad = model.compute_gains(&request([-1.0, 0.0, 0.0])).gains;
        assert!(bad.iter().any(|g| !g.is_finite()));
    }

    #[test]
    fn factory_declares_path_and_is_not_realtime() {
        let f = ScriptFactory;
        assert!(!f.realtime_capable());
        let schema = f.param_schema();
        assert!(matches!(schema[0].kind, ParamKind::Path));
        assert_eq!(schema[0].key, "path");
    }

    #[test]
    fn script_declared_params_are_parsed() {
        let src = r#"
            function params()
              return { { key="falloff", label="Falloff", min=0.0, max=1.0, default=0.1 } }
            end
            function gains(p,s,st,pa) return {} end
        "#;
        let specs = script_declared_params(src);
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].key, "falloff");
        assert!(matches!(specs[0].kind, ParamKind::Float { .. }));
        // A script without params() declares none.
        assert!(script_declared_params("function gains(p,s,st,pa) return {} end").is_empty());
    }

    #[test]
    fn shipped_example_loads_and_declares_its_params() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../script-backends/nearest_inverse_distance.lua"
        );
        let source = std::fs::read_to_string(path).expect("example script readable");
        let model =
            ScriptBackend::new(source.clone(), speakers(), Vec::new()).expect("example is valid");
        let gains = model.compute_gains(&request([0.7, -0.3, 0.2])).gains;
        let energy: f32 = gains.iter().map(|g| g * g).sum();
        assert!(
            (energy - 1.0).abs() < 1e-4,
            "constant-power, energy={energy}"
        );

        let keys: Vec<&str> = script_declared_params(&source)
            .iter()
            .map(|s| s.key)
            .collect();
        assert!(keys.contains(&"falloff") && keys.contains(&"sharpness"));
    }
}
