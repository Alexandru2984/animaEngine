---
name: Locale review
about: Native-speaker review for an animaEngine UI locale
title: "i18n(<lang>): native-speaker pass"
labels: i18n
---

## Locale

<!-- e.g. `de`, `es`, `pt-BR`, `ja`. Use the file-name suffix from
     src/i18n/locales/. -->

## What needs review

<!-- Tick whichever applies. Multiple checkboxes is fine. -->

- [ ] First-time review (locale ships placeholder English in some
      sections; nothing has been audited yet)
- [ ] Drift since the last audit (new strings have landed in
      English and need translating)
- [ ] Suspected mistranslation (specific keys called out below)
- [ ] Glossary inconsistency (one English term resolves to multiple
      target-language equivalents inside this locale)
- [ ] Register / tone (translation is grammatically correct but
      doesn't fit a desktop overlay's voice — too formal, too
      verbose, etc.)

## Reference material

- Pipeline + workflow: [`docs/i18n-pipeline.md`](../docs/i18n-pipeline.md)
- Current audit doc: `docs/locale-audit/<lang>.md`
- Canonical English: [`src/i18n/locales/en.ftl`](../src/i18n/locales/en.ftl)
- Source file: `src/i18n/locales/<lang>.ftl`

## Specific keys / sections

<!-- If you're flagging a specific issue rather than a full pass,
     list the message ids and what's wrong. Otherwise leave blank. -->

| Message id | Current value | Suggested fix | Reason |
|---|---|---|---|
|   |   |   |   |

## Reviewer

<!-- Optional: GitHub handle or contact. We don't require contributors
     to identify themselves — drive-by review is welcome. -->

## Acceptance

Once a PR opens with this work:

- [ ] `cargo test --lib i18n` passes (every-locale-covers-every-en-key
      + every-locale-parses)
- [ ] Audit doc updated to reflect what was applied / deferred /
      rejected (don't silently drop suggestions)
