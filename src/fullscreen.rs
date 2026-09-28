//! Stepping aside for full-screen applications.
//!
//! A game, a video or a presentation filling the screen is no place for
//! characters walking across it. On wlroots compositors the overlay's
//! layer sits *above* full-screen windows, so they did exactly that; on
//! X11 some window managers keep the overlay's always-on-top above them
//! too. Where the window manager does cover the overlay, it went on
//! animating unseen.
//!
//! Each backend reports whether the window in front is full screen — the
//! X11 path from EWMH (`window::x11_windows::FullscreenWatch`), native
//! Wayland from `wlr-foreign-toplevel-management` — and this decides what
//! the overlay does about it. The result is kept apart from the user's
//! own hide and pause, and recomputed every frame, so when the
//! full-screen app goes, the overlay returns to exactly what the user
//! left it as.

use serde::{Deserialize, Serialize};

/// What the overlay does while a full-screen app is in front. Set in
/// Appearance; `[global] on_fullscreen` in the config.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnFullscreen {
    /// Hide the characters, and hold them still while hidden.
    #[default]
    Hide,
    /// Keep them in view, but still.
    Pause,
    /// Carry on as usual.
    Ignore,
}

impl OnFullscreen {
    pub const ALL: [Self; 3] = [Self::Hide, Self::Pause, Self::Ignore];

    /// Stable i18n key for the Appearance picker.
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Hide => "fullscreen-hide",
            Self::Pause => "fullscreen-pause",
            Self::Ignore => "fullscreen-ignore",
        }
    }
}

/// What stepping aside asks of the overlay right now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StepAside {
    /// Draw nothing — no characters, no ⚙ corner.
    pub hidden: bool,
    /// Hold the scene still (`Scene::set_suspended`).
    pub paused: bool,
}

/// Decide, from the setting and whether a full-screen app is in front.
/// Never while editing: in edit mode the overlay is the window in use.
pub fn step_aside(setting: OnFullscreen, fullscreen_in_front: bool, edit_mode: bool) -> StepAside {
    if !fullscreen_in_front || edit_mode {
        return StepAside::default();
    }
    match setting {
        OnFullscreen::Hide => StepAside {
            hidden: true,
            paused: true,
        },
        OnFullscreen::Pause => StepAside {
            hidden: false,
            paused: true,
        },
        OnFullscreen::Ignore => StepAside::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_changes_without_a_full_screen_app() {
        for setting in OnFullscreen::ALL {
            assert_eq!(step_aside(setting, false, false), StepAside::default());
        }
    }

    #[test]
    fn each_setting_steps_aside_its_own_way() {
        assert_eq!(
            step_aside(OnFullscreen::Hide, true, false),
            StepAside {
                hidden: true,
                paused: true
            }
        );
        assert_eq!(
            step_aside(OnFullscreen::Pause, true, false),
            StepAside {
                hidden: false,
                paused: true
            }
        );
        assert_eq!(
            step_aside(OnFullscreen::Ignore, true, false),
            StepAside::default()
        );
    }

    #[test]
    fn never_while_editing() {
        for setting in OnFullscreen::ALL {
            assert_eq!(step_aside(setting, true, true), StepAside::default());
        }
    }

    #[test]
    fn hide_is_the_default() {
        assert_eq!(OnFullscreen::default(), OnFullscreen::Hide);
    }
}
