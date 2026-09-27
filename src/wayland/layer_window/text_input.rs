//! Input methods (fcitx5, ibus, …) through `zwp_text_input_v3` (C8).
//!
//! On the X11 path winit and `egui_winit` speak to input methods; the
//! native Wayland loop had nothing, so dead keys and accented letters
//! worked (xkbcommon composes those) but Chinese, Japanese or Korean
//! input could not reach a text field at all. An input method on Wayland
//! talks to the compositor, which relays to the focused client's text
//! input — this is that client side.
//!
//! The text input is enabled only while egui has a text field focused —
//! the frame's `IMEOutput` says so, and where its caret is, which the
//! compositor passes on so the candidate window sits by the text.
//!
//! Composition reaches egui as `ImeEvent`s. `Enabled` goes out only when
//! the input method actually starts composing, not when a field gains
//! focus: while egui's text field believes an IME is active it drops
//! Backspace, the arrow keys and key repeat, so an `Enabled` sent with no
//! input method running would break ordinary typing.

use super::state::WaylandState;
use wayland_client::{protocol::wl_seat, Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{self, ContentHint, ContentPurpose, ZwpTextInputV3},
};

/// Caret rectangle in the primary surface's logical pixels.
pub type CaretRect = (i32, i32, i32, i32);

/// Per-seat text-input state.
#[derive(Default)]
pub struct Ime {
    /// Bound when the compositor offers `zwp_text_input_manager_v3`;
    /// without it there is no IME, and nothing else changes.
    manager: Option<ZwpTextInputManagerV3>,
    input: Option<ZwpTextInputV3>,
    /// Between the compositor's `enter` and `leave`: our surface has the
    /// seat's keyboard focus, so an `enable` would take effect.
    entered: bool,
    /// Whether `enable` has been committed since the last `enter`.
    enabled: bool,
    /// Last caret rectangle sent.
    caret: Option<CaretRect>,
    /// egui was told `Enabled` and not yet `Disabled` or `Commit`.
    composing: bool,
    /// Double-buffered until `done`, as the protocol requires.
    pending_preedit: Option<String>,
    pending_commit: Option<String>,
}

impl Ime {
    pub fn new(manager: Option<ZwpTextInputManagerV3>) -> Self {
        Self {
            manager,
            ..Self::default()
        }
    }

    /// Create the seat's text input, once, when its keyboard appears.
    pub fn attach_seat(&mut self, seat: &wl_seat::WlSeat, qh: &QueueHandle<WaylandState>) {
        if self.input.is_some() {
            return;
        }
        if let Some(manager) = &self.manager {
            self.input = Some(manager.get_text_input(seat, qh, ()));
        }
    }

    /// Bring the text input in line with this frame: enabled with the
    /// caret where egui drew it while a field wants text, disabled
    /// otherwise. Requests are sent only on change. A composition cut
    /// short by losing the field is closed in egui through `events`.
    pub fn sync(&mut self, caret: Option<CaretRect>, events: &mut Vec<egui::Event>) {
        let Some(input) = &self.input else {
            return;
        };
        match caret {
            Some(rect) if self.entered => {
                if !self.enabled {
                    // Disable first. After a `leave` and a new `enter` —
                    // focus away and back, or the input method restarting
                    // — wlroots-based compositors still hold the text
                    // input as enabled, so a bare `enable` is no change
                    // and the input method is never activated: input
                    // methods silently stopped working until the field
                    // was refocused. A disable before it makes it one.
                    input.disable();
                    input.commit();
                    input.enable();
                    input.set_content_type(ContentHint::None, ContentPurpose::Normal);
                    input.set_cursor_rectangle(rect.0, rect.1, rect.2, rect.3);
                    input.commit();
                    self.enabled = true;
                    self.caret = Some(rect);
                    tracing::debug!("IME: text input enabled, caret {rect:?}");
                } else if self.caret != Some(rect) {
                    input.set_cursor_rectangle(rect.0, rect.1, rect.2, rect.3);
                    input.commit();
                    self.caret = Some(rect);
                }
            }
            _ => {
                if self.enabled {
                    input.disable();
                    input.commit();
                    self.enabled = false;
                    self.caret = None;
                    tracing::debug!("IME: text input disabled");
                }
                self.end_composition(events);
            }
        }
    }

    /// Close an open composition in egui: clear the preedit, then disable.
    fn end_composition(&mut self, events: &mut Vec<egui::Event>) {
        if self.composing {
            events.push(egui::Event::Ime(egui::ImeEvent::Preedit(String::new())));
            events.push(egui::Event::Ime(egui::ImeEvent::Disabled));
            self.composing = false;
        }
    }

    /// Apply one `done`: the protocol's order is the old preedit out, the
    /// commit in, the new preedit shown. A preedit not re-sent before
    /// `done` is cleared.
    fn apply(&mut self, events: &mut Vec<egui::Event>) {
        let commit = self.pending_commit.take().unwrap_or_default();
        let preedit = self.pending_preedit.take().unwrap_or_default();
        for event in ime_events(&mut self.composing, &commit, &preedit) {
            events.push(event);
        }
    }
}

