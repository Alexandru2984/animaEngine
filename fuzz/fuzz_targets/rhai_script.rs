//! Fuzz a behavior script as a character runs it (`ScriptHost`): Rhai
//! compiling the source under the app's limits, then a few steps of it
//! with its inputs and params (TEST-001: the script sandbox was never
//! fuzzed — SEC-004's runaway was found by hand). Each input is a script
//! of its own, on a host of its own; written to a RAM-backed directory.
//! What this finds: panics, hangs past the operation limits
//! (`-timeout`), and runaway allocation.

#![no_main]

use anima_engine::scripting::{ScriptHost, ScriptInputs};
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

fn dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let shm = std::path::Path::new("/dev/shm");
        let base = if shm.is_dir() {
            shm.to_path_buf()
        } else {
            std::env::temp_dir()
        };
        let dir = base.join(format!("anima-fuzz-rhai-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("fuzz dir");
        dir
    })
}

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let root = dir();
    if std::fs::write(root.join("s.rhai"), source).is_err() {
        return;
    }
    let mut host = ScriptHost::new();
    if host.ensure_compiled(root, "s.rhai").is_err() {
        return;
    }
    let inputs = ScriptInputs {
        x: 100.0,
        y: 100.0,
        dt: 1.0 / 60.0,
        sprite_width: 64.0,
        sprite_height: 64.0,
        bounds_min_x: 0.0,
        bounds_min_y: 0.0,
        bounds_max_x: 1920.0,
        bounds_max_y: 1080.0,
        cursor: Some((500.0, 400.0)),
        elapsed: 1.0,
        reduced_motion: false,
        cpu: 0.5,
        mem: 0.5,
    };
    let params: BTreeMap<String, f64> = [("speed".to_string(), 60.0)].into_iter().collect();
    let mut scope = rhai::Scope::new();
    for _ in 0..3 {
        if host.run("s.rhai", &mut scope, &inputs, &params).is_err() {
            break;
        }
    }
});
