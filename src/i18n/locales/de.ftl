# Deutsch — Basisübersetzung. Native-Speaker-Review steht aus.

app-name = animaEngine

settings-tab-inspector = Inspektor
settings-tab-scene = Szene
settings-tab-appearance = Darstellung
entity-count-zero = Keine Figuren
entity-count-singular = { $n } Figur
entity-count-plural = { $n } Figuren

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
inspector-section-talk = Sagt
inspector-talk-hint = Ein Satz pro Zeile. Ein Antippen der Figur im Bearbeitungsmodus lässt sie einen sagen, und sie kann sich auch von selbst melden.
inspector-talk-placeholder = Hallo!
inspector-talk-pace = Meldet sich
inspector-talk-only-poked = Nur beim Antippen
inspector-talk-every = Etwa alle { $minutes } Min.
inspector-talk-say-now = Jetzt einen sagen
placeholders-hint = In einer Zeile werden { $time }, { $day }, { $date } und { $name } beim Sprechen ausgefüllt.
placeholder-date = { $day }. { $month }
weekday-0 = Sonntag
weekday-1 = Montag
weekday-2 = Dienstag
weekday-3 = Mittwoch
weekday-4 = Donnerstag
weekday-5 = Freitag
weekday-6 = Samstag
month-1 = Januar
month-2 = Februar
month-3 = März
month-4 = April
month-5 = Mai
month-6 = Juni
month-7 = Juli
month-8 = August
month-9 = September
month-10 = Oktober
month-11 = November
month-12 = Dezember
inspector-sound-label = Klang beim Antippen
inspector-sound-none = Keiner
inspector-sound-play = Abspielen
inspector-sound-hint = Klangdateien (.ogg, .wav, .flac, .mp3) in { $path } erscheinen hier.
inspector-bubble-colour = Farbe der Sprechblase
inspector-bubble-size = Textgröße
inspector-bubble-reset = Standard
inspector-hours-label = Nur zu bestimmten Zeiten
inspector-hours-hint = Außerhalb dieser Zeiten ist die Figur nicht zu sehen — das Gespenst nachts, die Sonne am Tag. Im Bearbeitungsmodus sind alle da, um bearbeitet zu werden.
inspector-hours-from = von
inspector-hours-to = bis
inspector-visible = Sichtbar
inspector-gravity = Schwerkraft
inspector-scale = Skalierung
inspector-behavior-speed = Geschwindigkeit
inspector-follow-target = Folgt
inspector-follow-distance = Abstand
inspector-follow-nobody = (niemandem)
inspector-follow-none-free = Niemand zum Folgen: Die anderen fehlen oder folgen bereits dieser Figur.
inspector-tag-partner = Spielt mit
inspector-tag-hint = Sie jagen einander eine Weile — wer dran ist, läuft dem anderen nach — und ruhen dann anderthalb Minuten. Die andere Figur braucht keine eigene Einstellung.
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
inspector-nothing-selected-hint = Klicken Sie eine Figur im N „Szene“ an oder drücken Sie Tab, um sie durchzugehen.

behavior-idle = Untätig
behavior-walk = Umherlaufen
behavior-follow = Cursor folgen
behavior-follow-character = Einer Figur folgen
behavior-play-tag = Fangen spielen
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
sound-failed-toast = Der Klang { $sound } konnte nicht abgespielt werden: { $error }

