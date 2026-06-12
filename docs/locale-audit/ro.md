# Locale audit — Română (`ro`)

**Status:** complete (maintainer-native locale — translated at source for every release).
**Last AI cross-check:** 2026-06-05 (automated). Maintainer is native Romanian, so the audit role here is "fresh eye spot check," not gap-fill.

## Glossary (locale-specific anchors)

| English | Romanian | Notes |
|---|---|---|
| overlay | **suprapunere** | `action-hide-overlay` uses *suprapunerea* — keep this for any future overlay-related strings. |
| scene | **scenă** | Consistent. |
| entity | **entitate** | Consistent across inspector + scene. |
| edit mode | **mod editare** | Used as *modul editare* in action label — fine. |
| chord (key combo) | **combinație** | Used in `keybindings-recording`. Consider also *legătură* if "combinație" feels too literal in a future revision. |
| library | **bibliotecă** | Consistent. |
| monitor pin | **pinează / monitorul entității** | Mixed verb forms — see issues below. |
| preset | **preset** | Loan word retained; idiomatic in Romanian dev usage. |

## Suspected issues (spot-check)

These are advisory — apply, defer, or reject. Strikethroughs reflect choices made in the file.

### 1. `monitor-pin-label = Pinează pe monitor` vs `action-cycle-monitor = Schimbă monitorul entității`

The verb for "pinning" oscillates between `pinează` (loan from English, action) and `schimbă` (native verb, switching action). Both are arguably correct in context, but the noun form *pin* shows up only in English in the docs/UI. Consider:

- `monitor-pin-label = Fixează pe monitor` (replaces *pinează* with native *fixează*); or
- Keep as is, and document `pinează` as the canonical Romanian term in the glossary.

**Severity:** low. Both terms are intelligible.

### 2. `behavior-wander = Rătăcire delimitată`

Literally accurate but slightly clinical. Considering UI brevity, *Plimbare delimitată* would mirror the existing `behavior-walk = Plimbare` more directly. Tradeoff: "rătăcire" carries the random-wander connotation more precisely.

**Severity:** very low.

### 3. `appearance-accesskit-hint`

> Alimentează cititoarele de ecran AT-SPI (Orca etc.). Lasă activ dacă nu vrei să reduci consumul sau dacă desktop-ul tău nu rulează un bus AT-SPI.

The English source phrasing ("Leave on unless you want a tighter footprint or your desktop doesn't run an AT-SPI bus") inverts the conditional. The Romanian version flips polarity: "Lasă activ dacă **nu vrei** să reduci consumul **sau dacă** desktop-ul tău nu rulează…" — that "sau dacă" reads as "or if your desktop doesn't run an AT-SPI bus, leave it on", which is the opposite of intent.

**Suggested rewrite:**

> Alimentează cititoarele de ecran AT-SPI (Orca etc.). Lasă activ; dezactivează doar dacă vrei să reduci consumul sau dacă desktop-ul tău nu rulează un bus AT-SPI.

**Severity:** medium. Affects user decision on a setting.

### 4. `inspector-section-position = Poziție` vs UI hardcoded "Position"

The Inspector tab's section headers are currently hardcoded English literals in `src/ui/panels.rs` (`section(ui, "Position", …)`). These i18n keys exist but aren't wired. Cleanup is queued in D.9 (UX consistency pass) — same applies to Appearance, Animation, Behavior section headers.

**Severity:** structural, not Romanian-specific. Track in [[d-9-ux-consistency-pass]].

### 5. `action-pause-all = Oprește toate animațiile`

*Oprește* means "stop" (terminate); the action is actually a pause/resume toggle. The display label appears in the command palette where users expect "Pause all animations" semantics. Consider:

- `action-pause-all = Pauză pentru toate animațiile`; or
- `action-pause-all = Comută redarea globală` (matches in-app semantic).

**Severity:** medium. The current verb implies a one-way stop.

## Action labels — quick scan

| key | RO | comment |
|---|---|---|
| `action-cycle-entity = Treci la următoarea entitate` | ✅ |
| `action-nudge-up` etc. = `Mută selecția în sus/jos/stânga/dreapta` | ✅, consistent |
| `action-bring-forward = Adu selecția în față` / `action-send-backward = Trimite selecția în spate` | ✅ |
| `action-fps-up = Crește FPS-ul` / `action-fps-down = Scade FPS-ul` | ✅ |
| `action-opacity-up = Crește opacitatea` / `action-opacity-down = Scade opacitatea` | ✅ |

## Recommended actions before 0.4 release

- Fix #3 (accesskit hint polarity) — affects user understanding.
- Consider #5 (pause-all verb) — minor but improves precision.
- Park #1, #2 for the D.9 consistency pass.
