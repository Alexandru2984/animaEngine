//! Rhai host for scripted behaviors (sub-phase 2 of
//! `docs/plans/v1.2-scripting.md`).
//!
//! Nothing calls this yet — `Behavior::Script` still ticks as Idle. This
//! module exists so the engine, its limits and its sandbox can be built
//! and tested on their own, before they are threaded through `Scene` →
//! `Entity` → `Behavior`.
//!
//! # Why the script is top-level statements, not `fn tick(e)`
//!
//! The design sketched a `tick` function taking an entity object. Rhai
//! passes `Map` arguments by value, so a script mutating `e.x` would
//! change a copy and the move would be silently lost — a trap that only
//! shows up at runtime, as a behavior that does nothing. Top-level
//! statements over scope variables mutate in place, which both works and
//! is less to explain to a script author:
//!
//! ```rhai
//! x += params.speed * dt;
//! if x > bounds_max_x - w { x = bounds_min_x; }
//! ```
//!
//! # What contains a script
//!
//! Rhai has no file, network or process access in the language itself.
//! Two host-side doors are closed explicitly here: `import` (whose
//! default resolver reads files from disk) and `eval`. Everything else
//! is bounded — operations, call depth, expression nesting and the sizes
//! of strings, arrays and maps — because this runs on the UI thread and
//! a runaway script would otherwise freeze the overlay rather than merely
//! misbehave.

use std::collections::BTreeMap;

/// Ceiling on interpreter operations per tick.
///
/// A real motion script is a handful of arithmetic statements; this is
/// orders of magnitude above that and still far below anything that could
/// hold a frame. Exceeding it aborts the script, it does not slow it.
const MAX_OPERATIONS: u64 = 10_000;
/// Nested call depth. Motion scripts are flat; this only exists so
/// runaway recursion terminates.
const MAX_CALL_LEVELS: usize = 16;
/// Cap on strings, arrays and maps a script may build, in that order.
const MAX_STRING_SIZE: usize = 4 * 1024;
const MAX_ARRAY_SIZE: usize = 1024;
const MAX_MAP_SIZE: usize = 1024;

/// Everything a script may read for one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptInputs {
    pub x: f32,
    pub y: f32,
    pub dt: f32,
    pub sprite_width: f32,
    pub sprite_height: f32,
    pub bounds_min_x: f32,
    pub bounds_min_y: f32,
    pub bounds_max_x: f32,
    pub bounds_max_y: f32,
    /// Cursor position, or `None` when it isn't being tracked. Exposed to
    /// the script as `has_cursor` plus `cursor_x` / `cursor_y`, so a
    /// script never reads a stale coordinate believing it is live.
    pub cursor: Option<(f32, f32)>,
    /// Seconds since this behavior started, for phase accumulators.
    pub elapsed: f32,
    pub reduced_motion: bool,
}

/// What a script is allowed to change.
///
/// Position only, matching exactly what `Behavior::tick` may mutate today,
/// so nothing else in the engine has to learn that scripts exist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptOutputs {
    pub x: f32,
    pub y: f32,
}

/// Why a script did not run, or did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    /// The source did not parse.
    Compile(String),
    /// It parsed but failed while running — including hitting a limit,
    /// which Rhai reports as an ordinary runtime error.
    Runtime(String),
    /// Nothing has been compiled under this key.
    NotCompiled,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptError::Compile(e) => write!(f, "script failed to compile: {e}"),
            ScriptError::Runtime(e) => write!(f, "script failed while running: {e}"),
            ScriptError::NotCompiled => write!(f, "script was never compiled"),
        }
    }
}

impl std::error::Error for ScriptError {}

/// One Rhai engine plus the compiled scripts, keyed by whatever the
/// caller uses to identify them (a library-relative path, in practice).
///
/// Compilation is the expensive half, so an AST is built once and reused
/// every frame. The engine itself is not cheap to construct either, hence
/// one per host rather than one per entity.
pub struct ScriptHost {
    engine: rhai::Engine,
    compiled: BTreeMap<String, rhai::AST>,
}