scene-empty-headline = Leere Szene
scene-empty-hint = Ziehen Sie eine PNG- / GIF- / WebP- / MP4-Datei auf das Overlay — oder probieren Sie unten ein Preset.
scene-drop-hint = Ziehen Sie eine PNG- / GIF- / WebP-Datei auf das Overlay, um eine Figur hinzuzufügen.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Datei hinzufügen …
scene-add-file-tooltip = Bilder oder Videos auswählen, die als Figuren hinzugefügt werden.
scene-paste = Einfügen
scene-paste-tooltip = Die kopierten Figuren in diese Szene einfügen. Kopiert: { $count }
scene-paste-empty = Ein anderswo kopiertes Bild oder Dateien einfügen – oder Figuren: auswählen und Strg+C drücken oder per Rechtsklick „Kopieren“ wählen, in dieser oder einer anderen Szene.
file-chooser-title = Figuren hinzufügen
file-chooser-filter = Bilder und Videos
share-chooser-title = Szene teilen
gif-chooser-title = GIF speichern
character-chooser-title = Eine Figur teilen
backup-chooser-title = animaEngine sichern
restore-chooser-title = Eine Sicherung wiederherstellen
restore-chooser-filter = animaEngine-Sicherungen
import-chooser-title = Szene importieren
import-chooser-filter = animaEngine-Szenen und -Figuren
file-chooser-unavailable-toast = Die Dateiauswahl konnte nicht geöffnet werden. Dafür wird xdg-desktop-portal benötigt.
scene-presets-header = Presets
scene-groups-header = Gruppen
scene-characters-header = Figuren
scene-together-header = Miteinander
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
monitor-pinned-toast = Figur an { $name } gebunden
monitor-pin-cleared-toast = Figur folgt jetzt ihrer Position
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
menu-copy = Kopieren
menu-save-gif = Als GIF speichern…
menu-share-character = Figur teilen…
menu-cut = Ausschneiden
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
action-toggle-focus = Fokus-Timer starten oder stoppen
action-toggle-snooze = 15 Minuten schlummern oder aufwachen
action-toggle-mute = Töne stumm- oder einschalten
action-quit-with-save = Beenden (Konfiguration speichern)
action-save-now = Konfiguration jetzt speichern
action-open-command-palette = Befehlspalette
# Undo (1.5): machine-translated, pending native review.
action-undo = Rückgängig
action-redo = Wiederholen

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Auswahl gruppieren
action-ungroup-selected = Gruppierung der Auswahl aufheben
menu-group = Gruppieren
menu-ungroup = Gruppierung aufheben
group-default-name = Gruppe { $number }
group-copy-name = { $name } (Kopie)
toast-grouped = Gruppiert als { $name }
toast-ungrouped = Gruppen aufgelöst: { $count }
toast-nothing-to-ungroup = Nichts Ausgewähltes gehört zu einer Gruppe
scene-groups-empty-hint = Mehrere Figuren auswählen und Strg+G drücken – oder per Rechtsklick „Gruppieren“ wählen.
scene-group-members = Figuren: { $count }
scene-group-select-tooltip = Die Figuren dieser Gruppe auswählen
scene-group-rename = Gruppe umbenennen
scene-group-show = Gruppe einblenden
scene-group-hide = Gruppe ausblenden
scene-group-ungroup = Gruppierung aufheben – die Figuren bleiben

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Linke Kanten ausrichten
arrange-align-center = Auf einer senkrechten Linie zentrieren
arrange-align-right = Rechte Kanten ausrichten
arrange-align-top = Obere Kanten ausrichten
arrange-align-middle = Auf einer waagerechten Linie zentrieren
arrange-align-bottom = Untere Kanten ausrichten
arrange-distribute-horizontally = Gleichmäßig nebeneinander verteilen
arrange-distribute-vertically = Gleichmäßig untereinander verteilen
scene-snap = Beim Ziehen einrasten
scene-snap-tooltip = Kanten und Mitten rasten an denen der Bildschirme und anderer Figuren ein. Beim Ziehen Alt halten, um frei zu platzieren.
scene-bump = Figuren stoßen aneinander
scene-bump-tooltip = Mit Schwerkraft (G) landen Figuren auf den Köpfen anderer; laufende Figuren kehren um, wenn sie sich treffen.
scene-greet = Figuren grüßen sich, wenn sie sich treffen
scene-greet-tooltip = Zwei, die zusammenkommen — beim Laufen, beim Folgen, wenn eine auf der anderen landet —, bleiben kurz stehen und grüßen sich, mit einem Herz. Nicht zu oft.
greet-hi = Hallo!
greet-hello = Hi!
greet-hey = Hey!

