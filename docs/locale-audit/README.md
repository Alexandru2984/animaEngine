# Locale audits

Per-locale review notes for animaEngine's UI strings. Each file lists
placeholder strings waiting for translation, suspected issues in
already-translated entries, and locale-specific glossary anchors.

Status reflects the W.6 (0.9 freeze) re-audit — see "W.6 re-audit"
below. Every locale now carries a full translation of all 194 keys;
the remaining English-identical values are legitimate (the brand name,
the `X`/`Y`/`FPS`/`z-index` symbols, the pure-placeholder
`{ $name }: { $error }` template, and accepted cognates / loanwords).

| Locale | Status | Audit file |
|---|---|---|
| English (`en`) | canonical reference — no audit needed | — |
| Română (`ro`) | maintainer-native, full coverage | [ro.md](ro.md) |
| Deutsch (`de`) | full coverage; native review for register pending | [de.md](de.md) |
| Español (`es`) | full coverage; native review for register pending | [es.md](es.md) |
| Français (`fr`) | full coverage; native review for register pending | [fr.md](fr.md) |
| Italiano (`it`) | full coverage; native review for register pending | [it.md](it.md) |
| Nederlands (`nl`) | full coverage; native review for register pending | [nl.md](nl.md) |
| Polski (`pl`) | full coverage; native review for register pending | [pl.md](pl.md) |
| Português brasileiro (`pt-BR`) | full coverage; native review for register pending | [pt-BR.md](pt-BR.md) |
| 日本語 (`ja`) | full coverage; native review for register pending | [ja.md](ja.md) |

## W.6 re-audit (0.9 freeze, 2026-06-14)

The 0.4 cross-locale pipeline re-run over the complete 1.0 string set
(194 keys × 10 locales). Findings:

- **Key parity: clean.** Every locale has exactly the canonical key
  set — zero missing, zero extra. Guarded by
  `i18n::tests::every_locale_covers_every_en_key`.
- **Placeholder parity: clean, and now tested.** Every locale
  interpolates exactly the same `{ $var }` arguments per key as
  English. A new guard,
  `i18n::tests::every_locale_matches_en_placeholder_args`, makes this
  permanent — a translator dropping `{ $count }` now fails CI.
- **6 corrupted values fixed** — a stray duplicated suffix from an
  earlier batch (the strings were never wrong in meaning, just
  mistyped): `es` Imagenn→Imagen; `it` Linearee→Lineare,
  Verticalee→Verticale, Recentii→Recenti; `pt-BR` Imagemm→Imagem,
  Recenteses→Recentes.
- **RTL:** no RTL locale ships; the gating limitations are recorded in
  [`../i18n-pipeline.md`](../i18n-pipeline.md#rtl-right-to-left-status).

What's left is **tone/register native review**, not coverage — the
per-locale notes below remain advisory for a native speaker.

## How to contribute

See [`../i18n-pipeline.md`](../i18n-pipeline.md) for the full workflow.
Quick version:

1. Pick a locale that needs work.
2. Read its audit doc + the canonical [`src/i18n/locales/en.ftl`](
   ../../src/i18n/locales/en.ftl).
3. Apply translations / fixes in `src/i18n/locales/<lang>.ftl`.
4. Update the audit doc — comment out resolved items, leave a note
   for ones you considered but rejected so the next pass doesn't
   re-surface them.
5. Run `cargo test --lib i18n` to confirm parity + Fluent parse.
6. Open a PR using the [`Locale review` issue template](
   ../../.github/ISSUE_TEMPLATE/locale-review.md) for tracking.

## AI confidence — what the audits are and aren't

Initial sweep was an automated AI cross-check. Confidence varies
by language:

- **High confidence** (de, es, fr, it, pt-BR): the suggestions are
  reasonable starting points; native review still needed for tone
  and regional register.
- **Medium-low confidence** (nl, pl, ja): grammar and politeness
  level need human verification beyond the AI's reach. Treat
  suggested strings as scaffolding.
- **Native** (ro): maintainer-translated; audit is a fresh-eye spot
  check rather than a gap-fill.

The audits are advisory. A native speaker reading them decides what
to commit — none of the suggestions are auto-applied.
