# Deutsch — Basisübersetzung. Native-Speaker-Review steht aus.

app-name = animaEngine

settings-tab-inspector = Inspektor
settings-tab-scene = Szene
settings-tab-appearance = Darstellung
entity-count-zero = Keine Entitäten
entity-count-singular = { $n } Entität
entity-count-plural = { $n } Entitäten

inspector-section-position = Position
inspector-section-appearance = Darstellung
inspector-section-animation = Animation
animation-easing-label = Easing
easing-linear = Linear
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Sinus
easing-bounce-out = Bounce out
inspector-section-behavior = Verhalten
inspector-visible = Sichtbar
inspector-gravity = Schwerkraft
inspector-scale = Skalierung
inspector-opacity = Deckkraft
inspector-fps = FPS
inspector-playing = Wiedergabe
inspector-x = X
inspector-y = Y
inspector-z-index = z-Index
inspector-nothing-selected-headline = Nichts ausgewählt
inspector-nothing-selected-hint = Klicke eine Entität im Tab „Szene“ an oder drücke Tab, um sie durchzugehen.

behavior-idle = Untätig
behavior-walk = Umherlaufen
behavior-follow = Cursor folgen
behavior-wander = Begrenztes Wandern
behavior-bounce = Hüpfen
behavior-bounce-axis = Achse
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertikal
behavior-bounce-both = Beide (Kreis)

scene-empty-headline = Leere Szene
scene-empty-hint = Ziehe eine PNG- / GIF- / WebP- / MP4-Datei auf das Overlay — oder probiere unten ein Preset.
scene-drop-hint = Ziehe eine PNG- / GIF- / WebP-Datei auf das Overlay, um eine Entität hinzuzufügen.
scene-presets-header = Presets
scene-preset-append = Hinzufügen
scene-preset-replace = Ersetzen
scene-preset-replace-tooltip = Löscht die aktuelle Szene vor dem Hinzufügen

monitor-section-header = Monitore
monitor-mode-label = Verteilung
monitor-mode-per-monitor = Pro Monitor
monitor-mode-span = Über alle Monitore strecken
monitor-mode-single = Einzelner Monitor
monitor-pin-label = An Monitor binden
monitor-pin-auto = Auto (folgt der Position)
monitor-pinned-toast = Entität an { $name } gebunden
monitor-pin-cleared-toast = Entität folgt jetzt ihrer Position
monitor-no-monitors-detected = Keine Monitore erkannt

appearance-theme-header = Theme
appearance-theme-label = Theme
appearance-language-header = Sprache
theme-dark = Dunkel
theme-light = Hell
theme-dark-hc = Dunkel · Hoher Kontrast
theme-light-hc = Hell · Hoher Kontrast

onboarding-tabs = Einstellungen sind auf drei Tabs verteilt — Inspektor, Szene, Darstellung.
onboarding-quick-toggles = Tipp: V schaltet die Sichtbarkeit um, G die Schwerkraft — ohne dieses Panel zu öffnen.
onboarding-theme = Themes greifen sofort — kein Neustart nötig.
onboarding-dismiss = Schließen

menu-duplicate = Duplizieren
menu-reset-transform = Transform zurücksetzen
menu-toggle-gravity = Schwerkraft umschalten
menu-bring-forward = Nach vorne bringen
menu-send-backward = Nach hinten senden
menu-delete = Löschen

toggle-enter-edit = Bearbeitungsmodus aufrufen
toggle-exit-edit = Bearbeitungsmodus verlassen

palette-search-placeholder = Themes / Presets suchen…
palette-close-hint = Esc zum Schließen · Ctrl+K zum Umschalten
palette-switch-theme = Zum Theme { $theme } wechseln
palette-apply-preset = Preset anwenden: { $preset }

settings-tab-library = Bibliothek

# Asset library tab
library-empty-headline = Keine Assets indexiert
library-empty-hint = Lege Dateien in ~/.local/share/animaEngine/assets/ ab oder setze ANIMA_ASSETS_DIR.
library-no-asset-root = Kein Asset-Verzeichnis gefunden. Erstelle eines unter ~/.local/share/animaEngine/assets/
library-search-placeholder = Assets suchen…
library-add-to-scene = Zur Szene hinzufügen
library-sort-recent = Zuletzt
library-sort-name = Name
library-kind-image = Bild
library-kind-animated = Animiert
library-kind-video = Video
library-asset-added-toast = { $name } zur Szene hinzugefügt
library-asset-add-failed-toast = Konnte { $name } nicht hinzufügen
library-count = { $n } Assets indexiert

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

scene-empty-action-browse-presets = Browse presets
library-empty-action-copy-path = Copy path to clipboard

appearance-reset-onboarding-hint = Brings back the dismissed progressive hints and the "What's new" panel.