action-cycle-entity = Zur nächsten Figur wechseln
action-select-all = Alle Figuren auswählen
action-open-context-menu = Menü der Auswahl öffnen
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
action-copy-selected = Auswahl kopieren
action-cut-selected = Auswahl ausschneiden
action-paste = Einfügen
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
appearance-large-bubbles-label = Großer Text in Sprechblasen
appearance-large-bubbles-hint = Der Text aller Sprechblasen ist größer, zum Lesen aus der Entfernung oder bei Sehschwäche.
appearance-hover-startle-label = Erschrecken bei Annäherung
appearance-hover-startle-hint = Maskottchen weichen dem Mauszeiger aus, wenn er nahe kommt. Cursor-Verfolgung gibt es nur unter X11, unter nativem Wayland reagiert dies nur im Bearbeitungsmodus.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Wenn eine App im Vollbild ist
appearance-fullscreen-hint = Spiele, Videos und Präsentationen, die den Bildschirm füllen. Funktioniert unter X11 und auf Wayland-Compositors wie sway und Hyprland; unter GNOME und KDE mit Wayland nur für Apps, die über XWayland laufen.
appearance-apps-label = Auch bei diesen Apps ausblenden
appearance-apps-hint = Solange eine davon das Fenster im Vordergrund ist – ein Videoanruf, ein Stream, ein Spiel –, werden die Figuren ausgeblendet, wie bei einer Vollbild-App. X11 und Wayland-Compositoren wie sway und Hyprland; unter GNOME und KDE mit Wayland nur Apps, die über XWayland laufen.
appearance-apps-remove = { $app } entfernen
appearance-apps-add = Geöffnete App hinzufügen…
appearance-apps-none-open = Keine App, die diese Sitzung sehen kann, ist geöffnet.
appearance-behind-windows-label = Hinter den Fenstern
appearance-behind-windows-hint = Die Figuren leben auf dem Desktop unter den Fenstern statt darüber. Der Bearbeitungsmodus holt sie nach vorn; ist die ⚙-Schaltfläche verdeckt, öffnen ihn Strg+Shift+A oder das Tray.
appearance-tap-label = Figuren lassen sich antippen
appearance-tap-hint = Auch außerhalb des Bearbeitungsmodus: Ein Antippen stupst die Figur an – ein Hüpfer und ein eigener Satz. Klicks überall sonst erreichen weiterhin Ihre Fenster, und die App, in der Sie tippen, behält die Tastatur.
appearance-effects-label = Kleine Effekte
appearance-effects-hint = Herzen, wenn eine Figur angetippt wird, eine Staubwolke, wo eine hart landet, und Zs, wenn sie einschlafen. Keine mit „Bewegung reduzieren“.
appearance-bedtime-label = Schlafenszeit
appearance-bedtime-hint = Die Figuren dösen zwischen diesen Uhrzeiten ein, wer auch immer an der Tastatur sitzt; der Bearbeitungsmodus weckt sie, solange er dauert. Die Nacht zählt für die Erinnerungen als Pause.
appearance-moods-label = Stimmung nach Tageszeit
appearance-moods-hint = Morgens munterer, abends langsamer — Gehen und Animation folgen der Uhr — und mit Schlafenszeit ein Gähnen in der halben Stunde davor.
appearance-sounds-mute = Töne stummschalten
appearance-sounds-hint = Antippen, Erinnerungen, der Fokus-Timer und Skripte. Stummgeschaltet ist nichts zu hören, und das Audiogerät bleibt geschlossen.
appearance-sounds-volume = Lautstärke
appearance-sounds-bedtime = Ruhe zur Schlafenszeit
appearance-sounds-bedtime-hint = Keine Töne, solange die Figuren zur Schlafenszeit schlafen.
mood-yawn = *gähn*
appearance-bedtime-from = von
appearance-bedtime-to = bis
fullscreen-hide = Figuren ausblenden
fullscreen-pause = Anhalten
fullscreen-ignore = Weiterlaufen lassen
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Anhalten, wenn ich weg bin
appearance-away-hint = Ohne Maus- oder Tastatureingabe für diese Zeit halten die Figuren an, landen und dösen ein; die nächste Eingabe weckt sie.
away-never = Nie
away-after-minutes = Nach { $minutes } Min.
appearance-away-unavailable = Diese Sitzung kann nicht erkennen, wann Sie weg sind: Dazu braucht es GNOME, einen echten X-Server oder einen Wayland-Compositor mit Leerlaufbenachrichtigungen.
appearance-battery-label = Im Akkubetrieb anhalten
appearance-battery-hint = Die Figuren halten still, solange der Computer im Akkubetrieb läuft.
appearance-welcome-label = Bei der Rückkehr begrüßen
appearance-welcome-hint = Nach einer Weile weg vom Computer begrüßt eine Figur bei der Rückkehr — „Guten Morgen!“ nach der Nacht. Nicht zur Schlafenszeit.
appearance-backup = Sichern…
appearance-backup-hint = Alles in einer Datei — Einstellungen, gespeicherte Szenen, ihre Bilder — zum Aufbewahren oder für einen anderen Computer.
appearance-restore = Wiederherstellen…
appearance-restore-hint = Eine Sicherung zurückholen: Ihre Szenen kommen zu den vorhandenen hinzu, ihre Einstellungen ersetzen diese (die alten bleiben als config.toml.before-restore erhalten).
welcome-back = Willkommen zurück!
welcome-morning = Guten Morgen!
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Bei der Anmeldung starten
appearance-autostart-hint = animaEngine starten, wenn Sie sich am Desktop anmelden.
appearance-autostart-reason = Um animaEngine bei der Anmeldung zu starten.
appearance-autostart-failed = Konnte nicht geändert werden: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Szenen
scene-scenes-hint = Speichern Sie, was zu sehen ist, unter einem Namen, und wechseln Sie hier, in der Befehlspalette oder mit „Nächste Szene“ im Tray dorthin zurück.
scene-scene-name-hint = Name der Szene
scene-scene-save-new = Als Szene speichern
scene-scene-load-tooltip = Zu dieser Szene wechseln – Rückgängig wechselt zurück
scene-scene-save-over = Aktuelle Ansicht als { $name } speichern
scene-scene-share = { $name } als eine Datei teilen, samt Bildern
scene-import = Importieren…
scene-import-tooltip = Eine geteilte Szene öffnen (eine .animascene-Datei). Die Datei auf das Overlay zu ziehen geht auch.
scene-scene-delete = Szene löschen
scene-scene-delete-confirm = Löschen?
scene-scene-delete-cancel = Behalten
scene-scene-save-failed = Nicht gespeichert: { $error }
toast-scene-loaded = Szene: { $name }
toast-scene-partly-read = Einige Figuren darin konnten nicht gelesen werden – von einer neueren Version oder mit Tippfehlern – und wurden ausgelassen.
toast-scene-failed = Szene nicht geladen: { $error }
toast-scene-shared = Szene zum Teilen bereit: { $path }
toast-scene-shared-scripts = Verhaltensskripte werden nicht geteilt; Figuren, die in der Datei stillstehen: { $count }
toast-scene-share-failed = Teilen fehlgeschlagen: { $error }
toast-gif-saved = GIF gespeichert: { $path }
toast-gif-failed = Das GIF konnte nicht gespeichert werden: { $error }
toast-character-shared = Figur zum Teilen bereit: { $path }
toast-character-shared-script = Das Verhaltensskript wird nicht geteilt: In der Datei steht die Figur still.
toast-character-added = { $name } hinzugefügt
toast-characters-added = Figuren hinzugefügt: { $count }
toast-backup-saved = Gesichert: { $path }
toast-backup-failed = Sichern oder Wiederherstellen fehlgeschlagen: { $error }
toast-restored = Wiederhergestellt: die Einstellungen, und Szenen hinzugefügt: { $count }
toast-scene-imported = Szene importiert: { $name }
toast-scene-import-failed = Import fehlgeschlagen: { $error }
toast-no-scenes = Noch keine gespeicherten Szenen – speichern Sie eine im Tab „Szene“.
palette-load-scene = Zu Szene wechseln: { $name }
palette-snooze-minutes = { $minutes } Minuten schlummern
palette-focus-start = Fokus-Timer starten
palette-focus-stop = Fokus-Timer stoppen
palette-snooze-morning = Bis morgen früh schlummern
tray-next-scene = Nächste Szene
tray-show-overlay = Overlay einblenden
tray-hide-overlay = Overlay ausblenden
tray-snoozing-until = Schlummert bis { $until }
tray-wake = Jetzt aufwachen
tray-quit = Beenden
palette-mute = Töne stummschalten
palette-unmute = Töne wieder einschalten
# Reminders (1.5): machine-translated, pending native review.
scene-reminders-header = Erinnerungen
scene-reminders-hint = Eine Figur sagt es ab und zu in einer Sprechblase. Zeit weg vom Computer zählt als Pause und fängt von vorn an.
scene-focus-header = Fokus-Timer
scene-focus-hint = Eine Weile arbeiten, dann sagt eine Figur, dass es Zeit für eine Pause ist — und wann es weitergeht. Erinnerungen warten so lange. Auch über die Befehlspalette oder mit „Fokus-Timer starten“ im Tray.
scene-focus-work = Arbeit
scene-focus-break = Pause
scene-focus-start = Starten
scene-focus-stop = Stoppen
scene-focus-working = Fokus bis { $until }
scene-focus-on-break = Pause bis { $until }
reminder-sound-label = Klang bei Erinnerungen und beim Fokus-Timer
reminder-sound-none = Keiner
reminder-sound-speaker = Der Antippklang der sprechenden Figur
reminder-notify-label = Auch als Desktop-Benachrichtigungen
reminder-notify-hint = Erinnerungen und der Fokus-Timer erscheinen auch als Desktop-Benachrichtigungen — sofort, wenn die Figuren nicht zu sehen sind, statt auf sie zu warten.
focus-said-started = Fokuszeit — bis { $until }.
focus-said-break = Pause! Weiter geht’s um { $until }.
focus-said-back = Die Pause ist vorbei — weiter geht’s bis { $until }.
scene-reminder-every = alle { $minutes } Min.
scene-reminder-anyone = eine beliebige Figur
scene-reminder-delete = Erinnerung löschen
scene-reminder-text-hint = Was gesagt wird
scene-reminder-minutes = Alle (Minuten)
scene-reminder-timed = Zu einer festen Zeit
scene-reminder-at = um
scene-reminder-on-date = an einem Datum
scene-reminder-days = Tage
scene-reminder-date = Datum
scene-reminder-who = Wer es sagt
scene-reminder-add = Erinnerung hinzufügen
reminder-break = Zeit für eine kurze Pause!
reminder-water = Trinken Sie ein Glas Wasser.
reminder-stretch = Stehen Sie auf und strecken Sie sich.
# Scenes by time of day (1.5): machine-translated, pending native review.
scene-schedule-header = Automatisch wechseln
scene-schedule-hint = Zu festen Zeiten zu einer gespeicherten Szene wechseln – etwa „Arbeit“ werktags um 09:00. Nicht während Sie bearbeiten: dann wartet es.
scene-schedule-needs-scene = Speichern Sie oben eine Szene, um nach Zeitplan zu ihr zu wechseln.
scene-schedule-row = { $time } · { $days } → { $scene }
scene-schedule-every-day = jeden Tag
scene-schedule-weekdays = werktags
scene-schedule-weekends = am Wochenende
scene-schedule-at = Um
scene-schedule-minute = Minute
scene-schedule-days = Tage
scene-schedule-scene = Szene
scene-schedule-add = Hinzufügen
scene-schedule-delete = Regel löschen

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Globale Hotkeys konnten nicht registriert werden (typisch für native Wayland-Sitzungen). Tray-Menü und ⚙-Knopf funktionieren weiter.
warning-hot-reload-disconnected = Der Hot-Reload-Worker wurde unerwartet beendet; laufende Konfigurationsänderungen greifen erst nach einem Neustart.
# Machine-translated, pending native review.
warning-config-unreadable = Ihre Konfigurationsdatei konnte nicht gelesen werden, daher wurde die Standardszene geladen. Das Original liegt daneben als config.toml.bak-corrupt.
warning-config-partly-unread = Ein Teil Ihrer Konfigurationsdatei konnte nicht gelesen werden – er stammt vielleicht von einer neueren Version von animaEngine oder enthält einen Tippfehler. Die App arbeitet ohne ihn weiter: Figuren darin sind ausgeblendet, Einstellungen darin stehen auf ihrem Standardwert, aber alles bleibt in der Datei erhalten.
action-toggle-perf-overlay = Performance-Overlay umschalten

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Neu in 1.5
whats-new-alive = Die Figuren werden lebendig: Tippen Sie sie an, und lassen Sie sie einander grüßen, Fangen spielen oder einander folgen.
whats-new-your-day = Erinnerungen zu einer festen Zeit, ein Fokus-Timer, Schlafenszeit und Schlummern – und ein Gruß, wenn Sie zurückkommen.
whats-new-editing = Rückgängig mit Strg+Z, mehrere Figuren auf einmal auswählen, gruppieren und ausrichten.
whats-new-files = Szenen bekommen Namen, Bilder und einen Zeitplan; teilen Sie eine Szene oder eine einzelne Figur, oder sichern Sie alles in einer Datei.
whats-new-sound = Klänge in der gewünschten Lautstärke, mit einer eigenen für jede Figur, Ruhe zur Schlafenszeit und Stummschalten über das Tray oder ein Tastenkürzel.
whats-new-keyboard = Alles auch mit der Tastatur: Shift+F10 öffnet das Menü einer Figur, Strg+A wählt alle aus, N geht sie der Reihe nach durch.
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
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } ausgewählt. Dieses Panel bearbeitet { $name }; Ziehen, Tastenkürzel und das Rechtsklickmenü wirken auf alle.

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
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Figuren dupliziert: { $count }
toast-duplicate-failed = Duplizieren fehlgeschlagen: { $error }
toast-deleted = { $name } gelöscht
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Figuren gelöscht: { $count }
toast-copied = { $name } kopiert
toast-copied-many = Figuren kopiert: { $count }
toast-cut = { $name } ausgeschnitten
toast-cut-many = Figuren ausgeschnitten: { $count }
toast-pasted = { $name } eingefügt
toast-pasted-many = Figuren eingefügt: { $count }
toast-paste-failed = Einfügen fehlgeschlagen: { $error }
toast-nothing-to-paste = Nichts zum Einfügen: zuerst eine Figur oder ein Bild kopieren
pasted-image-name = Eingefügtes Bild
toast-playback-resumed = Wiedergabe fortgesetzt
toast-playback-paused = Wiedergabe pausiert
# Undo (1.5): machine-translated, pending native review.
toast-undone = Rückgängig gemacht
toast-redone = Wiederhergestellt
toast-nothing-to-undo = Nichts rückgängig zu machen
toast-nothing-to-redo = Nichts zu wiederholen
inspector-wander-box = Streifbereich
toast-perf-snapshot = Performance-Snapshot: { $path }
toast-perf-snapshot-failed = Snapshot fehlgeschlagen: { $error }

