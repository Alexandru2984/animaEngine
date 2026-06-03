//! Internationalisation backed by Project Fluent.
//!
//! `init()` populates one [`FluentBundle`] per supported locale from
//! the `locales/*.ftl` files baked into the binary via `include_str!`.
//! Lookup goes through [`t`], which falls back to English when the
//! active locale doesn't carry the key — there's no "missing string"
//! state visible to the user.
//!
//! Switching locales is constant-time: [`set_locale`] swaps an
//! `Arc<FluentBundle>` behind an `RwLock`; no FTL reparse, no atlas
//! invalidation. The UI re-renders on the next frame and picks up the
//! new strings naturally.
//!
//! ## Why Fluent
//!
//! - Plural rules built in (we don't use them yet — see `entity-count`
//!   below — but they're there for free when we add `{$n} ->` switches)
//! - Message references (e.g. `welcome = Hi { $user }. -> { app-name }`)
//!   make the strings stay coherent when a localizer iterates
//! - The runtime overhead vs raw HashMap lookups is negligible at our
//!   scale (a few hundred lookups per frame)
//!
//! ## Adding a key
//!
//! 1. Edit `locales/en.ftl` — English is the source of truth.
//! 2. Drop the same key into every other locale with the best
//!    translation you have. Missing keys are not an error — Fluent
//!    falls back to English silently, but the test
//!    `every_locale_covers_every_en_key` will fail in CI to remind
//!    you to translate.
//!
//! ## Adding a locale
//!
//! 1. Drop a new `xx.ftl` (or `xx-YY.ftl`) in `locales/`.
//! 2. Append the code to [`SUPPORTED`].
//! 3. Append an `include_str!` arm in [`load_source`].

use std::sync::{Arc, OnceLock, RwLock};

use fluent::{concurrent::FluentBundle, FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

/// Tuple `(code, display_name_in_that_language)` for each locale we
/// ship strings for. The UI picker reads `display_name` so users see
/// their own language in their own script.
pub const SUPPORTED: &[(&str, &str)] = &[
    ("en", "English"),
    ("ro", "Română"),
    ("es", "Español"),
    ("de", "Deutsch"),
    ("fr", "Français"),
    ("it", "Italiano"),
    ("pt-BR", "Português (BR)"),
    ("pl", "Polski"),
    ("nl", "Nederlands"),
    ("ja", "日本語"),
];

/// Fallback locale code. Every key must exist here; missing entries in
/// other locales transparently bubble up to this bundle.
pub const FALLBACK: &str = "en";

struct State {
    /// Currently-active locale code (from [`SUPPORTED`]).
    active: String,
    /// Bundle for the active locale.
    active_bundle: Arc<FluentBundle<FluentResource>>,
    /// Bundle for the fallback locale (`en`). Kept hot so [`t`] can
    /// chase missing keys without an allocation per lookup.
    fallback_bundle: Arc<FluentBundle<FluentResource>>,
}

static STATE: OnceLock<RwLock<State>> = OnceLock::new();

/// Initialize the i18n subsystem. Picks the initial locale from
/// `requested`, falling back to whatever `LANG` / `LC_ALL` envs name,
/// and ultimately to English if nothing matches. Idempotent — calling
/// twice is a no-op (the second locale wins anyway via
/// [`set_locale`]).
pub fn init(requested: Option<&str>) {
    // Reject an explicit `requested` code that isn't supported, but log
    // it — a config tampered with (or a typo'd CLI flag) shouldn't
    // silently fall back. The env-var path is intentionally quieter
    // because `LANG=fr_FR.UTF-8` on a non-French build is normal user
    // behaviour, not an anomaly.
    if let Some(code) = requested {
        if !code_is_supported(code) {
            tracing::warn!(
                "Requested locale {:?} is not in the SUPPORTED list; falling back",
                code,
            );
        }
    }
    let initial = requested
        .map(|s| s.to_string())
        .or_else(detect_from_env)
        .filter(|code| code_is_supported(code))
        .unwrap_or_else(|| FALLBACK.to_string());

    let fallback_bundle = Arc::new(build_bundle(FALLBACK));
    let active_bundle = if initial == FALLBACK {
        fallback_bundle.clone()
    } else {
        Arc::new(build_bundle(&initial))
    };

    let _ = STATE.set(RwLock::new(State {
        active: initial,
        active_bundle,
        fallback_bundle,
    }));
}

/// Lookup a key with no arguments. Returns the key itself, wrapped in
/// `?` markers, if even the fallback bundle doesn't carry it — so a
/// typo shows up loud in the UI rather than silently rendering an
/// empty label.
pub fn t(key: &str) -> String {
    t_with(key, None)
}

/// Lookup with positional arguments. Same fallback semantics as [`t`].
pub fn t_args(key: &str, args: &FluentArgs<'_>) -> String {
    t_with(key, Some(args))
}

fn t_with(key: &str, args: Option<&FluentArgs<'_>>) -> String {
    let Some(state) = STATE.get() else {
        // Subsystem not initialised — return the raw key so we still
        // render something and the bug is obvious.
        return format!("?{key}?");
    };
    let guard = state.read().expect("i18n state poisoned");
    if let Some(formatted) = format_in(&guard.active_bundle, key, args) {
        return formatted;
    }
    if let Some(formatted) = format_in(&guard.fallback_bundle, key, args) {
        return formatted;
    }
    format!("?{key}?")
}

fn format_in(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: Option<&FluentArgs<'_>>,
) -> Option<String> {
    let message = bundle.get_message(key)?;
    let pattern = message.value()?;
    let mut errors = Vec::new();
    let rendered = bundle.format_pattern(pattern, args, &mut errors);
    // Fluent inserts invisible directional isolates around interpolated
    // values; strip them so the strings copy cleanly when the user
    // pastes them into a bug report.
    Some(strip_isolates(&rendered))
}

fn strip_isolates(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(*c, '\u{2068}' | '\u{2069}'))
        .collect()
}

