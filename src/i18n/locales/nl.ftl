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

scene-empty-headline = Lege scène
scene-empty-hint = Sleep een PNG / GIF / WebP / MP4 naar de overlay — of probeer hieronder een preset.
scene-drop-hint = Sleep een PNG / GIF / WebP naar de overlay om een entiteit toe te voegen.
scene-presets-header = Presets
scene-preset-append = Toevoegen
scene-preset-replace = Vervangen
scene-preset-replace-tooltip = Wist de huidige scène vóór het toevoegen

monitor-section-header = Monitoren
monitor-mode-label = Verdeling
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Uitstrekken over alle monitors
monitor-mode-single = Enkele monitor
scene-window-awareness = Op vensters landen (X11)
scene-window-awareness-tooltip = Personages met actieve fysica landen op en lopen langs de bovenrand van uw open vensters. Alleen X11-sessies — Wayland geeft geen vensterposities, dus daar doet dit niets.
monitor-pin-label = Vastpinnen aan monitor
monitor-pin-auto = Auto (volgt positie)
monitor-pinned-toast = Entiteit vastgepind aan { $name }
monitor-pin-cleared-toast = Entiteit volgt nu zijn positie
monitor-no-monitors-detected = Geen monitors gedetecteerd

appearance-theme-header = Thema
appearance-theme-label = Thema
appearance-language-header = Taal
theme-dark = Donker
theme-light = Licht
theme-dark-hc = Donker · Hoog contrast
theme-light-hc = Licht · Hoog contrast

onboarding-tabs = Instellingen verspreid over drie tabbladen — Inspector, Scène, Weergave.
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

palette-search-placeholder = Typ om thema's / presets te zoeken…
palette-close-hint = Esc om te sluiten · Ctrl+K om te wisselen
palette-switch-theme = Wisselen naar thema { $theme }
palette-apply-preset = Preset toepassen: { $preset }

settings-tab-library = Bibliotheek

# Asset library tab
library-empty-headline = Geen assets geïndexeerd
library-empty-hint = Sleep bestanden naar ~/.local/share/animaEngine/assets/ of stel ANIMA_ASSETS_DIR in.
library-no-asset-root = Geen asset-map gevonden. Maak er een aan in ~/.local/share/animaEngine/assets/
library-search-placeholder = Assets zoeken…
library-add-to-scene = Toevoegen aan scène
library-sort-recent = Recent
library-sort-name = Naam
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
keybindings-help = Aangepaste sneltoetsen worden bewaard in config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Bewerkmodus omschakelen
action-hide-overlay = Overlay verbergen / tonen
action-pause-all = Alle animaties pauzeren
action-quit-with-save = Afsluiten (configuratie opslaan)
action-save-now = Configuratie nu opslaan
action-open-command-palette = Opdrachtenpalet
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
appearance-accesskit-hint = Voedt AT-SPI-schermlezers (Orca enz.). Laat dit aan, tenzij u minder resources wilt gebruiken of uw desktop geen AT-SPI-bus heeft. Let op: tekst die u in panelen typt verschijnt ook op de AT-SPI-bus, waar elk proces van uw gebruiker hem kan lezen.
appearance-reduced-motion-label = Beweging verminderen
appearance-reduced-motion-hint = Slaat UI-overgangen over (paneel schuiven, fades, palet-pop) en stopt decoratief wiebelen. Animaties die een toestand tonen blijven actief.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Globale sneltoetsen konden niet worden geregistreerd (gebruikelijk in een native Wayland-sessie). Het traymenu en de ⚙-knop blijven werken.
warning-hot-reload-disconnected = De hot-reload-worker is onverwacht gestopt; lopende configuratiewijzigingen gelden pas na een herstart.
action-toggle-perf-overlay = Prestatie-overlay omschakelen

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Nieuw in 0.4
whats-new-keybindings = Herinstelbare sneltoetsen — open het nieuwe tabblad Sneltoetsen.
whats-new-collapse-state = Inspector-secties onthouden hun open/dicht-stand tussen sessies.
whats-new-error-banners = Foutmeldingen (voorheen stil) tonen nu toasts of banners — u ziet ze.
whats-new-accessibility-toggle = AccessKit kan worden uitgeschakeld via Uiterlijk → Toegankelijkheid.
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
shimeji-no-library-toast = Geen bibliotheekmap — maak eerst ~/.local/share/animaEngine/assets/ aan.
crash-report-found-toast = De vorige sessie is gecrasht. Een rapport is opgeslagen in { $path } — voeg het toe aan een GitHub-issue.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Samengesteld door groep { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configuratie opgeslagen
toast-save-failed = Opslaan mislukt: { $error }
toast-rejected = Geweigerd: { $reason }
toast-added = { $name } toegevoegd
toast-load-failed = Laden mislukt: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Thema: { $theme }
toast-preset-entry-failed = Preset-item kon niet worden toegevoegd: { $error }
toast-preset-loaded = Preset geladen: { $name }
toast-duplicated = { $name } gedupliceerd
toast-duplicate-failed = Dupliceren mislukt: { $error }
toast-deleted = { $name } verwijderd
toast-playback-resumed = Afspelen hervat
toast-playback-paused = Afspelen gepauzeerd
inspector-wander-box = Zwerfgebied
toast-perf-snapshot = Prestatie-snapshot: { $path }
toast-perf-snapshot-failed = Snapshot mislukt: { $error }
