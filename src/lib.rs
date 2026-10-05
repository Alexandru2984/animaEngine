#[cfg(unix)]
pub mod a11y;
pub mod anim;
pub mod animation;
pub mod announce;
pub mod app;
pub mod asset_library;
pub mod audio;
pub mod autostart;
pub mod away;
pub mod bedtime;
pub mod behavior;
pub mod bump;
pub mod chosen_apps;
pub mod clipboard;
pub mod config;
pub mod config_watch;
pub mod constants;
pub mod crash;
pub mod demo;
pub mod doze;
pub mod drop_validate;
pub mod effects;
pub mod entity;
pub mod error;
pub mod event;
pub mod file_chooser;
pub mod focus;
pub mod fullscreen;
pub mod gif_export;
pub mod greet;
pub mod group;
pub mod hotkeys;
pub mod hours;
pub mod i18n;
pub mod input;
pub mod keybindings;
pub mod monitor;
pub mod outcomes;
pub mod pacing;
pub mod perf;
pub mod phrases;
pub mod physics;
pub mod placeholders;
pub mod platforms;
#[cfg(unix)]
pub mod portal;
pub mod presets;
pub mod reminders;
pub mod renderer;
pub mod scene;
pub mod scene_file;
pub mod scene_thumb;
pub mod scenes;
pub mod schedule;
pub mod scripting;
pub mod shimeji;
pub mod sysload;
// The D-Bus single-instance handshake, the StatusNotifierItem tray and the
// native wlr-layer-shell path are unix-desktop-only (zbus / ksni /
// wayland-client, target-gated in Cargo.toml). The Windows equivalents —
// a named mutex and Shell_NotifyIcon — are `win_instance` and `win_tray`;
// both trays render the one menu in `tray_menu`.
#[cfg(unix)]
pub mod single_instance;
pub mod snooze;
pub mod soak;
pub mod sounds;
pub mod speech;
pub mod system_clipboard;
pub mod tap;
#[cfg(unix)]
pub mod tray;
pub mod tray_menu;
pub mod ui;
pub mod undo;
pub mod util;
#[cfg(unix)]
pub mod wayland;
pub mod welcome;
#[cfg(windows)]
pub mod win_instance;
#[cfg(windows)]
pub mod win_tray;
pub mod window;
#[cfg(unix)]
pub mod x11_clipboard;

pub use error::{AnimaError, Result};
