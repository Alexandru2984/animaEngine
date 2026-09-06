pub mod anim;
pub mod animation;
pub mod app;
pub mod asset_library;
pub mod behavior;
pub mod config;
pub mod constants;
pub mod crash;
pub mod demo;
pub mod drop_validate;
pub mod entity;
pub mod error;
pub mod event;
pub mod group;
pub mod hotkeys;
pub mod i18n;
pub mod input;
pub mod keybindings;
pub mod monitor;
pub mod perf;
pub mod physics;
pub mod platforms;
pub mod presets;
pub mod renderer;
pub mod scene;
pub mod shimeji;
// The D-Bus single-instance handshake, the StatusNotifierItem tray and the
// native wlr-layer-shell path are unix-desktop-only (zbus / ksni /
// wayland-client, target-gated in Cargo.toml). The Windows equivalents —
// named mutex and Shell_NotifyIcon — arrive with the Windows backend (C4).
#[cfg(unix)]
pub mod single_instance;
pub mod soak;
#[cfg(unix)]
pub mod tray;
pub mod ui;
pub mod util;
#[cfg(unix)]
pub mod wayland;
pub mod window;

pub use error::{AnimaError, Result};