impl Default for ScriptHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptHost {
    pub fn new() -> Self {
        let mut engine = rhai::Engine::new();

        // Bound every axis a script could run away along. These are not
        // tuning knobs — `Behavior::tick` runs on the UI thread, so an
        // unbounded script is a frozen overlay, not a slow one.
        engine.set_max_operations(MAX_OPERATIONS);
        engine.set_max_call_levels(MAX_CALL_LEVELS);
        engine.set_max_string_size(MAX_STRING_SIZE);
        engine.set_max_array_size(MAX_ARRAY_SIZE);
        engine.set_max_map_size(MAX_MAP_SIZE);

        // `import` resolves through a file-reading resolver by default,
        // which would hand scripts the disk access the language otherwise
        // denies them. Swap in one that resolves nothing.
        engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
        // `eval` would let a script build code at runtime and slip past
        // any reasoning about what a given file does.
        engine.disable_symbol("eval");

        Self {
            engine,
            compiled: BTreeMap::new(),
        }
    }

    /// Compile `source` and keep it under `key`, replacing anything
    /// already there. Callers re-compile when the file's mtime moves.
    pub fn compile(&mut self, key: &str, source: &str) -> Result<(), ScriptError> {
        let ast = self
            .engine
            .compile(source)
            .map_err(|e| ScriptError::Compile(e.to_string()))?;
        self.compiled.insert(key.to_string(), ast);
        Ok(())
    }

    /// Whether `key` has a compiled script ready to run.
    pub fn is_compiled(&self, key: &str) -> bool {
        self.compiled.contains_key(key)
    }

    /// Forget one script, so a failure can be made sticky until its file
    /// changes rather than retried sixty times a second.
    pub fn forget(&mut self, key: &str) {
        self.compiled.remove(key);
    }

    /// Run one tick of the script under `key`.
    ///
    /// `scope` carries whatever the script left behind last tick, so an
    /// author can accumulate state across frames; the caller keeps one per
    /// entity. `params` are the author's tunables from the config.
    pub fn run(
        &self,
        key: &str,
        scope: &mut rhai::Scope<'static>,
        inputs: &ScriptInputs,
        params: &BTreeMap<String, f64>,
    ) -> Result<ScriptOutputs, ScriptError> {
        let ast = self.compiled.get(key).ok_or(ScriptError::NotCompiled)?;

        // Rhai's float is f64; the engine's geometry is f32. Convert at
        // this boundary only, so scripts accumulate in the wider type.
        scope.set_value("x", inputs.x as f64);
        scope.set_value("y", inputs.y as f64);
        scope.set_value("dt", inputs.dt as f64);
        scope.set_value("w", inputs.sprite_width as f64);
        scope.set_value("h", inputs.sprite_height as f64);
        scope.set_value("bounds_min_x", inputs.bounds_min_x as f64);
        scope.set_value("bounds_min_y", inputs.bounds_min_y as f64);
        scope.set_value("bounds_max_x", inputs.bounds_max_x as f64);
        scope.set_value("bounds_max_y", inputs.bounds_max_y as f64);
        scope.set_value("elapsed", inputs.elapsed as f64);
        scope.set_value("reduced_motion", inputs.reduced_motion);
        // Split rather than an Option so a script can't read a stale
        // coordinate while believing the cursor is live.
        scope.set_value("has_cursor", inputs.cursor.is_some());
        let (cx, cy) = inputs.cursor.unwrap_or((0.0, 0.0));
        scope.set_value("cursor_x", cx as f64);
        scope.set_value("cursor_y", cy as f64);

        let mut param_map = rhai::Map::new();
        for (k, v) in params {
            param_map.insert(k.as_str().into(), (*v).into());
        }
        scope.set_value("params", param_map);

        // Per-entity scratch space. Seeded once and then left alone, so
        // whatever the script stored last tick is still there — a `let`
        // at script top level would not survive, since Rhai unwinds the
        // scope when the run ends.
        if !scope.contains("state") {
            scope.push("state", rhai::Map::new());
        }

        self.engine
            .run_ast_with_scope(scope, ast)
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        // A script that deletes or retypes `x` gets its last good value
        // back rather than teleporting the entity to the origin.
        let x = read_coord(scope, "x", inputs.x);
        let y = read_coord(scope, "y", inputs.y);

        // Non-finite output would reach GPU quad coordinates. Same
        // reasoning as `Behavior::sanitize`, applied per tick because a
        // script can produce it at any time, not just on load.
        Ok(ScriptOutputs {
            x: finite_or(x as f32, inputs.x),
            y: finite_or(y as f32, inputs.y),
        })
    }
}