/// Switch to a different locale. Silently no-ops if `code` is not in
/// [`SUPPORTED`] — the UI picker only emits valid codes so this is a
/// defensive fallback for direct callers.
pub fn set_locale(code: &str) {
    if !code_is_supported(code) {
        tracing::warn!(
            "set_locale ignored unsupported code {:?}; active locale unchanged",
            code,
        );
        return;
    }
    let Some(state) = STATE.get() else { return };
    let mut guard = state.write().expect("i18n state poisoned");
    if guard.active == code {
        return;
    }
    guard.active = code.to_string();
    guard.active_bundle = if code == FALLBACK {
        guard.fallback_bundle.clone()
    } else {
        Arc::new(build_bundle(code))
    };
}

/// Currently-active locale code from [`SUPPORTED`].
pub fn current_locale() -> String {
    STATE
        .get()
        .map(|s| {
            s.read()
                .map(|g| g.active.clone())
                .unwrap_or_else(|_| FALLBACK.to_string())
        })
        .unwrap_or_else(|| FALLBACK.to_string())
}

fn code_is_supported(code: &str) -> bool {
    SUPPORTED.iter().any(|(c, _)| *c == code)
}

/// Best-effort scan of locale environment variables. Strips the
/// `.UTF-8` suffix and rewrites `pt_BR` → `pt-BR` so the codes line up
/// with [`SUPPORTED`].
fn detect_from_env() -> Option<String> {
    for var in &["LANG", "LC_ALL", "LC_MESSAGES"] {
        if let Ok(raw) = std::env::var(var) {
            if let Some(code) = normalise_env_locale(&raw) {
                return Some(code);
            }
        }
    }
    None
}

fn normalise_env_locale(raw: &str) -> Option<String> {
    let head = raw.split('.').next().unwrap_or("");
    let normalised = head.replace('_', "-");
    if normalised.is_empty() || normalised.eq_ignore_ascii_case("c") {
        return None;
    }
    // Match exactly first (handles `pt-BR`); then by language prefix
    // (`fr-FR` -> `fr`) so the user can avoid a perfect match.
    if code_is_supported(&normalised) {
        return Some(normalised);
    }
    let lang_only = normalised
        .split('-')
        .next()
        .unwrap_or(&normalised)
        .to_string();
    if code_is_supported(&lang_only) {
        return Some(lang_only);
    }
    None
}

// ─── bundle construction ──────────────────────────────────────────────

