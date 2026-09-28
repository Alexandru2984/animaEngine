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
inspector-nothing-selected-hint = Klicken Sie eine Entität im Tab „Szene“ an oder drücken Sie Tab, um sie durchzugehen.

behavior-idle = Untätig
behavior-walk = Umherlaufen
behavior-follow = Cursor folgen
behavior-wander = Begrenztes Wandern
behavior-bounce = Hüpfen
behavior-bounce-axis = Achse
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertikal
behavior-bounce-both = Beide (Kreis)
behavior-script = Skript
behavior-script-path-label = Pfad
behavior-script-path-hint = Relativ zu Ihrer Asset-Bibliothek — { $path }
behavior-script-params = Parameter
behavior-script-param-name = Name
behavior-script-add-param = Hinzufügen
behavior-script-remove-param = Diesen Parameter entfernen
script-failed-toast = Verhaltensskript { $script } fehlgeschlagen: { $error }

scene-empty-headline = Leere Szene
scene-empty-hint = Ziehen Sie eine PNG- / GIF- / WebP- / MP4-Datei auf das Overlay — oder probieren Sie unten ein Preset.
scene-drop-hint = Ziehen Sie eine PNG- / GIF- / WebP-Datei auf das Overlay, um eine Entität hinzuzufügen.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Datei hinzufügen …
scene-add-file-tooltip = Bilder oder Videos auswählen, die als Figuren hinzugefügt werden.
file-chooser-title = Figuren hinzufügen
file-chooser-filter = Bilder und Videos
file-chooser-unavailable-toast = Die Dateiauswahl konnte nicht geöffnet werden. Dafür wird xdg-desktop-portal benötigt.
scene-presets-header = Presets
scene-groups-header = Gruppen
scene-preset-append = Hinzufügen
scene-preset-replace = Ersetzen
scene-preset-replace-tooltip = Löscht die aktuelle Szene vor dem Hinzufügen

monitor-section-header = Monitore
monitor-mode-label = Verteilung
monitor-mode-per-monitor = Pro Monitor
monitor-mode-span = Über alle Monitore strecken
monitor-mode-span-unsupported = Auf diesem Backend kann ein Overlay nicht mehrere Monitore abdecken — nutze stattdessen „pro Monitor“.
monitor-mode-single = Einzelner Monitor
scene-window-awareness = Auf Fenstern landen (X11)
scene-window-awareness-tooltip = Figuren mit aktiver Physik landen auf den Oberkanten Ihrer offenen Fenster und laufen daran entlang. Nur X11-Sitzungen — Wayland liefert keine Fensterpositionen, dort bewirkt das nichts.
scene-window-awareness-unavailable = Auf nativem Wayland nicht verfügbar — kein Protokoll gibt Fensterpositionen preis.
monitor-pin-label = An Monitor binden
monitor-pin-auto = Auto (folgt der Position)
monitor-pinned-toast = Entität an { $name } gebunden
monitor-pin-cleared-toast = Entität folgt jetzt ihrer Position
monitor-no-monitors-detected = Keine Monitore erkannt

appearance-theme-header = Design
appearance-theme-label = Design
appearance-language-header = Sprache
appearance-language-no-font = Für diese Schrift ist keine Schriftart installiert — installiere zuerst ein Noto-CJK-Paket.
theme-dark = Dunkel
theme-light = Hell
theme-dark-hc = Dunkel · Hoher Kontrast
theme-light-hc = Hell · Hoher Kontrast

onboarding-tabs = Die Einstellungen verteilen sich auf fünf Tabs — Inspektor, Szene, Bibliothek, Darstellung, Kurzbefehle.
onboarding-quick-toggles = Tipp: V schaltet die Sichtbarkeit um, G die Schwerkraft — ohne dieses Panel zu öffnen.
onboarding-theme = Themes greifen sofort — kein Neustart nötig.
onboarding-coach-step1 = Willkommen! Ihre Figuren leben auf dem Desktop. Klicken Sie auf das Zahnrad oben rechts, um den Bearbeitungsmodus zu öffnen.
onboarding-coach-step2 = Ziehen Sie ein PNG, GIF, WebP oder MP4 irgendwo auf den Bildschirm, um es als Figur hinzuzufügen. Das Seitenpanel bearbeitet alles, was Sie auswählen.
onboarding-coach-step3 = Strg+K öffnet die Befehlspalette. Strg+Shift+A schaltet den Bearbeitungsmodus von überall um, Strg+Shift+H blendet das Overlay aus.
onboarding-coach-next = Weiter
onboarding-coach-skip = Tour überspringen
onboarding-coach-done = Verstanden
palette-replace-row = Szene ersetzen durch: { $preset }
palette-append-row = Preset anhängen: { $preset }
palette-footer-hint = Esc schließt · Strg+K schaltet um · ↑↓ + Enter wählt
onboarding-dismiss = Schließen

