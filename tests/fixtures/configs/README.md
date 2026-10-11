# Configs written by released versions

Each file here was written by a released animaEngine itself, not by
hand, and `config::upgrade_tests` checks that this version reads every
value in it the same. A test that fails here means a setting someone
has would change meaning, or be lost, on upgrading.

- `v1.0.0-default.toml` — what 1.0.0 writes on a first run.
- `v1.4.0-default.toml` — the same for 1.4.0. 1.1.0, 1.2.1 and 1.3.0
  write byte for byte the same file, so it stands for them too.
- `v1.4.0-full.toml` — every 1.4.0 setting changed from its default:
  each behavior and asset type, a window, a group, rebound and unbound
  shortcuts, the collapse state. Written by 1.4.0 reading a hand-made
  file and saving it back, so 1.4.0 knows every key in it.

## Adding a release

At each release, add its `default` (and a `full` one when the release
added settings) the same way: from the tag, not from the working tree.

```sh
git archive v1.5.0 | tar -x -C /var/tmp/anima-v1.5.0
# examples/dump_config.rs in that tree:
#   use anima_engine::config::AppConfig;
#   fn main() {
#       match std::env::args().nth(1).as_deref() {
#           Some("default") => print!("{}", toml::to_string_pretty(&AppConfig::default()).unwrap()),
#           // Reads $XDG_CONFIG_HOME/animaengine/config.toml as the app
#           // does, and prints what it would save back.
#           Some("read") => print!("{}", toml::to_string_pretty(&AppConfig::try_reload().unwrap()).unwrap()),
#           _ => std::process::exit(2),
#       }
#   }
cd /var/tmp/anima-v1.5.0
cargo build --no-default-features --example dump_config
env XDG_CONFIG_HOME=/var/tmp/anima-v1.5.0/cfg target/debug/examples/dump_config default > v1.5.0-default.toml
```

Run it with `XDG_CONFIG_HOME` pointing at a throwaway directory, so a
`read` never touches your own config.
