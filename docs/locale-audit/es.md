# Locale audit — Español (`es`)

**Status:** partial. Pre-D.1 strings translated previously; D.1.6, D.1.7, D.3 keys carry English placeholder.
**AI cross-check confidence:** high (the model has solid Spanish technical-UI familiarity for surface-level translation; regional register and idiom still need a human pass).

## Glossary

| English | Español (recommended) | Notes |
|---|---|---|
| overlay | **superposición** | |
| scene | **escena** | |
| entity | **entidad** | |
| edit mode | **modo edición** | |
| chord (key combo) | **combinación de teclas** | |
| library | **biblioteca** | |
| monitor pin | **fijar al monitor** | verb form |
| preset | **preajuste** / **preset** | Both used; *preajuste* is more formal. |

## Placeholder English strings — proposed translations

### D.1.6 — Keybindings tab UI

```ftl
settings-tab-keybindings = Atajos de teclado
keybindings-unbound = (sin asignar)
keybindings-add = Añadir
keybindings-recording = Pulsa una combinación… (Esc para cancelar)
keybindings-conflict = Conflicto con { $action }
keybindings-reset-all = Restablecer todo a los valores por defecto
keybindings-help = Los atajos personalizados se guardan en config.toml
```

### D.1.7 — Action labels

| key | suggested Spanish |
|---|---|
| `action-toggle-edit-mode` | Alternar modo edición |
| `action-hide-overlay` | Ocultar / mostrar superposición |
| `action-pause-all` | Pausar todas las animaciones |
| `action-quit-with-save` | Salir (guardar configuración) |
| `action-save-now` | Guardar configuración ahora |
| `action-open-command-palette` | Paleta de comandos |
| `action-cycle-entity` | Pasar a la siguiente entidad |
| `action-delete-selected` | Eliminar entidad seleccionada |
| `action-nudge-up` | Mover selección arriba |
| `action-nudge-down` | Mover selección abajo |
| `action-nudge-left` | Mover selección a la izquierda |
| `action-nudge-right` | Mover selección a la derecha |
| `action-center-on-screen` | Centrar selección en pantalla |
| `action-toggle-visible` | Alternar visibilidad |
| `action-toggle-gravity` | Alternar gravedad |
| `action-toggle-playback` | Alternar reproducción |
| `action-duplicate-selected` | Duplicar selección |
| `action-reset-transform` | Restablecer escala / opacidad |
| `action-bring-forward` | Traer la selección al frente |
| `action-send-backward` | Enviar la selección atrás |
| `action-fps-up` | Aumentar FPS |
| `action-fps-down` | Disminuir FPS |
| `action-opacity-up` | Aumentar opacidad |
| `action-opacity-down` | Disminuir opacidad |
| `action-cycle-monitor` | Cambiar el monitor de la entidad |
| `action-show-entity-info` | Mostrar información de la entidad |
| `action-show-help` | Mostrar ayuda de teclado |

### D.3 — Accessibility section

```ftl
appearance-accessibility-header = Accesibilidad
appearance-accesskit-label = Generar actualizaciones del árbol AccessKit
appearance-accesskit-hint = Alimenta los lectores de pantalla AT-SPI (Orca, etc.). Déjalo activado; desactívalo solo si quieres reducir el consumo o tu escritorio no tiene un bus AT-SPI.
```

## Suspected issues

### Verb register: tuteo vs voseo

Spanish UI text typically uses **tú** (familiar second-person). The proposed strings above use imperative + tú implicitly. If LatAm reviewers prefer **usted** for formality, switch consistently:

- Tú: `Pulsa una combinación…`
- Usted: `Pulse una combinación…`

Pick one across the file. Current EN-first translations look like tú; keep that as house style unless we hear otherwise.

### `action-pause-all` — *pausar* vs *detener*

"Pause all animations" → *pausar* fits because the action is reversible. *Detener* would imply a hard stop. Keeping *pausar*.

## Open questions for native reviewer

- *Atajos* vs *atajos de teclado*: drop the redundancy if context is clear?
- Regional: prefer *ordenador* (Spain) vs *computadora* (LatAm) anywhere? animaEngine is desktop-focused; the only computer term in scope is *escritorio* which is already neutral.
- *Preajuste* vs *preset*: pick one and apply to scene-presets-header (currently *Preajustes* / *Presets* depending on past pass).
