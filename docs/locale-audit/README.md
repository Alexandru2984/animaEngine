# Locale audits

Per-locale review notes for animaEngine's UI strings. Each file lists
placeholder strings waiting for translation, suspected issues in
already-translated entries, and locale-specific glossary anchors.

| Locale | Status | Audit file |
|---|---|---|
| English (`en`) | canonical reference — no audit needed | — |
| Română (`ro`) | maintainer-native, full coverage, spot-check suggestions | [ro.md](ro.md) |
| Deutsch (`de`) | partial; D.1+D.3 placeholder English | [de.md](de.md) |
| Español (`es`) | partial; D.1+D.3 placeholder English | [es.md](es.md) |
| Français (`fr`) | partial; D.1+D.3 placeholder English | [fr.md](fr.md) |
| Italiano (`it`) | partial; D.1+D.3 placeholder English | [it.md](it.md) |
| Nederlands (`nl`) | partial; D.1+D.3 placeholder English | [nl.md](nl.md) |
| Polski (`pl`) | partial; D.1+D.3 placeholder English | [pl.md](pl.md) |
| Português brasileiro (`pt-BR`) | partial; D.1+D.3 placeholder English | [pt-BR.md](pt-BR.md) |
| 日本語 (`ja`) | partial; D.1+D.3 placeholder English | [ja.md](ja.md) |

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
