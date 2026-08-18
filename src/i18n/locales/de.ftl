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
animation-easing-label = Easing-Kurve
easing-linear = Linear
easing-ease-in-quad = Einblenden
easing-ease-out-quad = Ausblenden
easing-ease-in-out-quad = Ein-/Ausblenden
easing-sine = Sinus
easing-bounce-out = Bounce out
inspector-section-behavior = Verhalten
inspector-visible = Sichtbar
inspector-gravity = Schwerkraft
inspector-scale = Skalierung
inspector-behavior-speed = Geschwindigkeit
inspector-behavior-comfort = Komfortabstand
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Periode
inspector-double-click-reset-hint = Doppelklick setzt auf den Standardwert zurück.
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
scene-window-awareness = Auf Fenstern landen (X11)
scene-window-awareness-tooltip = Figuren mit aktiver Physik landen auf den Oberkanten Ihrer offenen Fenster und laufen daran entlang. Nur X11-Sitzungen — Wayland liefert keine Fensterpositionen, dort bewirkt das nichts.
monitor-pin-label = An Monitor binden
monitor-pin-auto = Auto (folgt der Position)
monitor-pinned-toast = Entität an { $name } gebunden
monitor-pin-cleared-toast = Entität folgt jetzt ihrer Position
monitor-no-monitors-detected = Keine Monitore erkannt

appearance-theme-header = Design
appearance-theme-label = Design
appearance-language-header = Sprache
theme-dark = Dunkel
theme-light = Hell
theme-dark-hc = Dunkel · Hoher Kontrast
theme-light-hc = Hell · Hoher Kontrast

