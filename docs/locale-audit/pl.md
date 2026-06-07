# Locale audit — Polski (`pl`)

**Status:** partial. Pre-D.1 strings translated previously; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** low-medium. Polish grammar (especially case agreement for verbs of motion and prefixed perfective/imperfective pairs) is hard for Claude to get right in idiomatic UI register. The suggestions below are a starting point for a native speaker, not a finished draft.

## Glossary

| English | Polski (recommended) | Notes |
|---|---|---|
| overlay | **nakładka** | |
| scene | **scena** | |
| entity | **encja** / **obiekt** | *Obiekt* more general; *encja* preserves the technical term. |
| edit mode | **tryb edycji** | |
| chord (key combo) | **kombinacja klawiszy** | |
| library | **biblioteka** | |
| monitor pin | **przypięcie do monitora** | |
| preset | **predefinicja** / **preset** | Loan accepted. |

## Placeholder English — proposed translations (AI; verify with native)

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = Skróty klawiszowe
keybindings-unbound = (nieprzypisane)
keybindings-add = Dodaj
keybindings-recording = Naciśnij kombinację… (Esc, aby anulować)
keybindings-conflict = Konflikt z { $action }
keybindings-reset-all = Przywróć wszystkie ustawienia domyślne
keybindings-help = Niestandardowe skróty są zapisywane w config.toml
```

### D.1.7 — Action labels

| key | suggested Polish |
|---|---|
| `action-toggle-edit-mode` | Przełącz tryb edycji |
| `action-hide-overlay` | Ukryj / pokaż nakładkę |
| `action-pause-all` | Wstrzymaj wszystkie animacje |
| `action-quit-with-save` | Wyjdź (zapisz konfigurację) |
| `action-save-now` | Zapisz konfigurację |
| `action-open-command-palette` | Paleta poleceń |
| `action-cycle-entity` | Następna encja |
| `action-delete-selected` | Usuń zaznaczoną encję |
| `action-nudge-up` | Przesuń zaznaczenie w górę |
| `action-nudge-down` | Przesuń zaznaczenie w dół |
| `action-nudge-left` | Przesuń zaznaczenie w lewo |
| `action-nudge-right` | Przesuń zaznaczenie w prawo |
| `action-center-on-screen` | Wyśrodkuj zaznaczenie na ekranie |
| `action-toggle-visible` | Przełącz widoczność |
| `action-toggle-gravity` | Przełącz grawitację |
| `action-toggle-playback` | Odtwarzaj / wstrzymaj |
| `action-duplicate-selected` | Duplikuj zaznaczenie |
| `action-reset-transform` | Resetuj skalę / przezroczystość |
| `action-bring-forward` | Przesuń na wierzch |
| `action-send-backward` | Przesuń na spód |
| `action-fps-up` | Zwiększ FPS |
| `action-fps-down` | Zmniejsz FPS |
| `action-opacity-up` | Zwiększ przezroczystość |
| `action-opacity-down` | Zmniejsz przezroczystość |
| `action-cycle-monitor` | Zmień monitor encji |
| `action-show-entity-info` | Pokaż informacje o encji |
| `action-show-help` | Pokaż pomoc klawiszową |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = Dostępność
appearance-accesskit-label = Generuj aktualizacje drzewa AccessKit
appearance-accesskit-hint = Zasila czytniki ekranu AT-SPI (Orca itp.). Pozostaw włączone; wyłącz tylko jeśli chcesz zmniejszyć zużycie zasobów lub Twój pulpit nie ma magistrali AT-SPI.
```

## Suspected issues for native reviewer

### Case agreement on `keybindings-conflict`

The placeholder uses `Konflikt z { $action }`, where `{ $action }` is an injected nominative-form noun phrase (e.g. `Przełącz tryb edycji`). Polish would normally case-shift after `z` (instrumental case → `Konflikt z **edycją trybu**`), but since the injected string is an imperative action label not a noun, the grammar mismatch is unavoidable without per-action genitive forms.

Two pragmatic options:

- Accept the slight ungrammaticality (the action label functions as a quoted thing): `Konflikt z: { $action }` (with colon to signal "quoted name").
- Rephrase to avoid the case clash: `Kolizja klawiszy: { $action }` or `Już przypisane do: { $action }`.

### Aspect (perfective vs imperfective) on verbs

Polish verbs come in aspect pairs. UI labels conventionally use perfective for one-shot actions (`Zapisz`, `Usuń`, `Duplikuj`) and the imperative form. The proposed labels use perfective where the action completes in one click. Native reviewer to confirm.

### `encja` vs `obiekt`

If the existing pre-D translations use *obiekt* for entity, swap to that consistently; if *encja* is already in use, keep it.

## Open questions for native reviewer

- Pick a single term for `entity` and apply globally.
- Confirm the conflict-message phrasing — the grammar workaround above is opinionated.
- Verify `Pomoc klawiszowa` reads naturally for the H key action; alternatives include `Pomoc skrótów` or just `Pomoc`.