/// The egui events for one `done`, given whether a composition is open;
/// updates `composing`. Separate from the protocol plumbing so the rules
/// can be tested.
fn ime_events(composing: &mut bool, commit: &str, preedit: &str) -> Vec<egui::Event> {
    use egui::{Event::Ime, ImeEvent};
    let mut out = Vec::new();
    if !commit.is_empty() {
        // egui only inserts a commit at the caret it recorded on
        // `Enabled` or the last preedit, so a commit with no composition
        // before it (an input method that commits directly) opens one.
        if !*composing {
            out.push(Ime(ImeEvent::Enabled));
        }
        // The commit replaces the preedit egui is showing, and egui
        // leaves its IME mode on a commit by itself.
        out.push(Ime(ImeEvent::Commit(commit.to_owned())));
        *composing = false;
    }
    if !preedit.is_empty() {
        if !*composing {
            out.push(Ime(ImeEvent::Enabled));
            *composing = true;
        }
        out.push(Ime(ImeEvent::Preedit(preedit.to_owned())));
    } else if *composing {
        // Composition cancelled (Escape, Backspace over the last letter).
        out.push(Ime(ImeEvent::Preedit(String::new())));
        out.push(Ime(ImeEvent::Disabled));
        *composing = false;
    }
    out
}

impl Dispatch<ZwpTextInputManagerV3, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpTextInputManagerV3,
        _event: <ZwpTextInputManagerV3 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The manager has no events.
    }
}

impl Dispatch<ZwpTextInputV3, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        use zwp_text_input_v3::Event;
        let ime = &mut state.ime;
        match event {
            Event::Enter { .. } => {
                tracing::debug!("IME: text input focus entered");
                // Focus arrived: the next frame's sync re-enables if a
                // field still wants text.
                ime.entered = true;
                ime.enabled = false;
            }
            Event::Leave { .. } => {
                tracing::debug!("IME: text input focus left");
                // Leaving disables the text input on the compositor's
                // side; close whatever composition egui was showing.
                ime.entered = false;
                ime.enabled = false;
                ime.caret = None;
                ime.end_composition(&mut state.pending_egui_events);
            }
            Event::PreeditString { text, .. } => {
                ime.pending_preedit = Some(text.unwrap_or_default());
            }
            Event::CommitString { text } => {
                ime.pending_commit = Some(text.unwrap_or_default());
            }
            Event::Done { .. } => {
                tracing::debug!(
                    "IME: done, preedit {:?}, commit {:?}",
                    ime.pending_preedit,
                    ime.pending_commit
                );
                ime.apply(&mut state.pending_egui_events);
            }
            // No surrounding text is ever sent, so there is none to
            // delete; the remaining events carry nothing we act on.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event::Ime, ImeEvent};

    fn run(composing: &mut bool, commit: &str, preedit: &str) -> Vec<egui::Event> {
        ime_events(composing, commit, preedit)
    }

    /// Typing with an input method: a growing preedit, then the commit.
    #[test]
    fn a_composition_opens_updates_and_commits() {
        let mut composing = false;
        assert_eq!(
            run(&mut composing, "", "ni"),
            vec![Ime(ImeEvent::Enabled), Ime(ImeEvent::Preedit("ni".into()))]
        );
        assert_eq!(
            run(&mut composing, "", "nih"),
            vec![Ime(ImeEvent::Preedit("nih".into()))]
        );
        assert_eq!(
            run(&mut composing, "你好", ""),
            vec![Ime(ImeEvent::Commit("你好".into()))]
        );
        assert!(!composing);
    }

    /// An input method that commits without composing first still gets
    /// its text in: egui needs `Enabled` to know where the caret is.
    #[test]
    fn a_direct_commit_is_preceded_by_enabled() {
        let mut composing = false;
        assert_eq!(
            run(&mut composing, "é", ""),
            vec![Ime(ImeEvent::Enabled), Ime(ImeEvent::Commit("é".into()))]
        );
        assert!(!composing);
    }

    /// A cancelled composition clears the preedit and leaves IME mode,
    /// so Backspace and the arrow keys work again.
    #[test]
    fn a_cancelled_composition_disables() {
        let mut composing = true;
        assert_eq!(
            run(&mut composing, "", ""),
            vec![
                Ime(ImeEvent::Preedit(String::new())),
                Ime(ImeEvent::Disabled)
            ]
        );
        assert!(!composing);
    }

    /// Nothing composing and nothing sent: egui hears nothing, and in
    /// particular no `Enabled` — ordinary typing must stay untouched.
    #[test]
    fn an_empty_done_outside_a_composition_is_silent() {
        let mut composing = false;
        assert!(run(&mut composing, "", "").is_empty());
    }

    /// Commit and a new preedit in the same `done`: a word is committed
    /// and the next one starts.
    #[test]
    fn a_commit_can_start_the_next_composition() {
        let mut composing = true;
        assert_eq!(
            run(&mut composing, "日本", "ご"),
            vec![
                Ime(ImeEvent::Commit("日本".into())),
                Ime(ImeEvent::Enabled),
                Ime(ImeEvent::Preedit("ご".into()))
            ]
        );
        assert!(composing);
    }
}
