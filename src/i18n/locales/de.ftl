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
appearance-keyboard-header = Tastatur
appearance-keyboard-note = Nur lesbar in 0.2.0 — die Neubindung folgt in einer späteren Version.
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
