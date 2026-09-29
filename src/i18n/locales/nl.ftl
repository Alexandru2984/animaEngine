# Nederlands — basisvertaling. Nakijken door native speaker openstaand.

app-name = animaEngine

settings-tab-inspector = Inspector
settings-tab-scene = Scène
settings-tab-appearance = Weergave
entity-count-zero = Geen entiteiten
entity-count-singular = { $n } entiteit
entity-count-plural = { $n } entiteiten

inspector-section-position = Positie
inspector-section-appearance = Weergave
inspector-section-animation = Animatie
animation-easing-label = Easing
easing-linear = Lineair
easing-ease-in-quad = Inlopen
easing-ease-out-quad = Uitlopen
easing-ease-in-out-quad = In-/uitlopen
easing-sine = Sinus
easing-bounce-out = Stuiteren
inspector-section-behavior = Gedrag
inspector-visible = Zichtbaar
inspector-gravity = Zwaartekracht
inspector-scale = Schaal
inspector-behavior-speed = Snelheid
inspector-behavior-comfort = Comfortafstand
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Periode
inspector-double-click-reset-hint = Dubbelklik om de standaardwaarde te herstellen.
inspector-opacity = Dekking
inspector-fps = FPS
inspector-playing = Bezig
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Niets geselecteerd
inspector-nothing-selected-hint = Klik op een entiteit in het tabblad Scène, of druk op Tab om door te lopen.

behavior-idle = Inactief
behavior-walk = Rondlopen
behavior-follow = Cursor volgen
behavior-wander = Begrensd zwerven
behavior-bounce = Stuiteren
behavior-bounce-axis = As
behavior-bounce-horizontal = Horizontaal
behavior-bounce-vertical = Verticaal
behavior-bounce-both = Beide (cirkel)
behavior-script = Script
behavior-script-path-label = Pad
behavior-script-path-hint = Relatief aan uw assetbibliotheek — { $path }
behavior-script-params = Parameters
behavior-script-param-name = naam
behavior-script-add-param = Toevoegen
behavior-script-remove-param = Deze parameter verwijderen
script-failed-toast = Gedragsscript { $script } is mislukt: { $error }

scene-empty-headline = Lege scène
scene-empty-hint = Sleep een PNG / GIF / WebP / MP4 naar de overlay — of probeer hieronder een preset.
scene-drop-hint = Sleep een PNG / GIF / WebP naar de overlay om een entiteit toe te voegen.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Bestand toevoegen…
scene-add-file-tooltip = Kies afbeeldingen of video's om als figuren toe te voegen.
file-chooser-title = Figuren toevoegen
file-chooser-filter = Afbeeldingen en video's
file-chooser-unavailable-toast = De bestandskiezer kon niet worden geopend. Daarvoor is xdg-desktop-portal nodig.
scene-presets-header = Presets
scene-groups-header = Groepen
scene-preset-append = Toevoegen
scene-preset-replace = Vervangen
scene-preset-replace-tooltip = Wist de huidige scène vóór het toevoegen

monitor-section-header = Monitoren
monitor-mode-label = Verdeling
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Uitstrekken over alle monitors
monitor-mode-span-unsupported = Op deze backend kan één overlay niet meerdere monitoren bedekken — gebruik in plaats daarvan per monitor.
monitor-mode-single = Enkele monitor
scene-window-awareness = Op vensters landen (X11)
scene-window-awareness-tooltip = Personages met actieve fysica landen op en lopen langs de bovenrand van uw open vensters. Alleen X11-sessies — Wayland geeft geen vensterposities, dus daar doet dit niets.
scene-window-awareness-unavailable = Niet beschikbaar op native Wayland — geen protocol geeft vensterposities prijs.
monitor-pin-label = Vastpinnen aan monitor
monitor-pin-auto = Auto (volgt positie)
monitor-pinned-toast = Entiteit vastgepind aan { $name }
monitor-pin-cleared-toast = Entiteit volgt nu zijn positie
monitor-no-monitors-detected = Geen monitors gedetecteerd

appearance-theme-header = Thema
appearance-theme-label = Thema
appearance-language-header = Taal
appearance-language-no-font = Er is geen lettertype voor dit schrift geïnstalleerd — installeer eerst een Noto CJK-pakket.
theme-dark = Donker
theme-light = Licht
theme-dark-hc = Donker · Hoog contrast
theme-light-hc = Licht · Hoog contrast