menu-duplicate = Duplizieren
menu-reset-transform = Transform zurücksetzen
menu-toggle-gravity = Schwerkraft umschalten
menu-bring-forward = Nach vorne bringen
menu-send-backward = Nach hinten senden
menu-delete = Löschen

toggle-enter-edit = Bearbeitungsmodus aufrufen
toggle-exit-edit = Bearbeitungsmodus verlassen

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Aktionen, Themes und Presets suchen…
palette-switch-theme = Zum Theme { $theme } wechseln

settings-tab-library = Bibliothek

# Asset library tab
library-empty-headline = Keine Assets indexiert
library-empty-hint = Legen Sie Dateien in { $path } ab oder setzen Sie ANIMA_ASSETS_DIR.
library-no-asset-root = Kein Asset-Verzeichnis gefunden. Erstellen Sie eines unter { $path }
library-search-placeholder = Assets suchen…
library-add-to-scene = Zur Szene hinzufügen
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
keybindings-reset-one = Auf Standard zurücksetzen
keybindings-remove-chord = Diese Zuweisung entfernen
keybindings-help = Eigene Kurzbefehle werden in config.toml gespeichert
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Strg
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

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
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Lässt Screenreader (Orca und andere, über AT-SPI) die Panels lesen und bedienen. Ausgeschaltet sieht ein Screenreader ein leeres Fenster. Hinweis: Solange einer läuft, steht in Panels eingegebener Text auf dem AT-SPI-Bus, wo ihn jeder Prozess Ihres Benutzers lesen kann.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Screenreader werden auf diesem System noch nicht unterstützt.
appearance-reduced-motion-label = Bewegung reduzieren
appearance-reduced-motion-hint = Überspringt UI-Übergänge (Panel-Gleiten, Überblendungen, Paletten-Pop) und stoppt dekoratives Wippen. Zustandsanzeigende Animationen laufen weiter.
appearance-hover-startle-label = Erschrecken bei Annäherung
appearance-hover-startle-hint = Maskottchen weichen dem Mauszeiger aus, wenn er nahe kommt. Cursor-Verfolgung gibt es nur unter X11, unter nativem Wayland reagiert dies nur im Bearbeitungsmodus.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Globale Hotkeys konnten nicht registriert werden (typisch für native Wayland-Sitzungen). Tray-Menü und ⚙-Knopf funktionieren weiter.
warning-hot-reload-disconnected = Der Hot-Reload-Worker wurde unerwartet beendet; laufende Konfigurationsänderungen greifen erst nach einem Neustart.
# Machine-translated, pending native review.
warning-config-unreadable = Ihre Konfigurationsdatei konnte nicht gelesen werden, daher wurde die Standardszene geladen. Das Original liegt daneben als config.toml.bak-corrupt.
action-toggle-perf-overlay = Performance-Overlay umschalten

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Neu in 1.4
whats-new-add-file = „Datei hinzufügen …“ im Tab Szene fügt Figuren über die Dateiauswahl Ihres Desktops hinzu – ganz ohne Ziehen.
whats-new-palette = Strg+K führt jetzt jede Aktion aus und zeigt ihr Tastenkürzel daneben.
whats-new-screen-readers = Screenreader wie Orca können das Einstellungsfenster jetzt vorlesen und bedienen.
onboarding-keybindings = Klicken Sie auf ein Kürzel, um es zu entfernen; drücken Sie eine Kombination, um ein neues aufzunehmen.
onboarding-perf-overlay = Strg+Shift+` öffnet das Live-Performance-Overlay.
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
shimeji-no-library-toast = Kein Bibliotheksverzeichnis — legen Sie zuerst { $path } an.
crash-report-found-toast = Die letzte Sitzung ist abgestürzt. Ein Bericht wurde unter { $path } gespeichert — bitte an ein GitHub-Issue anhängen.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Komponiert durch Gruppe { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Konfiguration gespeichert
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Konfiguration von der Festplatte neu geladen
toast-config-reload-failed = Konfiguration nicht neu geladen — die Datei ist ungültig oder wird noch geschrieben. Die aktuelle Szene bleibt.
toast-config-reload-discarded = Konfiguration nicht neu geladen — Sie haben die Szene während des Ladens bearbeitet.
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
