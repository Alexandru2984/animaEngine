# Español — traducción base. Revisión por hablante nativo pendiente.

app-name = animaEngine

settings-tab-inspector = Inspector
settings-tab-scene = Escena
settings-tab-appearance = Apariencia
entity-count-zero = Sin entidades
entity-count-singular = { $n } entidad
entity-count-plural = { $n } entidades

inspector-section-position = Posición
inspector-section-appearance = Apariencia
inspector-section-animation = Animación
animation-easing-label = Easing
easing-linear = Lineal
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Seno
easing-bounce-out = Bounce out
inspector-section-behavior = Comportamiento
inspector-visible = Visible
inspector-gravity = Gravedad
inspector-scale = Escala
inspector-behavior-speed = Speed
inspector-behavior-comfort = Comfort distance
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Period
inspector-double-click-reset-hint = Double-click to reset to default.
inspector-opacity = Opacidad
inspector-fps = FPS
inspector-playing = Reproduciendo
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nada seleccionado
inspector-nothing-selected-hint = Haz clic en una entidad de la pestaña Escena, o presiona Tab para recorrerlas.

behavior-idle = En reposo
behavior-walk = Caminar
behavior-follow = Seguir el cursor
behavior-wander = Vagar acotado
behavior-bounce = Rebote
behavior-bounce-axis = Eje
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Ambos (círculo)

scene-empty-headline = Escena vacía
scene-empty-hint = Arrastra un PNG / GIF / WebP / MP4 sobre el overlay — o prueba un preset abajo.
scene-drop-hint = Arrastra un PNG / GIF / WebP sobre el overlay para añadir una entidad.
scene-presets-header = Presets
scene-preset-append = Añadir
scene-preset-replace = Reemplazar
scene-preset-replace-tooltip = Limpia la escena actual antes de añadir

monitor-section-header = Monitores
monitor-mode-label = Distribución
monitor-mode-per-monitor = Por monitor
monitor-mode-span = Extender en todos los monitores
monitor-mode-single = Un solo monitor
scene-window-awareness = Land on windows (X11)
scene-window-awareness-tooltip = Physics-enabled characters land on and walk along the top edges of your open windows. X11 sessions only — Wayland offers no window positions, so this does nothing there.
monitor-pin-label = Fijar al monitor
monitor-pin-auto = Auto (sigue la posición)
monitor-pinned-toast = Entidad fijada a { $name }
monitor-pin-cleared-toast = La entidad sigue ahora su posición
monitor-no-monitors-detected = No se detectaron monitores

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Idioma
theme-dark = Oscuro
theme-light = Claro
theme-dark-hc = Oscuro · Alto contraste
theme-light-hc = Claro · Alto contraste

onboarding-tabs = Los ajustes se reparten en tres pestañas — Inspector, Escena, Apariencia.
onboarding-quick-toggles = Consejo: V alterna la visibilidad, G alterna la gravedad — sin abrir este panel.
onboarding-theme = Los temas se aplican al instante — no hace falta reiniciar.
onboarding-coach-step1 = Welcome! Your characters live on the desktop. Click the gear button in the top-right corner to enter edit mode.
onboarding-coach-step2 = Drop a PNG, GIF, WebP or MP4 anywhere on the screen to add it as a character. The side panel edits everything you select.
onboarding-coach-step3 = Ctrl+K opens the command palette. Ctrl+Shift+A toggles edit mode from anywhere, Ctrl+Shift+H hides the overlay.
onboarding-coach-next = Next
onboarding-coach-skip = Skip tour
onboarding-coach-done = Got it
palette-replace-row = Replace scene with: { $preset }
palette-append-row = Append preset: { $preset }
palette-footer-hint = Esc to close · Ctrl+K to toggle · ↑↓ + Enter to pick
onboarding-dismiss = Cerrar

menu-duplicate = Duplicar
menu-reset-transform = Restablecer transformación
menu-toggle-gravity = Alternar gravedad
menu-bring-forward = Traer al frente
menu-send-backward = Enviar al fondo
menu-delete = Eliminar

toggle-enter-edit = Entrar al modo edición
toggle-exit-edit = Salir del modo edición

palette-search-placeholder = Escribe para buscar temas / presets…
palette-close-hint = Esc para cerrar · Ctrl+K para alternar
palette-switch-theme = Cambiar al tema { $theme }
palette-apply-preset = Aplicar preset: { $preset }

settings-tab-library = Biblioteca

# Asset library tab
library-empty-headline = Sin activos indexados
library-empty-hint = Arrastra archivos a ~/.local/share/animaEngine/assets/ o configura ANIMA_ASSETS_DIR.
library-no-asset-root = Directorio de assets no encontrado. Crea uno en ~/.local/share/animaEngine/assets/
library-search-placeholder = Buscar activos…
library-add-to-scene = Añadir a la escena
library-sort-recent = Recientes
library-sort-name = Nombre
library-kind-image = Imagen
library-kind-animated = Animado
library-kind-video = Video
library-asset-added-toast = { $name } añadido a la escena
library-asset-add-failed-toast = No se pudo añadir { $name }
library-count = { $n } activos indexados

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
appearance-reduced-motion-label = Reduce motion
appearance-reduced-motion-hint = Skips UI transitions (panel slide, fades, palette pop) and stops decorative bouncing. Animations that convey state still play.

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

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Shortcut permission was declined — using X11 hotkeys instead. Retry from the Keybindings tab.
portal-denied-native-toast = Shortcut permission was declined — the tray menu and compositor bindings still work.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Global shortcuts via:
keybindings-backend-tooltip = Which mechanism delivers the three global chords (toggle edit, hide, pause) while other apps have focus. Resolved at startup; in-app shortcuts are unaffected.
keybindings-portal-restart-hint = Trigger changes apply at the next launch (the desktop remembers your approval).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } disconnected — { $n } pinned entities now follow their position.
monitor-plugged-toast = Monitor { $name } connected.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Import Shimeji pack
library-import-shimeji-hint = Drop a pack folder onto the overlay, or paste its path here. Sprites are copied into the library.
library-import-shimeji-button = Import
shimeji-imported-toast = Imported { $name } ({ $n } parts skipped — see log)
shimeji-import-failed-toast = Import failed: { $reason }
shimeji-no-library-toast = No asset library root — create ~/.local/share/animaEngine/assets/ first.
crash-report-found-toast = The previous session crashed. A report was saved at { $path } — please attach it to a GitHub issue.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composed by group { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Config saved
toast-save-failed = Save failed: { $error }
toast-rejected = Rejected: { $reason }
toast-added = Added { $name }
toast-load-failed = Load failed: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Theme: { $theme }
toast-preset-entry-failed = Couldn’t add preset entry: { $error }
toast-preset-loaded = Loaded preset: { $name }
toast-duplicated = Duplicated { $name }
toast-duplicate-failed = Duplicate failed: { $error }
toast-deleted = Deleted { $name }
toast-playback-resumed = Playback resumed
toast-playback-paused = Playback paused
toast-wayland-no-library = Asset library not wired on the Wayland path yet
inspector-wander-box = Wander box
toast-perf-snapshot = Perf snapshot: { $path }
toast-perf-snapshot-failed = Snapshot failed: { $error }