onboarding-tabs = Instellingen staan in vijf tabbladen — Inspector, Scène, Bibliotheek, Weergave, Sneltoetsen.
onboarding-quick-toggles = Tip: V wisselt zichtbaarheid, G wisselt zwaartekracht — zonder dit paneel te openen.
onboarding-theme = Thema's worden direct toegepast — geen herstart nodig.
onboarding-coach-step1 = Welkom! Uw personages leven op het bureaublad. Klik op het tandwiel rechtsboven om de bewerkmodus te openen.
onboarding-coach-step2 = Sleep een PNG, GIF, WebP of MP4 ergens op het scherm om het als personage toe te voegen. Het zijpaneel bewerkt alles wat u selecteert.
onboarding-coach-step3 = Ctrl+K opent het opdrachtenpalet. Ctrl+Shift+A schakelt de bewerkmodus overal om, Ctrl+Shift+H verbergt de overlay.
onboarding-coach-next = Volgende
onboarding-coach-skip = Rondleiding overslaan
onboarding-coach-done = Begrepen
palette-replace-row = Scène vervangen door: { $preset }
palette-append-row = Preset toevoegen: { $preset }
palette-footer-hint = Esc sluit · Ctrl+K schakelt · ↑↓ + Enter kiest
onboarding-dismiss = Sluiten

menu-duplicate = Dupliceren
menu-reset-transform = Transformatie resetten
menu-toggle-gravity = Zwaartekracht wisselen
menu-bring-forward = Naar voren brengen
menu-send-backward = Naar achteren plaatsen
menu-delete = Verwijderen

toggle-enter-edit = Bewerkingsmodus openen
toggle-exit-edit = Bewerkingsmodus verlaten

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Typ om acties, thema's en presets te zoeken…
palette-switch-theme = Wisselen naar thema { $theme }

settings-tab-library = Bibliotheek

# Asset library tab
library-empty-headline = Geen assets geïndexeerd
library-empty-hint = Sleep bestanden naar { $path } of stel ANIMA_ASSETS_DIR in.
library-no-asset-root = Geen asset-map gevonden. Maak er een aan in { $path }
library-search-placeholder = Assets zoeken…
library-add-to-scene = Toevoegen aan scène
library-kind-image = Afbeelding
library-kind-animated = Animatie
library-kind-video = Video
library-asset-added-toast = { $name } toegevoegd aan de scène
library-asset-add-failed-toast = Kon { $name } niet toevoegen
library-count = { $n } assets geïndexeerd

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Sneltoetsen
keybindings-unbound = (niet toegewezen)
keybindings-add = Toevoegen
keybindings-recording = Druk een toetsencombinatie… (Esc annuleert)
keybindings-conflict = Conflicteert met { $action }
keybindings-reset-all = Alles naar standaard herstellen
keybindings-reset-one = Standaardwaarde herstellen
keybindings-remove-chord = Deze toewijzing verwijderen
keybindings-help = Aangepaste sneltoetsen worden bewaard in config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Bewerkmodus omschakelen
action-hide-overlay = Overlay verbergen / tonen
action-pause-all = Alle animaties pauzeren
action-quit-with-save = Afsluiten (configuratie opslaan)
action-save-now = Configuratie nu opslaan
action-open-command-palette = Opdrachtenpalet
# Undo (1.5): machine-translated, pending native review.
action-undo = Ongedaan maken
action-redo = Opnieuw

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Selectie groeperen
action-ungroup-selected = Groepering van selectie opheffen
menu-group = Groeperen
menu-ungroup = Groepering opheffen
group-default-name = Groep { $number }
group-copy-name = { $name } (kopie)
toast-grouped = Gegroepeerd als { $name }
toast-ungrouped = Groepen opgeheven: { $count }
toast-nothing-to-ungroup = Niets van de selectie zit in een groep
scene-groups-empty-hint = Selecteer meerdere figuren en druk op Ctrl+G — of klik er met de rechtermuisknop op en kies Groeperen.
scene-group-members = Figuren: { $count }
scene-group-select-tooltip = De figuren in deze groep selecteren
scene-group-rename = Groep hernoemen
scene-group-show = Groep tonen
scene-group-hide = Groep verbergen
scene-group-ungroup = Groepering opheffen — de figuren blijven

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Linkerranden uitlijnen
arrange-align-center = Centreren op één verticale lijn
arrange-align-right = Rechterranden uitlijnen
arrange-align-top = Bovenranden uitlijnen
arrange-align-middle = Centreren op één horizontale lijn
arrange-align-bottom = Onderranden uitlijnen
arrange-distribute-horizontally = Gelijkmatig naast elkaar verdelen
arrange-distribute-vertically = Gelijkmatig onder elkaar verdelen
scene-snap = Magnetisch slepen
scene-snap-tooltip = Randen en middens klikken vast aan die van de schermen en van andere figuren. Houd Alt ingedrukt tijdens het slepen om vrij te plaatsen.

