//! Egui paint integration on the native Wayland path (E.4).
//!
//! The X11 path uses `egui_winit::State` to bridge winit events into
//! egui. We don't have a winit window here — the Wayland queue
//! produces sctk events that the layer-window translator has already
//! mapped into `Vec<egui::Event>`. Building the `egui::RawInput`
//! manually from that vector is straightforward; the rest is plain
//! `egui::Context` + `egui_wgpu::Renderer` wiring.
//!
//! Per-frame protocol:
//!
//! 1. Caller drains pointer/keyboard events from `LayerWindow`.
//! 2. Caller invokes [`WaylandEguiRenderer::render`] with the events,
//!    surface size, and a UI closure.
//! 3. The renderer builds `RawInput`, runs egui, tessellates the
//!    output, paints over the wgpu surface with `LoadOp::Load` so the
//!    sprites underneath survive.
//!
//! HiDPI is intentionally pegged at 1× for now (compositor scale
//! plumbing is part of E.7's multi-monitor work; E.9 picks up the
//! polish).

use crate::ui::theme;

/// Single-window egui integration for the native Wayland path. Mirrors
/// `crate::ui::EguiRenderer` but skips the `egui_winit::State`: the frame
/// itself is `SurfaceEgui`'s, and this adds what the panel's surface has
/// that other monitors' do not — a screen reader and an input method.
pub struct WaylandEguiRenderer {
    surface: crate::ui::surface_egui::SurfaceEgui,
    /// Caret of the focused text field in the last frame, for the input
    /// method; see [`WaylandEguiRenderer::ime_caret`].
    ime_caret: Option<(i32, i32, i32, i32)>,
    /// Screen readers: this surface's tree on AT-SPI (`crate::a11y`).
    screen_reader: crate::a11y::ScreenReaderBridge,
    /// The Appearance setting that allows the tree at all.
    accesskit_allowed: bool,
    /// Set from the bridge's thread when a screen reader asks for
    /// something; the loop draws a frame to answer it.
    screen_reader_wake: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl WaylandEguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        theme: theme::Theme,
    ) -> Self {
        let screen_reader_wake = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let wake = std::sync::Arc::clone(&screen_reader_wake);
        Self {
            surface: crate::ui::surface_egui::SurfaceEgui::new(device, output_format, theme),
            ime_caret: None,
            // The loop wakes every frame interval, but draws only for a
            // reason; a reader's request is one.
            screen_reader: crate::a11y::ScreenReaderBridge::new(move || {
                wake.store(true, std::sync::atomic::Ordering::Release);
            }),
            accesskit_allowed: true,
            screen_reader_wake,
        }
    }

    fn context(&self) -> &egui::Context {
        self.surface.context()
    }

    /// Whether an input is under way that an edit is still coming from:
    /// a pointer button held, a text field focused, a list or the palette
    /// open. Holds an undo step open (`crate::undo`).
    pub fn input_in_progress(&self) -> bool {
        self.context().input(|i| i.pointer.any_down()) || self.wants_keyboard()
    }

    /// Whether a screen reader has requests waiting for the next frame.
    pub fn has_screen_reader_requests(&self) -> bool {
        self.screen_reader.has_requests()
    }

    /// Whether a screen reader asked for something since the last call.
    pub fn screen_reader_woke(&self) -> bool {
        self.screen_reader_wake
            .swap(false, std::sync::atomic::Ordering::AcqRel)
    }

    /// Whether egui asked to be run again by `now`.
    pub fn repaint_due(&self, now: std::time::Instant) -> bool {
        self.surface.repaint_due(now)
    }

    /// Follow the Appearance setting that allows screen readers the tree.
    /// Applies from the next frame.
    pub fn set_accesskit_allowed(&mut self, allowed: bool) {
        self.accesskit_allowed = allowed;
    }

    /// Tell the screen reader whether the surface has the keyboard, and
    /// where it is: `origin` is its output's position in the layout.
    /// Unchanged values send nothing.
    pub fn set_window_state(&mut self, focused: bool, origin: (f32, f32), size: [u32; 2]) {
        self.screen_reader.set_focused(focused);
        self.screen_reader.set_bounds(
            f64::from(origin.0),
            f64::from(origin.1),
            f64::from(size[0]),
            f64::from(size[1]),
        );
    }

    /// Whether egui owns the pointer right now, i.e. it is over a panel,
    /// window or popup rather than the bare overlay.
    ///
    /// The answer comes from the last completed frame, which is what any
    /// immediate-mode integration has to work with — the winit path gets
    /// the same guarantee from `egui_winit`'s "was this event consumed"
    /// return value. Panels do not move between frames, so it holds.
    pub fn owns_pointer(&self) -> bool {
        self.surface.owns_pointer()
    }

    /// The caret of the focused text field, from the last frame — `None`
    /// when no field wants text. Drives the input method (`text_input`).
    pub fn ime_caret(&self) -> Option<(i32, i32, i32, i32)> {
        self.ime_caret
    }

    /// Whether egui is collecting text right now — a focused text field,
    /// the keybinding capture widget — or holds the keyboard anyway: the
    /// command palette or a list is open (`panels::keyboard_held`).
    ///
    /// Callers must consult this **before** running a key through the
    /// keybinding table. The winit path gets the same answer from
    /// `EguiRenderer::handle_event` reporting the key as consumed; this
    /// loop reads egui's raw event list itself and so has to ask
    /// explicitly.
    pub fn wants_keyboard(&self) -> bool {
        self.context().wants_keyboard_input() || crate::ui::panels::keyboard_held(self.context())
    }

    /// Re-apply the design-system style if the active theme changed.
    pub fn ensure_theme(&mut self, theme: theme::Theme) {
        self.surface.ensure_theme(theme);
    }

    /// Run one egui frame on top of an already-rendered surface.
    ///
    /// `events` is consumed in place — every drained event from the
    /// layer-window's translator goes in unchanged. `size_in_pixels`
    /// must match the surface's current dimensions; `pixels_per_point`
    /// is the compositor's reported scale (1.0 on single-DPI displays,
    /// 2.0 on most "Retina"-grade panels, etc.). Egui's layout snaps
    /// to this scale so glyphs stay crisp.
    #[allow(clippy::too_many_arguments)]
    pub fn render<F>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        size_in_pixels: [u32; 2],
        pixels_per_point: f32,
        mut events: Vec<egui::Event>,
        modifiers: egui::Modifiers,
        build_ui: F,
    ) where
        F: FnMut(&egui::Context),
    {
        // egui answers `input.modifiers` from `RawInput`, not from the
        // modifiers carried on individual key events; hardcoding them to
        // `default()` once killed every modifier-gated interaction on this
        // path. See `effective_modifiers`.
        let modifiers = effective_modifiers(&events, modifiers);
        crate::a11y::sync_egui(
            self.surface.context(),
            self.accesskit_allowed,
            &self.screen_reader,
        );
        events.extend(self.screen_reader.drain_requests());
        let mut platform_output = self.surface.render(
            device,
            queue,
            view,
            size_in_pixels,
            pixels_per_point,
            events,
            modifiers,
            build_ui,
        );
        self.screen_reader.publish(
            self.accesskit_allowed,
            platform_output.accesskit_update.take(),
        );
        self.ime_caret = platform_output.ime.map(|ime| {
            let r = ime.cursor_rect;
            (
                r.min.x.round() as i32,
                r.min.y.round() as i32,
                r.width().round().max(1.0) as i32,
                r.height().round().max(1.0) as i32,
            )
        });
    }
}

