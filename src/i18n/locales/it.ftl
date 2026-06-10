# Italiano — traduzione di base. Revisione madrelingua in sospeso.

app-name = animaEngine

settings-tab-inspector = Ispettore
settings-tab-scene = Scena
settings-tab-appearance = Aspetto
entity-count-zero = Nessuna entità
entity-count-singular = { $n } entità
entity-count-plural = { $n } entità

inspector-section-position = Posizione
inspector-section-appearance = Aspetto
inspector-section-animation = Animazione
animation-easing-label = Easing
easing-linear = Lineare
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Seno
easing-bounce-out = Bounce out
inspector-section-behavior = Comportamento
inspector-visible = Visibile
inspector-gravity = Gravità
inspector-scale = Scala
inspector-opacity = Opacità
inspector-fps = FPS
inspector-playing = In riproduzione
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nessuna selezione
inspector-nothing-selected-hint = Clicca un'entità nella scheda Scena, o premi Tab per scorrerle.

behavior-idle = In riposo
behavior-walk = Cammina
behavior-follow = Segui il cursore
behavior-wander = Vagare entro limiti
behavior-bounce = Rimbalzo
behavior-bounce-axis = Asse
behavior-bounce-horizontal = Orizzontale
behavior-bounce-vertical = Verticale
behavior-bounce-both = Entrambi (cerchio)

scene-empty-headline = Scena vuota
scene-empty-hint = Trascina un PNG / GIF / WebP / MP4 sull'overlay — o prova un preset qui sotto.
scene-drop-hint = Trascina un PNG / GIF / WebP sull'overlay per aggiungere un'entità.
scene-presets-header = Preset
scene-preset-append = Aggiungi
scene-preset-replace = Sostituisci
scene-preset-replace-tooltip = Cancella la scena attuale prima di aggiungere

monitor-section-header = Monitor
monitor-mode-label = Distribuzione
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Estendi su tutti i monitor
monitor-mode-single = Monitor singolo
monitor-pin-label = Fissa al monitor
monitor-pin-auto = Auto (segue la posizione)
monitor-pinned-toast = Entità fissata a { $name }
monitor-pin-cleared-toast = L'entità segue ora la sua posizione
monitor-no-monitors-detected = Nessun monitor rilevato

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Lingua
theme-dark = Scuro
theme-light = Chiaro
theme-dark-hc = Scuro · Contrasto elevato
theme-light-hc = Chiaro · Contrasto elevato

onboarding-tabs = Le impostazioni sono divise su tre schede — Ispettore, Scena, Aspetto.
onboarding-quick-toggles = Suggerimento: V alterna la visibilità, G la gravità — senza aprire questo pannello.
onboarding-theme = I temi si applicano subito — nessun riavvio richiesto.
onboarding-dismiss = Chiudi

menu-duplicate = Duplica
menu-reset-transform = Reimposta trasformazione
menu-toggle-gravity = Attiva/disattiva gravità
menu-bring-forward = Porta in primo piano
menu-send-backward = Manda in fondo
menu-delete = Elimina

toggle-enter-edit = Entra in modalità modifica
toggle-exit-edit = Esci dalla modalità modifica

palette-search-placeholder = Cerca temi / preset…
palette-close-hint = Esc per chiudere · Ctrl+K per alternare
palette-switch-theme = Passa al tema { $theme }
palette-apply-preset = Applica preset: { $preset }

settings-tab-library = Libreria

# Asset library tab
library-empty-headline = Nessun asset indicizzato
library-empty-hint = Trascina file in ~/.local/share/animaEngine/assets/ o imposta ANIMA_ASSETS_DIR.
library-no-asset-root = Nessuna directory di asset trovata. Creane una in ~/.local/share/animaEngine/assets/
library-search-placeholder = Cerca asset…
library-add-to-scene = Aggiungi alla scena
library-sort-recent = Recenti
library-sort-name = Nome
library-kind-image = Immagine
library-kind-animated = Animato
library-kind-video = Video
library-asset-added-toast = { $name } aggiunto alla scena
library-asset-add-failed-toast = Impossibile aggiungere { $name }
library-count = { $n } asset indicizzati

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Keybindings
keybindings-unbound = (unbound)
keybindings-add = Add
keybindings-recording = Press a chord… (Esc to cancel)
keybindings-conflict = Conflicts with { $action }
keybindings-reset-all = Reset all to defaults
keybindings-help = Custom shortcuts persist in config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Toggle edit mode
action-hide-overlay = Hide / show overlay
action-pause-all = Pause all animations
action-quit-with-save = Quit (save config)
action-save-now = Save config now
action-open-command-palette = Command palette
action-cycle-entity = Cycle to next entity
action-delete-selected = Delete selected entity
action-nudge-up = Nudge selection up
action-nudge-down = Nudge selection down
action-nudge-left = Nudge selection left
action-nudge-right = Nudge selection right
action-center-on-screen = Center selection on screen
action-toggle-visible = Toggle visibility
action-toggle-gravity = Toggle gravity
action-toggle-playback = Toggle play/pause
action-duplicate-selected = Duplicate selection
action-reset-transform = Reset scale / opacity
action-bring-forward = Bring selection forward
action-send-backward = Send selection backward
action-fps-up = Increase FPS
action-fps-down = Decrease FPS
action-opacity-up = Increase opacity
action-opacity-down = Decrease opacity
action-cycle-monitor = Cycle entity monitor pin
action-show-entity-info = Show entity info
action-show-help = Show keyboard help

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Accessibility
appearance-accesskit-label = Generate AccessKit tree updates
appearance-accesskit-hint = Powers AT-SPI screen readers (Orca etc.). Leave on unless you want a tighter footprint or your desktop doesn't run an AT-SPI bus. Note: text you type in panels also appears on the AT-SPI bus, where any process running as your user can read it.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Global hotkeys couldn't register (typical on a native Wayland session). The tray menu and the ⚙ button still work.
warning-hot-reload-disconnected = The hot-reload worker stopped unexpectedly; in-flight config edits won't apply until you restart the app.
action-toggle-perf-overlay = Toggle perf overlay

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = What's new in 0.4
whats-new-keybindings = Rebindable keyboard shortcuts — open the new Keybindings tab.
whats-new-collapse-state = Inspector sections remember their open/closed state across sessions.
whats-new-error-banners = Failure surfaces (silent before) now toast or banner — you'll see them.
whats-new-accessibility-toggle = AccessKit can be turned off from Appearance → Accessibility.
onboarding-keybindings = Click any chord to remove it; press a key combo to record a new one.
onboarding-perf-overlay = Press Ctrl+Shift+` to open the live perf overlay.
appearance-reset-onboarding = Reset onboarding hints

scene-empty-action-browse-presets = Browse presets
library-empty-action-copy-path = Copy path to clipboard

appearance-reset-onboarding-hint = Brings back the dismissed progressive hints and the "What's new" panel.
