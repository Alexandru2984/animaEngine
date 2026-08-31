// `linux`, `x11_input` and `x11_windows` speak X11 through x11rb, which is
// target-gated to unix in Cargo.toml. `overlay` (the OverlayPlatform seam)
// and `platform` (display-server detection) stay portable — they are what a
// Windows or macOS backend plugs into.
#[cfg(unix)]
pub mod linux;
pub mod overlay;
pub mod platform;
#[cfg(windows)]
pub mod win_overlay;
#[cfg(unix)]
pub mod x11_input;
#[cfg(unix)]
pub mod x11_windows;