action-cycle-entity = Naar het volgende personage
action-delete-selected = Geselecteerd personage verwijderen
action-nudge-up = Selectie omhoog duwen
action-nudge-down = Selectie omlaag duwen
action-nudge-left = Selectie naar links duwen
action-nudge-right = Selectie naar rechts duwen
action-center-on-screen = Selectie centreren op het scherm
action-toggle-visible = Zichtbaarheid omschakelen
action-toggle-gravity = Zwaartekracht omschakelen
action-toggle-playback = Afspelen/pauzeren
action-duplicate-selected = Selectie dupliceren
action-reset-transform = Schaal / dekking herstellen
action-bring-forward = Selectie naar voren halen
action-send-backward = Selectie naar achteren sturen
action-fps-up = FPS verhogen
action-fps-down = FPS verlagen
action-opacity-up = Dekking verhogen
action-opacity-down = Dekking verlagen
action-cycle-monitor = Monitorkoppeling doorschakelen
action-show-entity-info = Personage-info tonen
action-show-help = Toetsenbordhulp tonen

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Toegankelijkheid
appearance-accesskit-label = AccessKit-boomupdates genereren
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Laat schermlezers (Orca en andere, via AT-SPI) de panelen lezen en bedienen. Uit ziet een schermlezer een leeg venster. Let op: zolang er een draait, staat tekst die u in panelen typt op de AT-SPI-bus, waar elk proces van uw gebruiker hem kan lezen.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Schermlezers worden op dit systeem nog niet ondersteund.
appearance-reduced-motion-label = Beweging verminderen
appearance-reduced-motion-hint = Slaat UI-overgangen over (paneel schuiven, fades, palet-pop) en stopt decoratief wiebelen. Animaties die een toestand tonen blijven actief.
appearance-hover-startle-label = Schrikken bij zweven
appearance-hover-startle-hint = Mascottes deinzen terug voor de muisaanwijzer als die dichtbij komt. Cursorvolging werkt alleen op X11, dus op native Wayland reageert dit alleen in de bewerkmodus.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Als een app schermvullend is
appearance-fullscreen-hint = Games, video’s en presentaties die het scherm vullen. Werkt onder X11 en op Wayland-compositors zoals sway en Hyprland; onder GNOME en KDE met Wayland alleen voor apps die via XWayland draaien.
fullscreen-hide = Figuren verbergen
fullscreen-pause = Pauzeren
fullscreen-ignore = Door laten gaan
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pauzeren als ik weg ben
appearance-away-hint = Zonder muis- of toetsenbordinvoer gedurende deze tijd staan de figuren stil; bij uw volgende invoer gaan ze verder.
away-never = Nooit
away-after-minutes = Na { $minutes } min
appearance-away-unavailable = Deze sessie kan niet zien wanneer u weg bent: daarvoor is GNOME nodig, een echte X-server of een Wayland-compositor met inactiviteitsmeldingen.
appearance-battery-label = Pauzeren op batterij
appearance-battery-hint = De figuren staan stil zolang de computer op de batterij draait.
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Starten bij aanmelden
appearance-autostart-hint = animaEngine starten wanneer u zich aanmeldt bij het bureaublad.
appearance-autostart-reason = Om animaEngine te starten bij het aanmelden.
appearance-autostart-failed = Kon het niet wijzigen: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Scènes
scene-scenes-hint = Sla op wat op het scherm staat onder een naam, en ga er hier naar terug, via het opdrachtenpalet of met ‘Next scene’ in het systeemvak.
scene-scene-name-hint = Naam van de scène
scene-scene-save-new = Opslaan als scène
scene-scene-load-tooltip = Naar deze scène wisselen — Ongedaan maken wisselt terug
scene-scene-save-over = Wat op het scherm staat opslaan als { $name }
scene-scene-delete = Scène verwijderen
scene-scene-delete-confirm = Verwijderen?
scene-scene-delete-cancel = Behouden
scene-scene-save-failed = Niet opgeslagen: { $error }
toast-scene-loaded = Scène: { $name }
toast-scene-failed = Scène niet geladen: { $error }
toast-no-scenes = Nog geen opgeslagen scènes — sla er een op in het tabblad Scène.
palette-load-scene = Naar scène wisselen: { $name }

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Globale sneltoetsen konden niet worden geregistreerd (gebruikelijk in een native Wayland-sessie). Het traymenu en de ⚙-knop blijven werken.
warning-hot-reload-disconnected = De hot-reload-worker is onverwacht gestopt; lopende configuratiewijzigingen gelden pas na een herstart.
# Machine-translated, pending native review.
warning-config-unreadable = Uw configuratiebestand kon niet worden gelezen, dus de standaardscène is geladen. Het origineel staat ernaast als config.toml.bak-corrupt.
action-toggle-perf-overlay = Prestatie-overlay omschakelen

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Nieuw in 1.4
whats-new-add-file = ‘Bestand toevoegen…’ op het tabblad Scène voegt figuren toe via de bestandskiezer van uw bureaublad — zonder slepen.
whats-new-palette = Ctrl+K voert nu elke actie uit, met de sneltoets ernaast.
whats-new-screen-readers = Schermlezers zoals Orca kunnen het instellingenpaneel nu voorlezen en bedienen.
onboarding-keybindings = Klik op een sneltoets om hem te verwijderen; druk een combinatie om een nieuwe op te nemen.
onboarding-perf-overlay = Druk Ctrl+Shift+` om de live prestatie-overlay te openen.
appearance-reset-onboarding = Introductietips herstellen

scene-empty-action-browse-presets = Presets verkennen
library-empty-action-copy-path = Pad naar klembord kopiëren

appearance-reset-onboarding-hint = Haalt de weggeklikte tips en het ‘Wat is nieuw’-paneel terug.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Toestemming voor sneltoetsen geweigerd — er worden X11-sneltoetsen gebruikt. Probeer opnieuw via het tabblad Sneltoetsen.
portal-denied-native-toast = Toestemming voor sneltoetsen geweigerd — het traymenu en compositor-bindingen blijven werken.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Globale sneltoetsen via:
keybindings-backend-tooltip = Welk mechanisme de drie globale sneltoetsen (bewerken, verbergen, pauzeren) levert terwijl andere apps de focus hebben. Bepaald bij het opstarten; in-app-sneltoetsen blijven ongemoeid.
keybindings-portal-restart-hint = Triggerwijzigingen gelden vanaf de volgende start (de desktop onthoudt uw goedkeuring).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } losgekoppeld — { $n } vastgezette personages volgen nu hun positie.
monitor-plugged-toast = Monitor { $name } aangesloten.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Shimeji-pakket importeren
library-import-shimeji-hint = Sleep de pakketmap op de overlay of plak het pad hier. Sprites worden naar de bibliotheek gekopieerd.
library-import-shimeji-button = Importeren
shimeji-imported-toast = { $name } geïmporteerd ({ $n } onderdelen overgeslagen — zie log)
shimeji-import-failed-toast = Import mislukt: { $reason }
shimeji-no-library-toast = Geen bibliotheekmap — maak eerst { $path } aan.
crash-report-found-toast = De vorige sessie is gecrasht. Een rapport is opgeslagen in { $path } — voeg het toe aan een GitHub-issue.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Samengesteld door groep { $group }: { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } geselecteerd. Dit paneel bewerkt { $name }; slepen, sneltoetsen en het rechtermuismenu werken op allemaal.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuratie opgeslagen
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Configuratie opnieuw geladen van schijf
toast-config-reload-failed = Configuratie niet opnieuw geladen — het bestand is ongeldig of wordt nog geschreven. De huidige scène blijft.
toast-config-reload-discarded = Configuratie niet opnieuw geladen — u hebt de scène bewerkt terwijl die werd geladen.
toast-save-failed = Opslaan mislukt: { $error }
toast-rejected = Geweigerd: { $reason }
toast-added = { $name } toegevoegd
toast-load-failed = Laden mislukt: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Thema: { $theme }
toast-preset-entry-failed = Preset-item kon niet worden toegevoegd: { $error }
toast-preset-loaded = Preset geladen: { $name }
toast-duplicated = { $name } gedupliceerd
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Figuren gedupliceerd: { $count }
toast-duplicate-failed = Dupliceren mislukt: { $error }
toast-deleted = { $name } verwijderd
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Figuren verwijderd: { $count }
toast-playback-resumed = Afspelen hervat
toast-playback-paused = Afspelen gepauzeerd
# Undo (1.5): machine-translated, pending native review.
toast-undone = Ongedaan gemaakt
toast-redone = Opnieuw uitgevoerd
toast-nothing-to-undo = Niets om ongedaan te maken
toast-nothing-to-redo = Niets om opnieuw uit te voeren
inspector-wander-box = Zwerfgebied
toast-perf-snapshot = Prestatie-snapshot: { $path }
toast-perf-snapshot-failed = Snapshot mislukt: { $error }