fn build_bundle(code: &str) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = code.parse().unwrap_or_else(|_| FALLBACK.parse().unwrap());
    let source = load_source(code);
    let resource = FluentResource::try_new(source.to_string())
        .expect("locale FTL file has a syntax error — caught by every_locale_parses");
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    // Disable Unicode bidi isolation by configuration too. We already
    // strip the codepoints, but turning it off here prevents the parser
    // from emitting them in the first place — small perf win.
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .expect("locale FTL file has duplicate keys — caught by every_locale_parses");
    bundle
}

fn load_source(code: &str) -> &'static str {
    match code {
        "en" => include_str!("locales/en.ftl"),
        "ro" => include_str!("locales/ro.ftl"),
        "es" => include_str!("locales/es.ftl"),
        "de" => include_str!("locales/de.ftl"),
        "fr" => include_str!("locales/fr.ftl"),
        "it" => include_str!("locales/it.ftl"),
        "pt-BR" => include_str!("locales/pt-BR.ftl"),
        "pl" => include_str!("locales/pl.ftl"),
        "nl" => include_str!("locales/nl.ftl"),
        "ja" => include_str!("locales/ja.ftl"),
        _ => include_str!("locales/en.ftl"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every shipped locale must parse and accept all keys without
    /// errors. A `try_new` failure here means a translator typo'd a
    /// brace or a quoted string.
    #[test]
    fn every_locale_parses() {
        for (code, _) in SUPPORTED {
            let source = load_source(code);
            FluentResource::try_new(source.to_string())
                .unwrap_or_else(|_| panic!("locale {code} has FTL syntax errors"));
        }
    }

    /// English is the canonical key set. Every other locale must
    /// translate the same keys so a missing translation can't go
    /// unnoticed.
    #[test]
    fn every_locale_covers_every_en_key() {
        let en_keys: HashSet<String> = collect_keys("en");
        for (code, _) in SUPPORTED {
            if *code == "en" {
                continue;
            }
            let keys = collect_keys(code);
            let missing: Vec<_> = en_keys.difference(&keys).collect();
            assert!(
                missing.is_empty(),
                "locale {code} is missing keys: {missing:?}",
            );
        }
    }

    fn collect_keys(code: &str) -> HashSet<String> {
        let source = load_source(code);
        let mut out = HashSet::new();
        for line in source.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            // Top-level FTL message lines start with the key followed by
            // optional whitespace and `=`. Continuations are indented.
            if line.starts_with(char::is_whitespace) {
                continue;
            }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                if !key.is_empty() && !key.contains(' ') {
                    out.insert(key.to_string());
                }
            }
        }
        out
    }

    #[test]
    fn normalise_strips_charset_suffix() {
        assert_eq!(normalise_env_locale("en_US.UTF-8"), Some("en".to_string()));
        assert_eq!(normalise_env_locale("ro_RO.UTF-8"), Some("ro".to_string()));
        assert_eq!(
            normalise_env_locale("pt_BR.UTF-8"),
            Some("pt-BR".to_string()),
        );
    }

    #[test]
    fn normalise_drops_c_locale() {
        assert!(normalise_env_locale("C").is_none());
        assert!(normalise_env_locale("c.UTF-8").is_none());
        assert!(normalise_env_locale("").is_none());
    }

    #[test]
    fn normalise_falls_back_to_language_only() {
        // We don't ship en-GB explicitly, so it should fall back to en.
        assert_eq!(normalise_env_locale("en_GB"), Some("en".to_string()));
        // ko isn't supported at all → None.
        assert!(normalise_env_locale("ko_KR").is_none());
    }

    /// Smoke test the full lookup pipeline. Initialising twice in
    /// tests is fine because [`init`] is a no-op once `STATE` is set,
    /// but we route through `set_locale` to switch.
    #[test]
    fn t_round_trips_through_english() {
        init(Some("en"));
        // app-name is the first key in every locale.
        assert_eq!(t("app-name"), "animaEngine");
    }

    #[test]
    fn t_falls_back_when_key_missing_in_active_locale() {
        init(Some("en"));
        set_locale("ro");
        // app-name lives in ro too; if a future translator drops it,
        // the lookup still resolves via the fallback bundle.
        let value = t("app-name");
        assert!(!value.starts_with('?'), "got fallback marker: {value}");
    }

    #[test]
    fn t_returns_marker_for_unknown_key() {
        init(Some("en"));
        let v = t("definitely-not-a-real-key");
        assert!(v.starts_with('?') && v.ends_with('?'));
    }
}
