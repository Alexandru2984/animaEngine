#[cfg(unix)]
pub mod a11y;
pub mod anim;
pub mod animation;
pub mod app;
pub mod asset_library;
pub mod audio;
pub mod autostart;
pub mod away;
pub mod behavior;
pub mod config;
pub mod config_watch;
pub mod constants;
pub mod crash;
pub mod demo;
pub mod drop_validate;
pub mod entity;
pub mod error;
pub mod event;
pub mod file_chooser;
pub mod fullscreen;
pub mod group;
pub mod hotkeys;
pub mod i18n;
pub mod input;
pub mod keybindings;
pub mod monitor;
pub mod outcomes;
pub mod pacing;
pub mod perf;
pub mod physics;
pub mod platforms;
#[cfg(unix)]
pub mod portal;
pub mod presets;
pub mod reminders;
pub mod renderer;
pub mod scene;
pub mod scenes;
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
pub mod soak;
pub mod speech;
#[cfg(unix)]
pub mod tray;
pub mod tray_menu;
pub mod ui;
pub mod undo;
pub mod util;
#[cfg(unix)]
pub mod wayland;
#[cfg(windows)]
pub mod win_instance;
#[cfg(windows)]
pub mod win_tray;
pub mod window;

pub use error::{AnimaError, Result};
