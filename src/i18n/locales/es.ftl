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
monitor-pin-label = Fijar al monitor
monitor-pin-auto = Auto (sigue la posición)
monitor-pinned-toast = Entidad fijada a { $name }
monitor-pin-cleared-toast = La entidad sigue ahora su posición
monitor-no-monitors-detected = No se detectaron monitores

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Idioma
appearance-keyboard-header = Teclado
appearance-keyboard-note = Solo lectura en 0.2.0 — el rebinding llegará en una versión posterior.
theme-dark = Oscuro
theme-light = Claro
theme-dark-hc = Oscuro · Alto contraste
theme-light-hc = Claro · Alto contraste

onboarding-tabs = Los ajustes se reparten en tres pestañas — Inspector, Escena, Apariencia.
onboarding-quick-toggles = Consejo: V alterna la visibilidad, G alterna la gravedad — sin abrir este panel.
onboarding-theme = Los temas se aplican al instante — no hace falta reiniciar.
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
appearance-accesskit-hint = Powers AT-SPI screen readers (Orca etc.). Leave on unless you want a tighter footprint or your desktop doesn't run an AT-SPI bus.

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