# Why a file was not taken, presets, the tray's tooltip, demo names (1.5)
error-file-unreadable = Die Datei konnte nicht gelesen werden: { $error }
error-not-a-file = Das ist keine Datei.
error-file-too-big = Die Datei ist { $size } MB groß; höchstens { $max } MB lassen sich hinzufügen.
error-file-type = Dateien mit der Endung .{ $ext } lassen sich nicht anzeigen; PNG, JPEG, GIF, WebP und MP4 schon.
error-file-no-extension = Der Dateiname hat keine Endung, die sagt, was die Datei ist; angezeigt werden PNG, JPEG, GIF, WebP und MP4.
error-asset-not-found = Nicht gefunden: { $path }
error-not-a-folder = Kein Ordner: { $path }
error-no-frames = Keine Bilder in { $path }
error-image-too-large = Das Bild ist { $width }×{ $height } groß; höchstens { $max }×{ $max } lassen sich anzeigen.
error-share-too-big = Die Szene ist größer als { $max } MB – zu groß, um sie als eine Datei zu teilen.
error-backup-too-big = Alles zusammen ist größer als { $max } MB – zu viel für eine Sicherung.
error-picture-not-shareable = { $path } ist kein Bild, das sich teilen lässt.
error-file-over = Die Datei ist größer als { $max } MB.
error-not-a-scene-file = Das ist weder eine Szene noch eine Figur aus animaEngine.
error-not-a-backup = Das ist keine Sicherung von animaEngine.
error-file-damaged = Die Datei ist beschädigt oder stammt nicht von animaEngine.
error-file-newer = Sie stammt von einer neueren Version von animaEngine; aktualisieren Sie, um sie zu öffnen.
shimeji-no-actions = Das Paket hat keine actions.xml in seinem Ordner conf.
shimeji-no-idle = Das Paket hat keine Ruhepose (eine Stay-Aktion).
shimeji-not-utf8 = Das XML des Pakets ist nicht UTF-8; Pakete in Shift-JIS lassen sich nicht lesen.
shimeji-too-big = Das Paket ist zu groß: höchstens { $sprites } Bilder und { $max } MiB.
tray-tooltip = Rechtsklick für Optionen
preset-cozy-companion = Gemütliche Begleitung
preset-cozy-companion-about = Ein sanfter Geist, der rechts still umherwandert.
preset-productivity-zen = Produktiv-Zen
preset-productivity-zen-about = Ein Schleim und ein Herz, still, in einer ruhigen Ecke.
preset-halloween-party = Halloween-Party
preset-halloween-party-about = Drei Geister marschieren am unteren Rand entlang.
preset-birthday-confetti = Geburtstagskonfetti
preset-birthday-confetti-about = Herzen und Sterne regnen herab. Schwerkraft an.
preset-studio-session = Studio-Session
preset-studio-session-about = Alle fünf Demo-Figuren über den Desktop verteilt.
preset-cursor-follower = Mauszeiger-Begleiter
preset-cursor-follower-about = Eine Katze folgt Ihrer Maus überallhin.
preset-companion = Begleiter
preset-zen-slime = Zen-Schleim
preset-zen-heart = Zen-Herz
preset-ghost-n = Geist { $n }
demo-ghost = Geist
demo-slime = Schleim
demo-heart = Herz
demo-star = Stern
demo-cat = Katze
demo-name = { $name } (Demo)
scene-shared-unnamed = Geteilte Szene
