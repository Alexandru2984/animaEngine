# Locale audit — Nederlands (`nl`)

**Status:** partial. Pre-D.1 strings translated previously; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** medium. Claude can produce reasonable surface-level Dutch but cannot reliably distinguish between formal Algemeen Beschaafd Nederlands and informal register. Native review essential before merging the suggestions below.

## Glossary

| English | Nederlands (recommended) | Notes |
|---|---|---|
| overlay | **overlay** | Loan word; standard in Dutch software UIs. |
| scene | **scène** | |
| entity | **entiteit** | |
| edit mode | **bewerkingsmodus** | |
| chord (key combo) | **toetsencombinatie** | |
| library | **bibliotheek** | |
| monitor pin | **vastzetten op monitor** | |
| preset | **voorinstelling** / **preset** | Loan accepted. |

## Placeholder English — proposed translations (AI; verify with native)

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = Sneltoetsen
keybindings-unbound = (niet toegewezen)
keybindings-add = Toevoegen
keybindings-recording = Druk op een toetsencombinatie… (Esc om te annuleren)
keybindings-conflict = Conflict met { $action }
keybindings-reset-all = Alles terugzetten naar standaard
keybindings-help = Aangepaste sneltoetsen worden opgeslagen in config.toml
```

### D.1.7 — Action labels

| key | suggested Dutch |
|---|---|
| `action-toggle-edit-mode` | Bewerkingsmodus wisselen |
| `action-hide-overlay` | Overlay verbergen / tonen |
| `action-pause-all` | Alle animaties pauzeren |
| `action-quit-with-save` | Afsluiten (configuratie opslaan) |
| `action-save-now` | Configuratie nu opslaan |
| `action-open-command-palette` | Opdrachtenpalet |
| `action-cycle-entity` | Naar volgende entiteit |
| `action-delete-selected` | Geselecteerde entiteit verwijderen |
| `action-nudge-up` | Selectie naar boven verplaatsen |
| `action-nudge-down` | Selectie naar beneden verplaatsen |
| `action-nudge-left` | Selectie naar links verplaatsen |
| `action-nudge-right` | Selectie naar rechts verplaatsen |
| `action-center-on-screen` | Selectie op scherm centreren |
| `action-toggle-visible` | Zichtbaarheid wisselen |
| `action-toggle-gravity` | Zwaartekracht wisselen |
| `action-toggle-playback` | Afspelen / pauzeren |
| `action-duplicate-selected` | Selectie dupliceren |
| `action-reset-transform` | Schaal / dekking herstellen |
| `action-bring-forward` | Selectie naar voren brengen |
| `action-send-backward` | Selectie naar achteren brengen |
| `action-fps-up` | FPS verhogen |
| `action-fps-down` | FPS verlagen |
| `action-opacity-up` | Dekking verhogen |
| `action-opacity-down` | Dekking verlagen |
| `action-cycle-monitor` | Monitor van entiteit wisselen |
| `action-show-entity-info` | Entiteitsinformatie tonen |
| `action-show-help` | Toetsenhulp tonen |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = Toegankelijkheid
appearance-accesskit-label = AccessKit-boomupdates genereren
appearance-accesskit-hint = Voorziet AT-SPI-schermlezers (Orca enz.) van data. Laat aan staan; alleen uitschakelen om verbruik te verminderen of als je desktop geen AT-SPI-bus aanbiedt.
```

## Open questions for native reviewer

- *Sneltoetsen* vs *toetsencombinaties* for the tab — *sneltoetsen* is the more common UI term.
- Compound noun separators: Dutch tends to compound aggressively (`bewerkingsmodus`, `entiteitsinformatie`); verify these read naturally rather than feeling stilted.
- *Wisselen* (toggle) is used heavily above; if there's a preferred shorter form for repeated UI buttons, suggest it.
- Native register: AI-translated Dutch often comes across as overly formal. Loosen where appropriate for a desktop overlay's casual context.