onboarding-tabs = Einstellungen sind auf drei Tabs verteilt — Inspektor, Szene, Darstellung.
onboarding-quick-toggles = Tipp: V schaltet die Sichtbarkeit um, G die Schwerkraft — ohne dieses Panel zu öffnen.
onboarding-theme = Themes greifen sofort — kein Neustart nötig.
onboarding-coach-step1 = Willkommen! Ihre Figuren leben auf dem Desktop. Klicken Sie auf das Zahnrad oben rechts, um den Bearbeitungsmodus zu öffnen.
onboarding-coach-step2 = Ziehen Sie ein PNG, GIF, WebP oder MP4 irgendwo auf den Bildschirm, um es als Figur hinzuzufügen. Das Seitenpanel bearbeitet alles, was Sie auswählen.
onboarding-coach-step3 = Ctrl+K öffnet die Befehlspalette. Ctrl+Shift+A schaltet den Bearbeitungsmodus von überall um, Ctrl+Shift+H blendet das Overlay aus.
onboarding-coach-next = Weiter
onboarding-coach-skip = Tour überspringen
onboarding-coach-done = Verstanden
palette-replace-row = Szene ersetzen durch: { $preset }
palette-append-row = Preset anhängen: { $preset }
palette-footer-hint = Esc schließt · Ctrl+K schaltet um · ↑↓ + Enter wählt
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
settings-tab-keybindings = Kurzbefehle
keybindings-unbound = (nicht belegt)
keybindings-add = Hinzufügen
keybindings-recording = Tastenkombination drücken… (Esc bricht ab)
keybindings-conflict = Kollidiert mit { $action }
keybindings-reset-all = Alle auf Standard zurücksetzen
keybindings-help = Eigene Kurzbefehle werden in config.toml gespeichert

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Bearbeitungsmodus umschalten
action-hide-overlay = Overlay aus-/einblenden
action-pause-all = Alle Animationen pausieren
action-quit-with-save = Beenden (Konfiguration speichern)
action-save-now = Konfiguration jetzt speichern
action-open-command-palette = Befehlspalette
action-cycle-entity = Zur nächsten Figur wechseln
action-delete-selected = Ausgewählte Figur löschen
action-nudge-up = Auswahl nach oben schieben
action-nudge-down = Auswahl nach unten schieben
action-nudge-left = Auswahl nach links schieben
action-nudge-right = Auswahl nach rechts schieben
action-center-on-screen = Auswahl auf dem Bildschirm zentrieren
action-toggle-visible = Sichtbarkeit umschalten
action-toggle-gravity = Schwerkraft umschalten
action-toggle-playback = Wiedergabe/Pause umschalten
action-duplicate-selected = Auswahl duplizieren
action-reset-transform = Skalierung / Deckkraft zurücksetzen
action-bring-forward = Auswahl nach vorne holen
action-send-backward = Auswahl nach hinten stellen
action-fps-up = FPS erhöhen
action-fps-down = FPS verringern
action-opacity-up = Deckkraft erhöhen
action-opacity-down = Deckkraft verringern
action-cycle-monitor = Monitor-Anheftung durchschalten
action-show-entity-info = Figuren-Info anzeigen
action-show-help = Tastaturhilfe anzeigen

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Barrierefreiheit
appearance-accesskit-label = AccessKit-Baumaktualisierungen erzeugen
appearance-accesskit-hint = Versorgt AT-SPI-Screenreader (Orca usw.). Eingeschaltet lassen, außer Sie wollen weniger Ressourcen nutzen oder Ihr Desktop hat keinen AT-SPI-Bus. Hinweis: In Panels eingegebener Text erscheint auch auf dem AT-SPI-Bus, wo ihn jeder Prozess Ihres Benutzers lesen kann.
appearance-reduced-motion-label = Bewegung reduzieren
appearance-reduced-motion-hint = Überspringt UI-Übergänge (Panel-Gleiten, Überblendungen, Paletten-Pop) und stoppt dekoratives Wippen. Zustandsanzeigende Animationen laufen weiter.
appearance-hover-startle-label = Erschrecken bei Annäherung
appearance-hover-startle-hint = Maskottchen weichen dem Mauszeiger aus, wenn er nahe kommt, und beruhigen sich dann wieder. Cursor-Verfolgung gibt es nur unter X11, unter nativem Wayland reagiert dies nur im Bearbeitungsmodus.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Globale Hotkeys konnten nicht registriert werden (typisch für native Wayland-Sitzungen). Tray-Menü und ⚙-Knopf funktionieren weiter.
warning-hot-reload-disconnected = Der Hot-Reload-Worker wurde unerwartet beendet; laufende Konfigurationsänderungen greifen erst nach einem Neustart.
action-toggle-perf-overlay = Performance-Overlay umschalten

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Neu in 0.4
whats-new-keybindings = Neu belegbare Tastenkürzel — öffnen Sie den neuen Tab „Kurzbefehle“.
whats-new-collapse-state = Inspector-Abschnitte merken sich ihren Auf-/Zu-Zustand über Sitzungen hinweg.
whats-new-error-banners = Fehlerflächen (früher stumm) zeigen jetzt Toasts oder Banner — Sie sehen sie.
whats-new-accessibility-toggle = AccessKit lässt sich unter Erscheinungsbild → Barrierefreiheit abschalten.
onboarding-keybindings = Klicken Sie auf ein Kürzel, um es zu entfernen; drücken Sie eine Kombination, um ein neues aufzunehmen.
onboarding-perf-overlay = Ctrl+Shift+` öffnet das Live-Performance-Overlay.
appearance-reset-onboarding = Einführungshinweise zurücksetzen

scene-empty-action-browse-presets = Presets durchstöbern
library-empty-action-copy-path = Pfad in die Zwischenablage kopieren

appearance-reset-onboarding-hint = Holt die ausgeblendeten Hinweise und das „Neuigkeiten“-Panel zurück.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Berechtigung für Kurzbefehle abgelehnt — es werden X11-Hotkeys verwendet. Erneut versuchen im Tab „Kurzbefehle“.
portal-denied-native-toast = Berechtigung für Kurzbefehle abgelehnt — Tray-Menü und Compositor-Bindungen funktionieren weiter.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Globale Kurzbefehle über:
keybindings-backend-tooltip = Welcher Mechanismus die drei globalen Kürzel (Bearbeiten, Ausblenden, Pause) liefert, während andere Apps den Fokus haben. Wird beim Start ermittelt; In-App-Kürzel sind nicht betroffen.
keybindings-portal-restart-hint = Trigger-Änderungen gelten ab dem nächsten Start (der Desktop merkt sich Ihre Freigabe).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } getrennt — { $n } angeheftete Figuren folgen jetzt ihrer Position.
monitor-plugged-toast = Monitor { $name } verbunden.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Shimeji-Paket importieren
library-import-shimeji-hint = Paketordner aufs Overlay ziehen oder den Pfad hier einfügen. Sprites werden in die Bibliothek kopiert.
library-import-shimeji-button = Importieren
shimeji-imported-toast = { $name } importiert ({ $n } Teile übersprungen — siehe Log)
shimeji-import-failed-toast = Import fehlgeschlagen: { $reason }
shimeji-no-library-toast = Kein Bibliotheksverzeichnis — legen Sie zuerst ~/.local/share/animaEngine/assets/ an.
crash-report-found-toast = Die letzte Sitzung ist abgestürzt. Ein Bericht wurde unter { $path } gespeichert — bitte an ein GitHub-Issue anhängen.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Komponiert durch Gruppe { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Konfiguration gespeichert
toast-save-failed = Speichern fehlgeschlagen: { $error }
toast-rejected = Abgelehnt: { $reason }
toast-added = { $name } hinzugefügt
toast-load-failed = Laden fehlgeschlagen: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Design: { $theme }
toast-preset-entry-failed = Preset-Eintrag konnte nicht hinzugefügt werden: { $error }
toast-preset-loaded = Preset geladen: { $name }
toast-duplicated = { $name } dupliziert
toast-duplicate-failed = Duplizieren fehlgeschlagen: { $error }
toast-deleted = { $name } gelöscht
toast-playback-resumed = Wiedergabe fortgesetzt
toast-playback-paused = Wiedergabe pausiert
inspector-wander-box = Streifbereich
toast-perf-snapshot = Performance-Snapshot: { $path }
toast-perf-snapshot-failed = Snapshot fehlgeschlagen: { $error }
