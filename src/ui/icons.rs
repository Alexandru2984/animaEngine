//! Iconography — the live implementation of `docs/design-system.md` §5.
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

/// Where a CJK face is likely to live, most-preferred first.
///
/// Scanned rather than pulled in through a font-discovery crate: the list
/// is short, the paths are stable across the distributions this ships to,
/// and a fontconfig dependency would be a large tree for one lookup that
/// happens at most once per session.
///
/// `.ttc` entries are collections; face 0 is the Sans/SC face in every
/// Noto CJK collection, which covers kana and the kanji a UI needs.
const CJK_FONT_CANDIDATES: &[(&str, u32)] = &[
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
    (
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        0,
    ),
    (
        "/usr/share/fonts/opentype/noto/NotoSansCJKjp-Regular.otf",
        0,
    ),
    ("/usr/share/fonts/truetype/fonts-japanese-gothic.ttf", 0),
    ("/usr/share/fonts/opentype/ipafont-gothic/ipag.ttf", 0),
    (
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        0,
    ),
];

/// Upper bound on a font file we will read. Noto Sans CJK is ~19 MB, so
/// this is generous; it exists for the same reason every other loader here
/// is bounded, not because a realistic font approaches it.
const MAX_FONT_BYTES: u64 = 64 * 1024 * 1024;

/// Whether `code` is written in a script the bundled fonts cannot draw.
///
/// The bundled stack is Latin plus emoji, so every shipped locale renders
/// except the CJK ones. Kept as a prefix test so adding `zh` or `ko` later
/// needs no change here.
pub fn locale_needs_cjk(code: &str) -> bool {
    let lang = code.split(['-', '_']).next().unwrap_or(code);
    matches!(lang, "ja" | "zh" | "ko")
}

/// Load the first CJK face we can find, if any.
///
/// `None` means the machine has no CJK font installed — which is normal on
/// a system nobody reads those languages on, and is exactly when the
/// language picker should stop offering them.
fn load_cjk_font() -> Option<egui::FontData> {
    for (path, index) in CJK_FONT_CANDIDATES {
        let path = std::path::Path::new(path);
        let Ok(meta) = std::fs::metadata(path) else {
            continue;
        };
        if !meta.is_file() || meta.len() > MAX_FONT_BYTES {
            continue;
        }
        match std::fs::read(path) {
            Ok(bytes) => {
                tracing::info!("Loaded CJK font from {}", path.display());
                return Some(egui::FontData {
                    font: std::borrow::Cow::Owned(bytes),
                    index: *index,
                    tweak: egui::FontTweak::default(),
                });
            }
            Err(e) => tracing::debug!("CJK font {} unreadable: {e}", path.display()),
        }
    }
    None
}

/// Whether a CJK face is available on this machine.
///
/// The language picker uses this to avoid offering a language it would
/// draw as a row of empty boxes.
pub fn cjk_font_available() -> bool {
    CJK_FONT_CANDIDATES
        .iter()
        .any(|(p, _)| std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false))
}

/// Register the Phosphor icon font with `ctx`. Idempotent: calling
/// twice replaces the previous registration with the same data, which
/// is cheap. Invoked from `EguiRenderer::new` after `theme::apply`
/// so both the palette and the icon font are ready before the first
/// frame.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    // Give the proportional family the monospace face as a last-resort
    // fallback.
    //
    // egui's proportional stack is Ubuntu-Light → NotoEmoji-Regular →
    // emoji-icon-font, and **none** of the three carries the arrow block
    // (U+2190..U+2193). So "Appearance → Accessibility" in the what's-new
    // panel and the command palette's "↑↓ + Enter to pick" footer both
    // painted a missing-glyph box, while the same arrows rendered fine in
    // the keybindings chords — those use the monospace family, and the
    // bundled Hack face does have them.
    //
    // Appending the whole monospace list, rather than shipping a font for
    // four codepoints, keeps the binary the same size and also covers
    // anything else Ubuntu-Light happens to lack.
    let fallback = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    if let Some(proportional) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        for name in fallback {
            if !proportional.contains(&name) {
                proportional.push(name);
            }
        }
    }

    // CJK is not in the bundled stack at all, so a Japanese UI drew every
    // character as a missing-glyph box — the whole locale, not one symbol.
    // Loaded only when the active locale needs it: the face is ~19 MB, and
    // holding that resident for a user reading English buys nothing.
    // Re-installed on a language change, so switching at runtime works.
    if locale_needs_cjk(&crate::i18n::current_locale()) {
        match load_cjk_font() {
            Some(data) => {
                fonts.font_data.insert("cjk".to_owned(), data.into());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    if let Some(list) = fonts.families.get_mut(&family) {
                        list.push("cjk".to_owned());
                    }
                }
            }
            None => tracing::warn!(
                "No CJK font found; this locale will render as empty boxes. \
                 Install a Noto CJK package to fix it."
            ),
        }
    }

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
pub const PLUS: &str = ph::PLUS;
pub const COPY: &str = ph::COPY;
pub const RESET: &str = ph::ARROW_COUNTER_CLOCKWISE;
pub const BRING_FORWARD: &str = ph::ARROW_FAT_UP;
pub const SEND_BACKWARD: &str = ph::ARROW_FAT_DOWN;
pub const PLAY: &str = ph::PLAY;
pub const PAUSE: &str = ph::PAUSE;

// State / status (used inline with labels).
pub const HIDDEN: &str = ph::EYE_SLASH;
pub const GRAVITY: &str = ph::ARROW_FAT_DOWN; // physics on = pulled down

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
pub const BEHAVIOR_BOUNCE: &str = ph::ARROWS_DOWN_UP;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// The bundled stack is Latin plus emoji, so only the CJK locales need
    /// a font from outside it. Every other shipped language must not pay
    /// the ~19 MB load.
    #[test]
    fn only_cjk_locales_ask_for_an_extra_font() {
        for code in ["ja", "ja-JP", "ja_JP", "zh", "zh-CN", "ko"] {
            assert!(locale_needs_cjk(code), "{code} should need CJK");
        }
        for code in [
            "en", "en-US", "de", "es", "fr", "it", "nl", "pl", "pt-BR", "ro",
        ] {
            assert!(!locale_needs_cjk(code), "{code} should not need CJK");
        }
    }

    /// Both separators appear in the wild — Fluent uses `-`, POSIX `LANG`
    /// uses `_` — and getting this wrong silently reinstates the bug.
    #[test]
    fn the_language_subtag_is_read_past_either_separator() {
        assert!(locale_needs_cjk("ja-JP"));
        assert!(locale_needs_cjk("ja_JP"));
        assert!(!locale_needs_cjk("jamaican-english"));
    }

    /// Availability is a plain filesystem probe, so it must answer without
    /// panicking whatever the machine has installed.
    #[test]
    fn availability_is_safe_to_probe() {
        let _ = cjk_font_available();
    }
}
