# Locale audit — Français (`fr`)

**Status:** partial. Pre-D.1 strings translated previously; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** high (the model has solid French technical-UI familiarity for surface-level translation; regional register and idiom still need a human pass).

## Glossary

| English | Français (recommended) | Notes |
|---|---|---|
| overlay | **superposition** | |
| scene | **scène** | |
| entity | **entité** | |
| edit mode | **mode édition** | |
| chord (key combo) | **combinaison de touches** | |
| library | **bibliothèque** | |
| monitor pin | **épingler à l'écran** | verb form |
| preset | **préréglage** | French software convention. |

## Placeholder English — proposed translations

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = Raccourcis clavier
keybindings-unbound = (non assigné)
keybindings-add = Ajouter
keybindings-recording = Appuyez sur une combinaison… (Échap pour annuler)
keybindings-conflict = Conflit avec { $action }
keybindings-reset-all = Tout réinitialiser aux valeurs par défaut
keybindings-help = Les raccourcis personnalisés sont enregistrés dans config.toml
```

### D.1.7 — Action labels

| key | suggested French |
|---|---|
| `action-toggle-edit-mode` | Basculer le mode édition |
| `action-hide-overlay` | Masquer / afficher la superposition |
| `action-pause-all` | Mettre en pause toutes les animations |
| `action-quit-with-save` | Quitter (enregistrer la configuration) |
| `action-save-now` | Enregistrer la configuration |
| `action-open-command-palette` | Palette de commandes |
| `action-cycle-entity` | Entité suivante |
| `action-delete-selected` | Supprimer l'entité sélectionnée |
| `action-nudge-up` | Déplacer la sélection vers le haut |
| `action-nudge-down` | Déplacer la sélection vers le bas |
| `action-nudge-left` | Déplacer la sélection vers la gauche |
| `action-nudge-right` | Déplacer la sélection vers la droite |
| `action-center-on-screen` | Centrer la sélection à l'écran |
| `action-toggle-visible` | Basculer la visibilité |
| `action-toggle-gravity` | Basculer la gravité |
| `action-toggle-playback` | Basculer lecture / pause |
| `action-duplicate-selected` | Dupliquer la sélection |
| `action-reset-transform` | Réinitialiser l'échelle / l'opacité |
| `action-bring-forward` | Avancer la sélection |
| `action-send-backward` | Reculer la sélection |
| `action-fps-up` | Augmenter les FPS |
| `action-fps-down` | Diminuer les FPS |
| `action-opacity-up` | Augmenter l'opacité |
| `action-opacity-down` | Diminuer l'opacité |
| `action-cycle-monitor` | Changer l'écran de l'entité |
| `action-show-entity-info` | Afficher les détails de l'entité |
| `action-show-help` | Afficher l'aide clavier |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = Accessibilité
appearance-accesskit-label = Générer les mises à jour de l'arbre AccessKit
appearance-accesskit-hint = Alimente les lecteurs d'écran AT-SPI (Orca, etc.). Laisser activé ; désactiver uniquement pour réduire la consommation ou si votre bureau n'expose pas de bus AT-SPI.
```

## Suspected issues

### Apostrophe form

French UI strings should use the typographic apostrophe `’` rather than the straight `'` when feasible. Fluent files accept Unicode without issue. The placeholder strings above use straight `'` for ease of parsing; consider sweeping to `’` in a single pass.

### *Écran* vs *moniteur*

The pre-D translations use *moniteur* in monitor-related strings. *Écran* is more colloquial. For technical UI consistency, *moniteur* is correct in display-distribution contexts (`monitor-mode-*`), while user-facing labels (e.g. `action-cycle-monitor`) can use either. Keep one for each context.

### Lecture / mise en pause

For playback, *lecture* (play) and *pause* (pause) form the standard French verb pair; *basculer lecture / pause* in `action-toggle-playback` works for a toggle action.

## Open questions for native reviewer

- Use *raccourcis* or *raccourcis clavier* for `settings-tab-keybindings`? The shorter form risks ambiguity if other shortcut surfaces emerge later.
- Verb mood: imperative (`Ajouter`) vs infinitive (`Ajouter`) — both look identical for `-er` verbs but `Pulse / Appuyer` style differs. The suggested strings use a mix (`Appuyez sur…` is vous-form imperative). Keep this register everywhere unless a different convention dominates.
