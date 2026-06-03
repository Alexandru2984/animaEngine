//! Iconography — the live implementation of [`docs/design-system.md`] §5.
//!
//! Two exports matter:
//!
//! - [`install`] — registers the Phosphor icon font with an `egui::Context`,
//!   merged into the proportional family so any `Label` / `Button` can
//!   embed a glyph inline with body text.
//! - The `pub const` re-exports — every icon the UI uses goes through a
//!   named constant in this module, not a raw Phosphor identifier. That
//!   way the icon set stays auditable: `grep` for `icons::TRASH` shows
//!   every "delete" affordance in one shot.
//!
//! Phosphor's regular weight is our baseline (§5 of the design system).
//! When a heavier weight is wanted for emphasis (e.g. a destructive
//! confirm button), wrap the glyph in `RichText::new(...).strong()` —
//! egui will fake-bold the proportional font rather than swapping to a
//! second variant, keeping the binary slim.

use egui_phosphor::regular as ph;

/// Register the Phosphor icon font with `ctx`. Idempotent: calling
/// twice replaces the previous registration with the same data, which
/// is cheap. Invoked from [`EguiRenderer::new`] after `theme::apply`
/// so both the palette and the icon font are ready before the first
/// frame.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

// ─── named glyphs ────────────────────────────────────────────────────
//
// Grouped by domain so a contributor adding an action knows which
// section to extend. Each constant is just a `&'static str` carrying
// the Phosphor Private-Use codepoint — egui paints it as a glyph when
// the font is installed.

// Action / verbs (used in buttons, menus, toolbars).
pub const TRASH: &str = ph::TRASH;
pub const COPY: &str = ph::COPY;
pub const RESET: &str = ph::ARROW_COUNTER_CLOCKWISE;
pub const BRING_FORWARD: &str = ph::ARROW_FAT_UP;
pub const SEND_BACKWARD: &str = ph::ARROW_FAT_DOWN;
pub const PLAY: &str = ph::PLAY;
pub const PAUSE: &str = ph::PAUSE;

// State / status (used inline with labels).
pub const VISIBLE: &str = ph::EYE;
pub const HIDDEN: &str = ph::EYE_SLASH;
pub const GRAVITY: &str = ph::ARROW_FAT_DOWN; // physics on = pulled down
pub const NO_GRAVITY: &str = ph::CIRCLE;

// Severity (toasts, badges).
pub const SUCCESS: &str = ph::CHECK_CIRCLE;
pub const WARN: &str = ph::WARNING;
pub const ERROR: &str = ph::X_CIRCLE;
pub const INFO: &str = ph::INFO;

// Theme / appearance.
pub const DARK_MODE: &str = ph::MOON;
pub const LIGHT_MODE: &str = ph::SUN;
pub const PALETTE: &str = ph::PALETTE;

// Toggle button + chrome.
pub const SETTINGS: &str = ph::GEAR_SIX;
pub const CURSOR: &str = ph::CURSOR;
pub const GHOST: &str = ph::GHOST;
pub const STACK: &str = ph::STACK;
pub const KEYBOARD: &str = ph::KEYBOARD;
pub const CLOSE: &str = ph::X;
pub const HINT: &str = ph::LIGHTBULB;

// Behaviors (used in the behavior picker).
pub const BEHAVIOR_IDLE: &str = ph::PERSON_SIMPLE;
pub const BEHAVIOR_WALK: &str = ph::FOOTPRINTS;
pub const BEHAVIOR_FOLLOW: &str = ph::CURSOR_CLICK;
pub const BEHAVIOR_WANDER: &str = ph::ARROWS_OUT_CARDINAL;

// Presets (used in the Scene tab preset gallery).
pub const HEART: &str = ph::HEART;
pub const FLAME: &str = ph::FLAME;
pub const CONFETTI: &str = ph::CONFETTI;
pub const SPARKLE: &str = ph::SPARKLE;

// Library
pub const LIBRARY: &str = ph::FOLDER;
pub const SEARCH: &str = ph::MAGNIFYING_GLASS;
pub const KIND_IMAGE: &str = ph::IMAGE;
pub const KIND_ANIMATED: &str = ph::FILM_REEL;
pub const KIND_VIDEO: &str = ph::FILM_SCRIPT;
pub const ADD: &str = ph::PLUS;
