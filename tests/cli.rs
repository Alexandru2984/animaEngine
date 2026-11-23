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

/// `--recover`, from the built binary, in a home of its own: it never
/// reaches the real config. Linux only — where `directories` follows the
/// XDG variables set here.
#[cfg(target_os = "linux")]
mod recover {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    struct Home(PathBuf);

    impl Home {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("anima-cli-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn config(&self) -> PathBuf {
            self.0.join("config/animaengine/config.toml")
        }
        fn snapshot(&self) -> PathBuf {
            self.0.join("cache/animaengine/crash-recovery.toml")
        }
        fn write(path: &Path, text: &str) {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        fn recover(&self) -> std::process::Output {
            Command::new(env!("CARGO_BIN_EXE_anima_engine"))
                .arg("--recover")
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .env("HOME", &self.0)
                .env("XDG_CONFIG_HOME", self.0.join("config"))
                .env("XDG_CACHE_HOME", self.0.join("cache"))
                .env("XDG_DATA_HOME", self.0.join("data"))
                .output()
                .expect("run anima_engine --recover")
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn config_named(name: &str) -> String {
        let mut config = anima_engine::config::AppConfig::default();
        config.characters[0].name = name.into();
        config.to_toml_string().unwrap()
    }

    #[test]
    fn with_no_snapshot_nothing_is_done() {
        let home = Home::new("none");
        let out = home.recover();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains("No crash-recovery snapshot"));
        assert!(!home.config().exists(), "no config made");
    }

    /// The snapshot goes in, the config it replaces is kept beside as
    /// `.bak` — the user's alone — and the snapshot is gone.
    #[test]
    fn a_snapshot_is_restored_and_the_old_config_kept() {
        use std::os::unix::fs::PermissionsExt;
        let home = Home::new("restore");
        let before = config_named("Before");
        let snapshot = config_named("Recovered");
        Home::write(&home.config(), &before);
        Home::write(&home.snapshot(), &snapshot);
        let out = home.recover();
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{said}");
        assert!(said.contains("Snapshot restored"), "{said}");
        assert_eq!(std::fs::read_to_string(home.config()).unwrap(), snapshot);
        let bak = home.config().with_extension("toml.bak");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), before);
        let mode = std::fs::metadata(&bak).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the old config readable by the user alone");
        assert!(!home.snapshot().exists(), "used up");
    }

    /// A snapshot that is no config is refused, and nothing is touched.
    #[test]
    fn a_damaged_snapshot_is_refused_and_nothing_touched() {
        let home = Home::new("damaged");
        let before = config_named("Before");
        Home::write(&home.config(), &before);
        Home::write(&home.snapshot(), "version = 2\n[[[ not toml");
        let out = home.recover();
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("Recovery failed"));
        assert_eq!(std::fs::read_to_string(home.config()).unwrap(), before);
        assert!(home.snapshot().exists(), "kept for another try");
    }

    /// A snapshot holding what a newer version wrote — kept aside, as the
    /// app does — is a config, and comes back (BUG-011).
    #[test]
    fn a_snapshot_with_a_newer_versions_setting_is_restored() {
        let home = Home::new("newer");
        let snapshot: String = config_named("Recovered")
            .lines()
            .map(|l| {
                if l.starts_with("theme = ") {
                    "theme = \"future_theme_v9\"\n".to_string()
                } else {
                    format!("{l}\n")
                }
            })
            .collect();
        assert!(snapshot.contains("future_theme_v9"), "{snapshot}");
        Home::write(&home.snapshot(), &snapshot);
        let out = home.recover();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(std::fs::read_to_string(home.config())
            .unwrap()
            .contains("future_theme_v9"));
    }
}