/// Read a coordinate the script may have left as either a float or an
/// integer.
///
/// Rhai is dynamically typed and `x = 100` produces an integer, which
/// would otherwise fail the `f64` read and silently pin the entity in
/// place. Accepting both is what a script author expects; anything else
/// (a string, a deleted variable) falls back to the incoming value rather
/// than teleporting the entity to the origin.
fn read_coord(scope: &rhai::Scope<'static>, name: &str, fallback: f32) -> f64 {
    if let Some(v) = scope.get_value::<f64>(name) {
        return v;
    }
    if let Some(v) = scope.get_value::<i64>(name) {
        return v as f64;
    }
    fallback as f64
}

fn finite_or(v: f32, fallback: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> ScriptInputs {
        ScriptInputs {
            x: 100.0,
            y: 200.0,
            dt: 1.0 / 60.0,
            sprite_width: 64.0,
            sprite_height: 64.0,
            bounds_min_x: 0.0,
            bounds_min_y: 0.0,
            bounds_max_x: 1920.0,
            bounds_max_y: 1080.0,
            cursor: None,
            elapsed: 0.0,
            reduced_motion: false,
        }
    }

    fn run_once(source: &str) -> Result<ScriptOutputs, ScriptError> {
        let mut host = ScriptHost::new();
        host.compile("t", source)?;
        let mut scope = rhai::Scope::new();
        host.run("t", &mut scope, &inputs(), &BTreeMap::new())
    }

    #[test]
    fn a_script_can_move_the_entity() {
        let out = run_once("x += 10.0; y -= 5.0;").unwrap();
        assert_eq!(out, ScriptOutputs { x: 110.0, y: 195.0 });
    }

    #[test]
    fn a_script_reads_dt_and_bounds() {
        let out = run_once("x = bounds_max_x - w; y = dt * 60.0;").unwrap();
        assert_eq!(out.x, 1920.0 - 64.0);
        assert!((out.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn params_reach_the_script() {
        let mut host = ScriptHost::new();
        host.compile("t", "x += params.speed;").unwrap();
        let mut scope = rhai::Scope::new();
        let params: BTreeMap<String, f64> = [("speed".to_string(), 42.0)].into_iter().collect();
        let out = host.run("t", &mut scope, &inputs(), &params).unwrap();
        assert_eq!(out.x, 142.0);
    }

    /// A script needs somewhere to accumulate across frames. A top-level
    /// `let` will not do it — Rhai unwinds the scope when a run ends — so
    /// the host seeds a `state` map instead. This is the test that pins
    /// that down.
    #[test]
    fn state_map_persists_across_ticks() {
        let mut host = ScriptHost::new();
        host.compile(
            "t",
            r#"
            if !state.contains("n") { state.n = 0; }
            state.n += 1;
            x = state.n;
            "#,
        )
        .unwrap();
        let mut scope = rhai::Scope::new();
        for expected in 1..=3 {
            let out = host
                .run("t", &mut scope, &inputs(), &BTreeMap::new())
                .unwrap();
            assert_eq!(out.x, expected as f32, "state did not survive tick");
        }
    }

    /// Two entities running the same script must not share accumulators,
    /// which is the whole reason the scope is owned by the caller.
    #[test]
    fn separate_scopes_do_not_share_state() {
        let mut host = ScriptHost::new();
        host.compile(
            "t",
            r#"if !state.contains("n") { state.n = 0; } state.n += 1; x = state.n;"#,
        )
        .unwrap();
        let mut a = rhai::Scope::new();
        let mut b = rhai::Scope::new();
        host.run("t", &mut a, &inputs(), &BTreeMap::new()).unwrap();
        host.run("t", &mut a, &inputs(), &BTreeMap::new()).unwrap();
        let out_b = host.run("t", &mut b, &inputs(), &BTreeMap::new()).unwrap();
        assert_eq!(out_b.x, 1.0, "second entity inherited the first's state");
    }

    #[test]
    fn a_syntax_error_is_reported_not_panicked() {
        let mut host = ScriptHost::new();
        let err = host.compile("t", "x +=* 3").unwrap_err();
        assert!(matches!(err, ScriptError::Compile(_)));
        assert!(!host.is_compiled("t"));
    }

    #[test]
    fn running_an_uncompiled_key_is_an_error() {
        let host = ScriptHost::new();
        let mut scope = rhai::Scope::new();
        let err = host
            .run("missing", &mut scope, &inputs(), &BTreeMap::new())
            .unwrap_err();
        assert_eq!(err, ScriptError::NotCompiled);
    }

    /// The one that matters most: this runs on the UI thread, so an
    /// infinite loop has to terminate rather than hang the overlay.
    #[test]
    fn an_infinite_loop_is_killed_by_the_operation_limit() {
        let err = run_once("while true { x += 1.0; }").unwrap_err();
        assert!(
            matches!(err, ScriptError::Runtime(_)),
            "expected a runtime abort, got {err:?}"
        );
    }

    #[test]
    fn runaway_recursion_terminates() {
        let err = run_once("fn f(n) { f(n + 1) } f(0);").unwrap_err();
        assert!(matches!(err, ScriptError::Runtime(_)));
    }

    /// Non-finite output would land in GPU quad coordinates.
    #[test]
    fn non_finite_output_falls_back_to_the_previous_position() {
        let out = run_once("x = 0.0/0.0; y = 1.0/0.0;").unwrap();
        assert_eq!(out, ScriptOutputs { x: 100.0, y: 200.0 });
    }

    #[test]
    fn deleting_x_does_not_teleport_the_entity() {
        let out = run_once("x = \"not a number\";").unwrap();
        assert_eq!(out.x, 100.0);
    }

    /// `x = 100` is an integer literal in Rhai, and a script author will
    /// write it without thinking. Rejecting it would pin the entity in
    /// place with no error to explain why.
    #[test]
    fn an_integer_coordinate_is_accepted() {
        let out = run_once("x = 250; y = 300;").unwrap();
        assert_eq!(out, ScriptOutputs { x: 250.0, y: 300.0 });
    }

    // ── Sandbox ───────────────────────────────────────────────────────

    #[test]
    fn import_cannot_reach_the_filesystem() {
        let err = run_once(r#"import "/etc/passwd" as p;"#).unwrap_err();
        // Either the resolver refuses it or it never parses — both are
        // closed doors; what matters is that no file is read.
        assert!(matches!(
            err,
            ScriptError::Runtime(_) | ScriptError::Compile(_)
        ));
    }

    #[test]
    fn eval_is_disabled() {
        let err = run_once(r#"eval("x = 999.0");"#).unwrap_err();
        assert!(matches!(
            err,
            ScriptError::Runtime(_) | ScriptError::Compile(_)
        ));
    }

    /// Rhai has no file or network API in the language, so the check that
    /// matters is that we did not register one. Spot-check the names a
    /// script author would reach for first.
    #[test]
    fn no_host_functions_expose_io() {
        for source in [
            "open(\"/etc/passwd\");",
            "read_file(\"/etc/passwd\");",
            "system(\"id\");",
            "http_get(\"http://example.com\");",
        ] {
            let err = run_once(source).unwrap_err();
            assert!(
                matches!(err, ScriptError::Runtime(_) | ScriptError::Compile(_)),
                "{source} was not rejected"
            );
        }
    }

    #[test]
    fn forget_makes_a_script_unavailable_again() {
        let mut host = ScriptHost::new();
        host.compile("t", "x += 1.0;").unwrap();
        assert!(host.is_compiled("t"));
        host.forget("t");
        assert!(!host.is_compiled("t"));
    }
}
