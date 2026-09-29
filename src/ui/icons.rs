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

/// Whether `text` has characters only the CJK face can draw: the CJK,
/// kana and Hangul blocks, and the full-width forms.
pub fn text_needs_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(u32::from(c),
            0x2E80..=0x9FFF      // radicals, kana, CJK symbols, unified ideographs
            | 0xAC00..=0xD7AF    // Hangul syllables
            | 0xF900..=0xFAFF    // compatibility ideographs
            | 0xFF00..=0xFFEF    // half- and full-width forms
            | 0x2_0000..=0x3_134F) // the supplementary ideograph planes
    })
}

/// Load the CJK face when an input method is composing text that needs
/// it. Call every frame; cheap until it matters.
///
/// A composition in Chinese, Japanese or Korean is as deliberate as
/// opening the language picker (R33), and without the face the text
/// arrives but reads as empty boxes. Typing accented Latin through an
/// input method pays nothing: the check is on the characters, not on the
/// input method being active.
pub fn load_cjk_for_ime(ctx: &egui::Context) {
    let composing_cjk = ctx.input(|i| {
        i.events.iter().any(|e| match e {
            egui::Event::Ime(egui::ImeEvent::Preedit(t) | egui::ImeEvent::Commit(t)) => {
                text_needs_cjk(t)
            }
            _ => false,
        })
    });
    if composing_cjk {
        install_with_cjk(ctx);
    }
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
/// Install the fonts, forcing the CJK face in even when the active
/// locale does not need it.
///
/// For the language picker. Its entry for Japanese is labelled `日本語`,
/// and the face is normally loaded only once Japanese is *already*
/// active — so the one row a Japanese reader has to find in order to
/// switch was the one row drawn as empty boxes (R33). Opening the
/// dropdown is a deliberate act, so paying the ~19 MB there is fine;
/// what R23 wanted to avoid was paying it for someone reading English
/// who never opens it.
pub fn install_with_cjk(ctx: &egui::Context) {
    // Cheap re-entry guard. The caller asks once per frame for as long as
    // the dropdown is open, and a rebuild re-reads ~19 MB from disk and
    // re-parses it — sixty times a second would be a visible stall, which
    // is a poor trade for fixing a label.
    if CJK_INSTALLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    install_inner(ctx, true)
}

pub fn install(ctx: &egui::Context) {
    install_inner(ctx, false)
}

/// Whether the last `install_inner` put the CJK face in the stack.
///
/// Mirrors the fonts currently on the context, so it has to be updated on
/// every path through `install_inner`, including the ones that leave CJK
/// out — otherwise switching back to English would leave the flag set and
/// a later `install_with_cjk` would skip a rebuild it genuinely needs.
static CJK_INSTALLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn install_inner(ctx: &egui::Context, force_cjk: bool) {
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
    let mut installed_cjk = false;
    if force_cjk || locale_needs_cjk(&crate::i18n::current_locale()) {
        match load_cjk_font() {
            Some(data) => {
                fonts.font_data.insert("cjk".to_owned(), data.into());
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    if let Some(list) = fonts.families.get_mut(&family) {
                        list.push("cjk".to_owned());
                    }
                }
                installed_cjk = true;
            }
            None => tracing::warn!(
                "No CJK font found; this locale will render as empty boxes. \
                 Install a Noto CJK package to fix it."
            ),
        }
    }

    CJK_INSTALLED.store(installed_cjk, std::sync::atomic::Ordering::Relaxed);
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
pub const GROUP: &str = ph::BOUNDING_BOX;
pub const UNGROUP: &str = ph::LINK_BREAK;
pub const RENAME: &str = ph::PENCIL_SIMPLE;
pub const SCENE: &str = ph::BOOKMARK_SIMPLE;
pub const REMINDER: &str = ph::BELL_SIMPLE;
pub const SAVE: &str = ph::FLOPPY_DISK;
pub const ALIGN_LEFT: &str = ph::ALIGN_LEFT;
pub const ALIGN_CENTER: &str = ph::ALIGN_CENTER_HORIZONTAL;
pub const ALIGN_RIGHT: &str = ph::ALIGN_RIGHT;
pub const ALIGN_TOP: &str = ph::ALIGN_TOP;
pub const ALIGN_MIDDLE: &str = ph::ALIGN_CENTER_VERTICAL;
pub const ALIGN_BOTTOM: &str = ph::ALIGN_BOTTOM;
pub const DISTRIBUTE_HORIZONTALLY: &str = ph::ARROWS_OUT_LINE_HORIZONTAL;
pub const DISTRIBUTE_VERTICALLY: &str = ph::ARROWS_OUT_LINE_VERTICAL;

// State / status (used inline with labels).
pub const HIDDEN: &str = ph::EYE_SLASH;
pub const VISIBLE: &str = ph::EYE;
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
pub const BEHAVIOR_SCRIPT: &str = ph::CODE;

// What behavior scripts can reach (the what's-new panel).
pub const SOUND: &str = ph::SPEAKER_HIGH;
pub const MACHINE_LOAD: &str = ph::GAUGE;
pub const COMMAND_PALETTE: &str = ph::LIGHTNING;
pub const ACCESSIBILITY: &str = ph::PERSON_ARMS_SPREAD;

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

    #[test]
    fn cjk_text_is_recognised_and_latin_is_not() {
        for text in ["日本語", "にほん", "中文", "한국어", "ＡＢＣ"] {
            assert!(text_needs_cjk(text), "{text} should need the CJK face");
        }
        for text in ["zen", "héllo", "Ärger", "ñandú", "Привет", ""] {
            assert!(!text_needs_cjk(text), "{text} should not need it");
        }
    }

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

    /// The language picker labels every entry with its *own* name, so a
    /// CJK locale's label needs the CJK face to be drawable — while the
    /// UI around it is still English and would not otherwise load it.
    /// That is why `install_with_cjk` exists, and this pins the condition
    /// that makes it necessary: remove the last non-Latin autonym and the
    /// forced load becomes dead code rather than silently useless.
    #[test]
    fn the_picker_offers_a_language_its_own_label_cannot_draw_unaided() {
        let needing: Vec<_> = crate::i18n::SUPPORTED
            .iter()
            .filter(|(code, _)| locale_needs_cjk(code))
            .collect();
        assert!(
            !needing.is_empty(),
            "no CJK locale is offered any more; install_with_cjk is now dead"
        );
        for (code, autonym) in needing {
            assert!(
                !autonym.is_ascii(),
                "{code} is labelled {autonym:?}, which the bundled fonts \
                 already draw — the forced CJK load would be pointless"
            );
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
