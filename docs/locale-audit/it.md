# Locale audit — Italiano (`it`)

**Status:** complete through 0.8 (2026-06-12 pass — all 0.4–0.8 strings translated; identical-to-EN keys verified as legitimate cognates). Human native review still welcome before 1.0.
**AI cross-check confidence:** high (the model has solid Italian technical-UI familiarity for surface-level translation; regional register and idiom still need a human pass).

## Glossary

| English | Italiano (recommended) | Notes |
|---|---|---|
| overlay | **overlay** / **sovrapposizione** | Loan `overlay` common; *sovrapposizione* more formal. Pick one. |
| scene | **scena** | |
| entity | **entità** | |
| edit mode | **modalità modifica** | |
| chord (key combo) | **combinazione di tasti** | |
| library | **libreria** | |
| monitor pin | **fissare allo schermo** | |
| preset | **preimpostazione** / **preset** | Loan accepted in IT dev UIs. |

## Placeholder English — proposed translations

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = Scorciatoie
keybindings-unbound = (non assegnato)
keybindings-add = Aggiungi
keybindings-recording = Premi una combinazione… (Esc per annullare)
keybindings-conflict = Conflitto con { $action }
keybindings-reset-all = Ripristina tutto ai valori predefiniti
keybindings-help = Le scorciatoie personalizzate vengono salvate in config.toml
```

### D.1.7 — Action labels

| key | suggested Italian |
|---|---|
| `action-toggle-edit-mode` | Attiva/disattiva modalità modifica |
| `action-hide-overlay` | Nascondi / mostra overlay |
| `action-pause-all` | Metti in pausa tutte le animazioni |
| `action-quit-with-save` | Esci (salva la configurazione) |
| `action-save-now` | Salva la configurazione |
| `action-open-command-palette` | Tavolozza comandi |
| `action-cycle-entity` | Entità successiva |
| `action-delete-selected` | Elimina l'entità selezionata |
| `action-nudge-up` | Sposta la selezione in alto |
| `action-nudge-down` | Sposta la selezione in basso |
| `action-nudge-left` | Sposta la selezione a sinistra |
| `action-nudge-right` | Sposta la selezione a destra |
| `action-center-on-screen` | Centra la selezione sullo schermo |
| `action-toggle-visible` | Attiva/disattiva visibilità |
| `action-toggle-gravity` | Attiva/disattiva gravità |
| `action-toggle-playback` | Riproduci / metti in pausa |
| `action-duplicate-selected` | Duplica la selezione |
| `action-reset-transform` | Ripristina scala / opacità |
| `action-bring-forward` | Porta la selezione in primo piano |
| `action-send-backward` | Porta la selezione sullo sfondo |
| `action-fps-up` | Aumenta FPS |
| `action-fps-down` | Riduci FPS |
| `action-opacity-up` | Aumenta opacità |
| `action-opacity-down` | Riduci opacità |
| `action-cycle-monitor` | Cambia monitor dell'entità |
| `action-show-entity-info` | Mostra dettagli dell'entità |
| `action-show-help` | Mostra aiuto tastiera |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = Accessibilità
appearance-accesskit-label = Genera aggiornamenti dell'albero AccessKit
appearance-accesskit-hint = Alimenta i lettori di schermo AT-SPI (Orca, ecc.). Lascia attivo; disattiva solo se vuoi ridurre il consumo o se il tuo desktop non espone un bus AT-SPI.
```

## Suspected issues

### *Tavolozza comandi* vs *Palette dei comandi*

`palette` translates to *tavolozza* (palette as in painter's palette) or *palette dei comandi* (loan). Italian software conventions split — VSCode IT uses *Palette dei comandi*. Pick one consistently with `palette-search-placeholder` already in the file.

### Anglicism management

Italian has high tolerance for English loanwords in tech: *FPS*, *preset*, *overlay*, *layer* are all acceptable. The proposed translations retain *overlay* as a loan; reviewer should confirm that matches house style for the rest of the existing file.

### Gendered nouns and adjectives

*Animazione* is feminine; agreements like `tutte le animazioni` are correct in `action-pause-all`. Spot-check the rest for f/m agreement after applying.

## Open questions for native reviewer

- *Scorciatoie* (shortcuts) vs *combinazioni di tasti* (key combos) for the tab title?
- *Sovrapposizione* (full Italian) vs *overlay* (loan) — pick one and apply globally.
- *Tavolozza* vs *Palette* for the command palette.
