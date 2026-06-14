# i18n pipeline — native-speaker review workflow

animaEngine ships UI strings in 10 locales (English canonical +
Română, Español, Deutsch, Français, Italiano, Português brasileiro,
Polski, Nederlands, 日本語). This doc is the contract for keeping
those translations honest as we add new strings.

The TL;DR: **every new string lands in English first, then in every
other locale within the same PR.** Non-native locales accept
placeholder English so the build doesn't break — they get rewritten
in a follow-up review pass. The two halves are separable, but only
the first half blocks release.

## Where strings live

```
src/i18n/locales/
├── en.ftl          ← canonical reference; every other locale mirrors its keys
├── ro.ftl          ← Romanian (maintainer-native)
├── de.ftl  es.ftl  fr.ftl  it.ftl  nl.ftl  pl.ftl  pt-BR.ftl  ja.ftl
```

Format: [Fluent](https://projectfluent.org/) (`.ftl`). Message ids
use `kebab-case` with a domain prefix:

```ftl
# domain-noun-context = "..."
settings-tab-inspector   = Inspector
keybindings-recording    = Press a chord… (Esc to cancel)
action-toggle-edit-mode  = Toggle edit mode
```

Selectors and arguments use the standard Fluent syntax:

```ftl
keybindings-conflict = Conflicts with { $action }
entity-count-plural  = { $n } entities
```

## Adding a new string

1. **Write the English form in `en.ftl`.** Group it under the
   existing `# ──` section if one fits, or add a new section header.
2. **Copy the key to every other locale file** with one of:
   - A real translation, if you speak the language or have a native
     speaker on standby; **or**
   - The English text as a placeholder, with a comment marker so the
     audit pass can find it:

     ```ftl
     # ── New keybindings block (D.1.6) — placeholder pending native-speaker audit
     keybindings-recording = Press a chord… (Esc to cancel)
     ```

3. **Run the parity tests** before committing. Two guards enforce the
   contract: no key may exist in `en.ftl` yet be missing from another
   locale, and every locale must interpolate exactly the same
   `{ $variable }` arguments per key as English (a dropped or renamed
   placeholder renders a broken string at runtime):

   ```bash
   cargo test --lib i18n::tests::every_locale_covers_every_en_key
   cargo test --lib i18n::tests::every_locale_matches_en_placeholder_args
   # or just: cargo test --lib i18n
   ```

4. **Use a `t()` lookup at the call site** — never inline a literal
   user-visible string in Rust code:

   ```rust
   ui.label(t("keybindings-help"));
   // For Fluent arguments, use t_args:
   let mut args = FluentArgs::new();
   args.set("action", "Toggle visibility");
   ui.label(t_args("keybindings-conflict", &args));
   ```

## Audit pass (per-locale review)

The per-locale audit lives under [`docs/locale-audit/`](
locale-audit/). Each file lists, for one locale:

- Keys currently carrying English placeholder.
- Suspected translation issues (ambiguity, register mismatch,
  terminology drift) in the already-translated entries.
- Suggested fixes, when the reviewer is confident.

The audits are **advisory** — a native speaker reading the doc
decides what to commit. Cross-check by an LLM gives a starting
point but is explicitly *not* authoritative on idiom or register;
treat it as a sniff test.

### Workflow for a native speaker

1. Open the `.ftl` file for your locale.
2. Open the matching `docs/locale-audit/<lang>.md`.
3. Apply translations for any "placeholder pending" block.
4. Skim the suspected-issue list — apply, defer, or reject each
   suggestion. The audit doc isn't sacred; comment out rejected items
   so the next pass doesn't surface them again.
5. Open a PR titled `i18n(<lang>): native-speaker pass <date>`.
6. Re-run `cargo test --lib i18n` to confirm parity + Fluent parse.

### How to request a review

Issues are tracked via the
[`Locale review` template](../.github/ISSUE_TEMPLATE/locale-review.md).
File one issue per locale that wants attention; link any related
PRs that introduced the placeholder strings.

## Glossary anchors (terminology stability)

These terms appear repeatedly across the UI and must translate
consistently within a single locale. The audit docs flag deviations.

| English term | Domain | Notes |
|---|---|---|
| **overlay** | windowing | The always-on-top transparent surface |
| **scene** | data | The collection of entities + groups |
| **entity** | data | One animated sprite instance |
| **edit mode** | UX | Interactive vs pass-through state |
| **chord** | input | Key + modifiers combo, e.g. `Ctrl+Shift+A` |
| **library** | data | Indexed asset directory |
| **monitor pin** | windowing | Per-entity binding to one monitor |
| **preset** | UX | Curated scene template |

Pick one Romanian / Spanish / etc. equivalent per term and use it
everywhere. The audit doc for each locale carries the locale-specific
glossary at the top.

## What we explicitly do *not* do

- **Auto-translate via online services.** DeepL, Google Translate,
  etc. produce plausible strings that fail register checks (UI
  terseness, button verbs, etc.) and silently introduce loan words
  in languages with tight technical vocabulary. Placeholders are
  honest about being unfinished; auto-translations look done but
  aren't.
- **Mark partial locales as "beta".** All shipped locales are
  treated as production from the user's point of view. The audit
  process is internal scaffolding; the UI never tells the user
  "your language is partly machine-translated."
- **Branch translations per dialect.** `pt-BR` is the only
  Portuguese variant we ship; Continental Portuguese contributors
  can submit a separate `pt-PT.ftl` if they want, but we won't
  fork strings inside an existing file.

## RTL (right-to-left) status

All 10 shipped locales are left-to-right. **No RTL locale (Arabic,
Hebrew, Persian, …) ships through 1.0**, and adding one is *not* a
drop-in `.ftl` file — it's gated on two things this codebase doesn't
do yet:

- **egui text shaping** has no bidirectional reordering, so an RTL
  string would render visually reversed.
- The i18n layer deliberately **disables Fluent's bidi isolation**
  (`bundle.set_use_isolating(false)` in `src/i18n/mod.rs`, and strips
  the FSI/PDI codepoints) so interpolated values copy cleanly into bug
  reports. That trade-off is correct for LTR-only; an RTL locale would
  need isolation re-enabled and scoped to the interpolation sites.

The *key layer* itself is direction-agnostic (Fluent is just
key → string), so the data model doesn't block RTL — only the
rendering and isolation policy do. Recorded here so a future RTL
contributor knows the real cost up front rather than discovering it
after translating 194 strings.
