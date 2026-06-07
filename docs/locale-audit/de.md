# Locale audit — Deutsch (`de`)

**Status:** partial. Pre-D.1 strings translated by previous pass; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** medium-high (Claude has solid German technical-UI familiarity but cannot judge regional register).

## Glossary (locale-specific anchors)

| English | German (recommended) | Notes |
|---|---|---|
| overlay | **Overlay** / **Einblendung** | Loan word `Overlay` is common in German dev contexts; `Einblendung` is more native. Pick one and use it everywhere. |
| scene | **Szene** | |
| entity | **Entität** | Standard technical translation. |
| edit mode | **Bearbeitungsmodus** | |
| chord (key combo) | **Tastenkombination** | |
| library | **Bibliothek** | |
| monitor pin | **Monitor anheften** (verb) / **Monitor-Bindung** (noun) | |
| preset | **Voreinstellung** / **Preset** | Loan `Preset` widely accepted in DE software UIs. |

## Placeholder English strings to translate

These 37 keys ship English text in `de.ftl`. Marked `# placeholder pending …`. Order matches the file.

### D.1.6 — Keybindings tab UI (7 keys)

```ftl
settings-tab-keybindings = Keybindings
keybindings-unbound = (unbound)
keybindings-add = Add
keybindings-recording = Press a chord… (Esc to cancel)
keybindings-conflict = Conflicts with { $action }
keybindings-reset-all = Reset all to defaults
keybindings-help = Custom shortcuts persist in config.toml
```

**Suggested German (AI; native review essential):**

```ftl
settings-tab-keybindings = Tastenkürzel
keybindings-unbound = (nicht zugewiesen)
keybindings-add = Hinzufügen
keybindings-recording = Tastenkombination drücken… (Esc zum Abbrechen)
keybindings-conflict = Konflikt mit { $action }
keybindings-reset-all = Alle auf Standard zurücksetzen
keybindings-help = Eigene Kürzel werden in config.toml gespeichert
```

### D.1.7 — Action labels (27 keys)

| key | suggested German |
|---|---|
| `action-toggle-edit-mode` | Bearbeitungsmodus umschalten |
| `action-hide-overlay` | Overlay aus-/einblenden |
| `action-pause-all` | Alle Animationen pausieren |
| `action-quit-with-save` | Beenden (Konfiguration speichern) |
| `action-save-now` | Konfiguration speichern |
| `action-open-command-palette` | Befehlspalette öffnen |
| `action-cycle-entity` | Nächste Entität auswählen |
| `action-delete-selected` | Ausgewählte Entität löschen |
| `action-nudge-up` | Auswahl nach oben schieben |
| `action-nudge-down` | Auswahl nach unten schieben |
| `action-nudge-left` | Auswahl nach links schieben |
| `action-nudge-right` | Auswahl nach rechts schieben |
| `action-center-on-screen` | Auswahl zentrieren |
| `action-toggle-visible` | Sichtbarkeit umschalten |
| `action-toggle-gravity` | Schwerkraft umschalten |
| `action-toggle-playback` | Wiedergabe umschalten |
| `action-duplicate-selected` | Auswahl duplizieren |
| `action-reset-transform` | Größe / Deckkraft zurücksetzen |
| `action-bring-forward` | Auswahl nach vorne |
| `action-send-backward` | Auswahl nach hinten |
| `action-fps-up` | FPS erhöhen |
| `action-fps-down` | FPS verringern |
| `action-opacity-up` | Deckkraft erhöhen |
| `action-opacity-down` | Deckkraft verringern |
| `action-cycle-monitor` | Monitor der Entität wechseln |
| `action-show-entity-info` | Entitätsinformationen anzeigen |
| `action-show-help` | Tastenhilfe anzeigen |

### D.3 — Accessibility section (3 keys)

```ftl
appearance-accessibility-header = Barrierefreiheit
appearance-accesskit-label = AccessKit-Baumaktualisierungen generieren
appearance-accesskit-hint = Versorgt AT-SPI-Screenreader (Orca etc.). Aktiv lassen — deaktivieren nur, wenn ein schlankerer Footprint gewünscht ist oder der Desktop keinen AT-SPI-Bus betreibt.
```

## Suspected issues in already-translated strings

(Pending native-speaker review — Claude's confidence here is "this looks reasonable" not "this is idiomatic.")

### `behavior-wander` translation drift risk

If existing `behavior-wander` translates as something like *Streifen* (to roam), confirm it doesn't shift to *Umherirren* (to wander confused) — the latter has a negative connotation.

### Capitalisation discipline

German nouns are capitalised. Spot-check that every multi-word UI key keeps proper capitalisation in noun positions (e.g. `Konfiguration speichern`, not `konfiguration speichern`). The placeholder block above follows the rule.

## Open questions for native reviewer

- Prefer `Overlay` (loan) or `Einblendung` (native)?
- Prefer `Preset` (loan) or `Voreinstellung` (native)?
- Is *Tastenkombination* preferred over *Tastenkürzel* for "chord"? They overlap but carry slightly different shades (combination vs shortcut).
