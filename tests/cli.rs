//! The command line's answers, from the built binary.
//!
//! `--version` is what packagers' smoke checks and bug reports reach for
//! first, so it answers on stdout with one line and nothing else. During
//! 1.5 a log line about a laptop's GPUs came before it there, on the
//! machine it was written on, and only there — this runs the binary
//! itself, wherever the tests run.

use std::process::Command;

fn run(arg: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_anima_engine"))
        .arg(arg)
        // Nothing here may reach the real session or config.
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env("RUST_LOG", "info")
        .output()
        .expect("run anima_engine")
}

#[test]
fn version_is_one_line_on_stdout() {
    let out = run("--version");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        format!("anima-engine {}\n", env!("CARGO_PKG_VERSION")),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn help_and_an_unknown_flag_write_no_log() {
    for arg in ["--help", "--no-such-flag"] {
        let out = run(arg);
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(all.contains("USAGE"), "{arg}: {all}");
        assert!(!all.contains(" INFO "), "{arg} logged: {all}");
    }
}