/// Pick the modifier state to report to egui for one frame.
///
/// `live` is the seat state sampled when the frame's events were drained.
/// That is the right answer for a modifier held across frames — Ctrl held
/// down while clicking produces no key event at all in the frames between —
/// but the wrong one when a whole chord lands inside a single frame: by
/// drain time the modifier is already released, so the shortcut reads as
/// unmodified and silently does nothing. One long frame is enough to hit
/// this, so a video or GIF decode stall can swallow shortcuts at random.
///
/// Each key event carries the modifier snapshot taken when it arrived, so
/// the newest of those wins when the batch has one.
fn effective_modifiers(events: &[egui::Event], live: egui::Modifiers) -> egui::Modifiers {
    events
        .iter()
        .rev()
        .find_map(|e| match e {
            egui::Event::Key { modifiers, .. } => Some(*modifiers),
            _ => None,
        })
        .unwrap_or(live)
}

#[cfg(test)]
mod tests {
    use super::effective_modifiers;

    fn key(k: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn empty_batch_keeps_live_state() {
        let live = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        assert_eq!(effective_modifiers(&[], live), live);
    }

    #[test]
    fn non_key_events_keep_live_state() {
        let live = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        let events = [egui::Event::Text("a".into())];
        assert_eq!(effective_modifiers(&events, live), live);
    }

    /// The regression this exists for: the chord arrived and was gone again
    /// before the frame drained, so the live state is bare and only the
    /// event still remembers that Ctrl was down.
    #[test]
    fn chord_inside_one_frame_beats_stale_live_state() {
        let held = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let events = [key(egui::Key::K, held)];
        let got = effective_modifiers(&events, egui::Modifiers::default());
        assert!(got.command, "Ctrl+K must still read as a command chord");
    }

    #[test]
    fn newest_key_event_wins() {
        let with_ctrl = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let events = [
            key(egui::Key::K, with_ctrl),
            key(egui::Key::A, egui::Modifiers::default()),
        ];
        let got = effective_modifiers(&events, with_ctrl);
        assert!(
            !got.command,
            "the later unmodified key is the current truth"
        );
    }
}
