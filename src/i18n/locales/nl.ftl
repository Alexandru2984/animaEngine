# Nederlands — basisvertaling. Nakijken door native speaker openstaand.

app-name = animaEngine

settings-tab-inspector = Inspector
settings-tab-scene = Scène
settings-tab-appearance = Weergave
entity-count-zero = Geen figuren
entity-count-singular = { $n } figuur
entity-count-plural = { $n } figuren

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
inspector-section-talk = Zegt
inspector-talk-hint = Eén zin per regel. Een tik op de figuur in de bewerkmodus laat hem er een zeggen, en hij kan ook uit zichzelf iets zeggen.
inspector-talk-placeholder = Hallo!
inspector-talk-pace = Spreekt
inspector-talk-only-poked = Alleen bij een tik
inspector-talk-every = Ongeveer elke { $minutes } min
inspector-talk-say-now = Nu een zeggen
placeholders-hint = In een zin worden { $time }, { $day }, { $date } en { $name } ingevuld zodra hij wordt gezegd.
placeholder-date = { $day } { $month }
weekday-0 = zondag
weekday-1 = maandag
weekday-2 = dinsdag
weekday-3 = woensdag
weekday-4 = donderdag
weekday-5 = vrijdag
weekday-6 = zaterdag
month-1 = januari
month-2 = februari
month-3 = maart
month-4 = april
month-5 = mei
month-6 = juni
month-7 = juli
month-8 = augustus
month-9 = september
month-10 = oktober
month-11 = november
month-12 = december
inspector-sound-label = Geluid bij een tik
inspector-sound-none = Geen
inspector-sound-play = Afspelen
inspector-sound-hint = Geluidsbestanden (.ogg, .wav, .flac, .mp3) in { $path } verschijnen hier.
inspector-bubble-colour = Kleur van de tekstballon
inspector-bubble-size = Tekstgrootte
inspector-bubble-reset = Standaard
inspector-hours-label = Alleen op bepaalde uren
inspector-hours-hint = Buiten deze uren staat de figuur niet op het scherm — het spook ’s nachts, de zon overdag. In de bewerkmodus zijn ze er allemaal, om te bewerken.
inspector-hours-from = van
inspector-hours-to = tot
inspector-visible = Zichtbaar
inspector-gravity = Zwaartekracht
inspector-scale = Schaal
inspector-behavior-speed = Snelheid
inspector-follow-target = Volgt
inspector-follow-distance = Afstand
inspector-follow-nobody = (niemand)
inspector-follow-none-free = Niemand om te volgen: de anderen zijn er niet of volgen deze al.
inspector-tag-partner = Speelt met
inspector-tag-hint = Ze zitten elkaar een tijdje achterna — wie hem is, rent de ander na — en rusten dan anderhalve minuut. De ander hoeft niets in te stellen.
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
inspector-nothing-selected-hint = Klik op een figuur in het tabblad Scène, of druk op N om ze te doorlopen.

behavior-idle = Inactief
behavior-walk = Rondlopen
behavior-follow = Cursor volgen
behavior-follow-character = Een figuur volgen
behavior-play-tag = Tikkertje spelen
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
sound-failed-toast = Het geluid { $sound } kon niet worden afgespeeld: { $error }

scene-empty-headline = Lege scène
scene-empty-hint = Sleep een PNG / GIF / WebP / MP4 naar de overlay — of probeer hieronder een preset.
scene-drop-hint = Sleep een PNG / GIF / WebP naar de overlay om een figuur toe te voegen.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Bestand toevoegen…
scene-add-file-tooltip = Kies afbeeldingen of video's om als figuren toe te voegen.
scene-paste = Plakken
scene-paste-tooltip = De gekopieerde figuren aan deze scène toevoegen. Gekopieerd: { $count }
scene-paste-empty = Plak een elders gekopieerde afbeelding of bestanden — of figuren: selecteer ze en druk op Ctrl+C, of klik er met de rechtermuisknop op en kies Kopiëren, in deze scène of een andere.
file-chooser-title = Figuren toevoegen
file-chooser-filter = Afbeeldingen en video's
share-chooser-title = Een scène delen
gif-chooser-title = Een GIF opslaan
character-chooser-title = Een figuur delen
backup-chooser-title = Back-up van animaEngine
restore-chooser-title = Een back-up herstellen
restore-chooser-filter = animaEngine-back-ups
import-chooser-title = Een scène importeren
import-chooser-filter = animaEngine-scènes en -figuren
file-chooser-unavailable-toast = De bestandskiezer kon niet worden geopend. Daarvoor is xdg-desktop-portal nodig.
scene-presets-header = Presets
scene-groups-header = Groepen
scene-characters-header = Figuren
scene-together-header = Samen
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
scene-window-awareness-tooltip = Figuren met actieve fysica landen op en lopen langs de bovenrand van uw open vensters. Alleen X11-sessies — Wayland geeft geen vensterposities, dus daar doet dit niets.
scene-window-awareness-unavailable = Niet beschikbaar op native Wayland — geen protocol geeft vensterposities prijs.
monitor-pin-label = Vastpinnen aan monitor
monitor-pin-auto = Auto (volgt positie)
monitor-pinned-toast = Figuur vastgepind aan { $name }
monitor-pin-cleared-toast = Figuur volgt nu zijn positie
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
onboarding-coach-step1 = Welkom! Uw figuren leven op het bureaublad. Klik op het tandwiel rechtsboven om de bewerkmodus te openen.
onboarding-coach-step2 = Sleep een PNG, GIF, WebP of MP4 ergens op het scherm om het als figuur toe te voegen. Het zijpaneel bewerkt alles wat u selecteert.
onboarding-coach-step3 = Ctrl+K opent het opdrachtenpalet. Ctrl+Shift+A schakelt de bewerkmodus overal om, Ctrl+Shift+H verbergt de overlay.
onboarding-coach-next = Volgende
onboarding-coach-skip = Rondleiding overslaan
onboarding-coach-done = Begrepen
palette-replace-row = Scène vervangen door: { $preset }
palette-append-row = Preset toevoegen: { $preset }
palette-footer-hint = Esc sluit · Ctrl+K schakelt · ↑↓ + Enter kiest
onboarding-dismiss = Sluiten

menu-duplicate = Dupliceren
menu-copy = Kopiëren
menu-save-gif = Opslaan als GIF…
menu-share-character = Figuur delen…
menu-cut = Knippen
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
library-count = { $n ->
        [one] { $n } asset geïndexeerd
       *[other] { $n } assets geïndexeerd
    }

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
action-toggle-focus = Focustimer starten of stoppen
action-toggle-snooze = 15 minuten sluimeren of wakker worden
action-toggle-mute = Geluiden dempen of weer aanzetten
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
scene-bump = Figuren botsen tegen elkaar
scene-bump-tooltip = Met zwaartekracht aan (G) landen figuren op elkaars hoofd; lopende figuren keren om als ze elkaar tegenkomen.
scene-greet = Figuren groeten elkaar als ze elkaar tegenkomen
scene-greet-tooltip = Twee die bij elkaar komen — lopend, volgend, of de een die op de ander landt — staan even stil en groeten elkaar, met een hartje. Niet te vaak.
greet-hi = Hoi!
greet-hello = Hallo!
greet-hey = Hé daar!

action-cycle-entity = Naar de volgende figuur
action-select-all = Alle figuren selecteren
action-open-context-menu = Menu van de selectie openen
action-delete-selected = Geselecteerde figuur verwijderen
action-nudge-up = Selectie omhoog duwen
action-nudge-down = Selectie omlaag duwen
action-nudge-left = Selectie naar links duwen
action-nudge-right = Selectie naar rechts duwen
action-center-on-screen = Selectie centreren op het scherm
action-toggle-visible = Zichtbaarheid omschakelen
action-toggle-gravity = Zwaartekracht omschakelen
action-toggle-playback = Afspelen/pauzeren
action-duplicate-selected = Selectie dupliceren
action-copy-selected = Selectie kopiëren
action-cut-selected = Selectie knippen
action-paste = Plakken
action-reset-transform = Schaal / dekking herstellen
action-bring-forward = Selectie naar voren halen
action-send-backward = Selectie naar achteren sturen
action-fps-up = FPS verhogen
action-fps-down = FPS verlagen
action-opacity-up = Dekking verhogen
action-opacity-down = Dekking verlagen
action-cycle-monitor = Monitorkoppeling doorschakelen
action-show-entity-info = Figuurinfo tonen
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
appearance-large-bubbles-label = Grote tekst in tekstballonnen
appearance-large-bubbles-hint = De tekst in alle tekstballonnen is groter, om op afstand of bij slechtziendheid te lezen.
appearance-hover-startle-label = Schrikken bij zweven
appearance-hover-startle-hint = Mascottes deinzen terug voor de muisaanwijzer als die dichtbij komt. Cursorvolging werkt alleen op X11, dus op native Wayland reageert dit alleen in de bewerkmodus.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Als een app schermvullend is
appearance-fullscreen-hint = Games, video’s en presentaties die het scherm vullen. Werkt onder X11 en op Wayland-compositors zoals sway en Hyprland; onder GNOME en KDE met Wayland alleen voor apps die via XWayland draaien.
appearance-apps-label = Ook verbergen bij deze apps
appearance-apps-hint = Zolang een ervan het venster op de voorgrond is — een videogesprek, een stream, een game — verbergen de figuren zich, zoals bij een app op volledig scherm. X11 en Wayland-compositors zoals sway en Hyprland; onder GNOME en KDE met Wayland alleen apps die via XWayland draaien.
appearance-apps-remove = { $app } verwijderen
appearance-apps-add = Een geopende app toevoegen…
appearance-apps-none-open = Er is geen app open die deze sessie kan zien.
appearance-behind-windows-label = Achter uw vensters
appearance-behind-windows-hint = De figuren leven op het bureaublad, onder uw vensters, in plaats van erboven. De bewerkmodus haalt ze naar voren; is de ⚙-knop bedekt, dan opent Ctrl+Shift+A of het systeemvak hem.
appearance-tap-label = Figuren kunnen aangetikt worden
appearance-tap-hint = Ook buiten de bewerkmodus: een tik op een figuur port hem — een sprongetje en een eigen zin. Klikken elders komen nog steeds bij uw vensters, en de app waarin u typt houdt het toetsenbord.
appearance-effects-label = Kleine effecten
appearance-effects-hint = Hartjes als u op een figuur tikt, een stofwolkje waar er een hard landt, en z'tjes als ze in slaap vallen. Geen met Beweging verminderen.
appearance-bedtime-label = Bedtijd
appearance-bedtime-hint = De figuren dommelen tussen deze tijden in, wie er ook achter het toetsenbord zit; de bewerkmodus maakt ze wakker zolang hij duurt. De nacht telt als pauze voor de herinneringen.
appearance-moods-label = Stemming naar het uur
appearance-moods-hint = ’s Ochtends levendiger, ’s avonds trager — lopen en animatie volgen de klok — en met bedtijd aan een gaap in het halfuur ervoor.
appearance-sounds-mute = Geluiden dempen
appearance-sounds-hint = Aantikken, herinneringen, de focustimer en scripts. Gedempt is er niets te horen en blijft het geluidsapparaat gesloten.
appearance-sounds-volume = Volume
appearance-sounds-bedtime = Stil tijdens bedtijd
appearance-sounds-bedtime-hint = Geen geluiden terwijl de figuren slapen tijdens bedtijd.
mood-yawn = *gaap*
appearance-bedtime-from = van
appearance-bedtime-to = tot
fullscreen-hide = Figuren verbergen
fullscreen-pause = Pauzeren
fullscreen-ignore = Door laten gaan
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pauzeren als ik weg ben
appearance-away-hint = Zonder muis- of toetsenbordinvoer gedurende deze tijd stoppen de figuren, landen ze en dommelen ze in; uw volgende invoer maakt ze wakker.
away-never = Nooit
away-after-minutes = Na { $minutes } min
appearance-away-unavailable = Deze sessie kan niet zien wanneer u weg bent: daarvoor is GNOME nodig, een echte X-server of een Wayland-compositor met inactiviteitsmeldingen.
appearance-battery-label = Pauzeren op batterij
appearance-battery-hint = De figuren staan stil zolang de computer op de batterij draait.
appearance-welcome-label = Groeten als ik terugkom
appearance-welcome-hint = Na een tijdje weg van de computer groet een figuur bij terugkomst — ‘Goedemorgen!’ na de nacht. Niet tijdens bedtijd.
appearance-backup = Back-up maken…
appearance-backup-hint = Alles in één bestand — instellingen, opgeslagen scènes en hun afbeeldingen — om te bewaren of mee te nemen naar een andere computer.
appearance-restore = Herstellen…
appearance-restore-hint = Een back-up terugzetten: de scènes komen bij de uwe en de instellingen vervangen deze (de oude blijven bewaard als config.toml.before-restore).
welcome-back = Welkom terug!
welcome-morning = Goedemorgen!
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Starten bij aanmelden
appearance-autostart-hint = animaEngine starten wanneer u zich aanmeldt bij het bureaublad.
appearance-autostart-reason = Om animaEngine te starten bij het aanmelden.
appearance-autostart-failed = Kon het niet wijzigen: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Scènes
scene-scenes-hint = Sla op wat op het scherm staat onder een naam, en ga er hier naar terug, via het opdrachtenpalet of met ‘Volgende scène’ in het systeemvak.
scene-scene-name-hint = Naam van de scène
scene-scene-save-new = Opslaan als scène
scene-scene-load-tooltip = Naar deze scène wisselen — Ongedaan maken wisselt terug
scene-scene-save-over = Wat op het scherm staat opslaan als { $name }
scene-scene-share = { $name } delen als één bestand, met de afbeeldingen
scene-import = Importeren…
scene-import-tooltip = Open een scène die met u is gedeeld (een .animascene-bestand). Het bestand op de overlay slepen werkt ook.
scene-scene-delete = Scène verwijderen
scene-scene-delete-confirm = Verwijderen?
scene-scene-delete-cancel = Behouden
scene-scene-save-failed = Niet opgeslagen: { $error }
toast-scene-loaded = Scène: { $name }
toast-scene-partly-read = Sommige figuren konden niet worden gelezen — van een nieuwere versie of met typefouten — en zijn weggelaten.
toast-scene-failed = Scène niet geladen: { $error }
toast-scene-shared = Scène klaar om te delen: { $path }
toast-scene-shared-scripts = Gedragsscripts worden niet gedeeld; figuren die in het bestand stilstaan: { $count }
toast-scene-share-failed = Delen mislukt: { $error }
toast-gif-saved = GIF opgeslagen: { $path }
toast-gif-failed = De GIF kon niet worden opgeslagen: { $error }
toast-character-shared = Figuur klaar om te delen: { $path }
toast-character-shared-script = Het gedragsscript wordt niet gedeeld: in het bestand staat de figuur stil.
toast-character-added = { $name } toegevoegd
toast-characters-added = Figuren toegevoegd: { $count }
toast-backup-saved = Back-up gemaakt: { $path }
toast-backup-failed = Back-up of herstel mislukt: { $error }
toast-restored = Hersteld: de instellingen, en scènes toegevoegd: { $count }
toast-scene-imported = Scène geïmporteerd: { $name }
toast-scene-import-failed = Importeren mislukt: { $error }
toast-no-scenes = Nog geen opgeslagen scènes — sla er een op in het tabblad Scène.
palette-load-scene = Naar scène wisselen: { $name }
palette-snooze-minutes = { $minutes } minuten sluimeren
palette-focus-start = Focustimer starten
palette-focus-stop = Focustimer stoppen
palette-snooze-morning = Sluimeren tot de ochtend
tray-next-scene = Volgende scène
tray-show-overlay = Overlay tonen
tray-hide-overlay = Overlay verbergen
tray-snoozing-until = Sluimert tot { $until }
tray-wake = Nu wakker worden
tray-quit = Afsluiten
palette-mute = Geluiden dempen
palette-unmute = Geluiden weer aanzetten
# Reminders (1.5): machine-translated, pending native review.
scene-reminders-header = Herinneringen
scene-reminders-hint = Een figuur zegt het af en toe, in een tekstballon. Tijd weg van de computer telt als pauze en begint opnieuw.
scene-focus-header = Focustimer
scene-focus-hint = Een tijdje werken, dan zegt een figuur dat het tijd is voor een pauze — en wanneer het weer verder gaat. Herinneringen wachten zolang. Ook via het opdrachtenpalet of met ‘Focustimer starten’ in het systeemvak.
scene-focus-work = Werk
scene-focus-break = Pauze
scene-focus-start = Starten
scene-focus-stop = Stoppen
scene-focus-working = Focus tot { $until }
scene-focus-on-break = Pauze tot { $until }
reminder-sound-label = Geluid bij herinneringen en de focustimer
reminder-sound-none = Geen
reminder-sound-speaker = Het tikgeluid van de figuur die spreekt
reminder-notify-label = Ook als bureaubladmeldingen
reminder-notify-hint = Herinneringen en de focustimer verschijnen ook als bureaubladmeldingen — meteen als de figuren niet te zien zijn, in plaats van op ze te wachten.
focus-said-started = Tijd om te focussen — tot { $until }.
focus-said-break = Pauze! Om { $until } gaan we verder.
focus-said-back = De pauze is voorbij — weer aan de slag tot { $until }.
scene-reminder-every = elke { $minutes } min
scene-reminder-anyone = een willekeurig figuur
scene-reminder-delete = Herinnering verwijderen
scene-reminder-text-hint = Wat er gezegd wordt
scene-reminder-minutes = Elke (minuten)
scene-reminder-timed = Op een vast tijdstip
scene-reminder-at = om
scene-reminder-on-date = op een datum
scene-reminder-days = Dagen
scene-reminder-date = Datum
scene-reminder-who = Wie het zegt
scene-reminder-add = Herinnering toevoegen
reminder-break = Tijd voor een korte pauze!
reminder-water = Drink een glas water.
reminder-stretch = Sta op en rek u even uit.
# Scenes by time of day (1.5): machine-translated, pending native review.
scene-schedule-header = Automatisch wisselen
scene-schedule-hint = Wissel op vaste tijden naar een opgeslagen scène — Werk om 09:00 op werkdagen, bijvoorbeeld. Niet terwijl u bewerkt: dan wacht het.
scene-schedule-needs-scene = Sla hierboven een scène op om er volgens een schema naar te wisselen.
scene-schedule-row = { $time } · { $days } → { $scene }
scene-schedule-every-day = elke dag
scene-schedule-weekdays = werkdagen
scene-schedule-weekends = weekenden
scene-schedule-at = Om
scene-schedule-minute = Minuut
scene-schedule-days = Dagen
scene-schedule-scene = Scène
scene-schedule-add = Toevoegen
scene-schedule-delete = Regel verwijderen

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Globale sneltoetsen konden niet worden geregistreerd (gebruikelijk in een native Wayland-sessie). Het menu in het systeemvak en de ⚙-knop blijven werken.
warning-hot-reload-disconnected = De hot-reload-worker is onverwacht gestopt; lopende configuratiewijzigingen gelden pas na een herstart.
# Machine-translated, pending native review.
warning-config-unreadable = Uw configuratiebestand kon niet worden gelezen, dus de standaardscène is geladen. Het origineel staat ernaast als config.toml.bak-corrupt.
warning-config-partly-unread = Een deel van uw configuratiebestand kon niet worden gelezen — het komt misschien van een nieuwere versie van animaEngine of bevat een typefout. De app gaat zonder verder: figuren erin zijn verborgen en instellingen erin staan op hun standaardwaarde, maar alles blijft in het bestand bewaard.
action-toggle-perf-overlay = Prestatie-overlay omschakelen

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Nieuw in 1.5
whats-new-alive = De figuren komen tot leven: tik ze aan, en laat ze elkaar groeten, tikkertje spelen of elkaar volgen.
whats-new-your-day = Herinneringen op een vast tijdstip, een focustimer, bedtijd en sluimeren — en een groet als u terugkomt.
whats-new-editing = Ongedaan maken met Ctrl+Z, meerdere figuren tegelijk selecteren, groeperen en uitlijnen.
whats-new-files = Scènes krijgen een naam, een afbeelding en een schema; deel een scène of één figuur, of maak een back-up van alles in één bestand.
whats-new-sound = Geluiden op het volume dat u wilt, met een eigen volume voor elke figuur, stilte bij bedtijd en dempen vanuit het systeemvak of met een sneltoets.
whats-new-keyboard = Alles ook met het toetsenbord: Shift+F10 opent het menu van een figuur, Ctrl+A selecteert ze allemaal, N gaat ze een voor een langs.
onboarding-keybindings = Klik op een sneltoets om hem te verwijderen; druk een combinatie om een nieuwe op te nemen.
onboarding-perf-overlay = Druk Ctrl+Shift+` om de live prestatie-overlay te openen.
appearance-reset-onboarding = Introductietips herstellen

scene-empty-action-browse-presets = Presets verkennen
library-empty-action-copy-path = Pad naar klembord kopiëren

appearance-reset-onboarding-hint = Haalt de weggeklikte tips en het ‘Wat is nieuw’-paneel terug.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Toestemming voor sneltoetsen geweigerd — er worden X11-sneltoetsen gebruikt. Probeer opnieuw via het tabblad Sneltoetsen.
portal-denied-native-toast = Toestemming voor sneltoetsen geweigerd — het menu in het systeemvak en compositor-bindingen blijven werken.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Globale sneltoetsen via:
keybindings-backend-tooltip = Welk mechanisme de drie globale sneltoetsen (bewerken, verbergen, pauzeren) levert terwijl andere apps de focus hebben. Bepaald bij het opstarten; in-app-sneltoetsen blijven ongemoeid.
keybindings-backend-portal-waiting = De desktop (wacht op uw goedkeuring)
keybindings-backend-portal = De desktop
keybindings-backend-grab = De app zelf
keybindings-backend-grab-declined = De app zelf (toestemming voor sneltoetsen geweigerd)
keybindings-backend-compositor = Geen — wijs toetsen toe in uw compositor, of gebruik het menu in het systeemvak
keybindings-backend-tray = Geen — gebruik het menu in het systeemvak
keybindings-portal-restart-hint = Triggerwijzigingen gelden vanaf de volgende start (de desktop onthoudt uw goedkeuring).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = { $n ->
        [0] Monitor { $name } losgekoppeld.
        [one] Monitor { $name } losgekoppeld — { $n } vastgezette figuur volgt nu zijn positie.
       *[other] Monitor { $name } losgekoppeld — { $n } vastgezette figuren volgen nu hun positie.
    }
monitor-plugged-toast = Monitor { $name } aangesloten.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Shimeji-pakket importeren
library-import-shimeji-hint = Sleep de pakketmap op de overlay of plak het pad hier. Sprites worden naar de bibliotheek gekopieerd.
library-import-shimeji-button = Importeren
shimeji-imported-toast = { $n ->
        [0] { $name } geïmporteerd
        [one] { $name } geïmporteerd ({ $n } onderdeel overgeslagen — zie log)
       *[other] { $name } geïmporteerd ({ $n } onderdelen overgeslagen — zie log)
    }
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
toast-copied = { $name } gekopieerd
toast-copied-many = Figuren gekopieerd: { $count }
toast-cut = { $name } geknipt
toast-cut-many = Figuren geknipt: { $count }
toast-pasted = { $name } geplakt
toast-pasted-many = Figuren geplakt: { $count }
toast-paste-failed = Plakken mislukt: { $error }
toast-nothing-to-paste = Niets om te plakken: kopieer eerst een figuur of een afbeelding
pasted-image-name = Geplakte afbeelding
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

# Why a file was not taken, presets, the tray's tooltip, demo names (1.5)
error-file-unreadable = Het bestand kon niet worden gelezen: { $error }
error-not-a-file = Dat is geen bestand.
error-file-too-big = Het bestand is { $size } MB; er kan hoogstens { $max } MB worden toegevoegd.
error-file-type = Bestanden met .{ $ext } kunnen niet worden getoond; PNG, JPEG, GIF, WebP en MP4 wel.
error-file-no-extension = De bestandsnaam heeft geen extensie die zegt wat het is; PNG, JPEG, GIF, WebP en MP4 kunnen worden getoond.
error-asset-not-found = Niet gevonden: { $path }
error-not-a-folder = Geen map: { $path }
error-no-frames = Geen afbeeldingen in { $path }
error-image-too-large = De afbeelding is { $width }×{ $height }; hoogstens { $max }×{ $max } kan worden getoond.
error-share-too-big = De scène is groter dan { $max } MB — te groot om als één bestand te delen.
error-backup-too-big = Alles samen is groter dan { $max } MB — te veel voor één back-up.
error-picture-not-shareable = { $path } is geen afbeelding die kan worden gedeeld.
error-file-over = Het bestand is groter dan { $max } MB.
error-not-a-scene-file = Dat is geen scène of figuur uit animaEngine.
error-not-a-backup = Dat is geen back-up van animaEngine.
error-file-damaged = Het bestand is beschadigd of niet door animaEngine gemaakt.
error-file-newer = Het is gemaakt door een nieuwere versie van animaEngine; werk animaEngine bij om het te openen.
shimeji-no-actions = Het pakket heeft geen actions.xml in de map conf.
shimeji-no-idle = Het pakket heeft geen rusthouding (een Stay-actie).
shimeji-not-utf8 = De XML van het pakket is geen UTF-8; pakketten in Shift-JIS kunnen niet worden gelezen.
shimeji-too-big = Het pakket is te groot: hoogstens { $sprites } afbeeldingen en { $max } MiB.
tray-tooltip = Rechtsklik voor opties
preset-cozy-companion = Knus gezelschap
preset-cozy-companion-about = Eén zacht spook dat rustig aan de rechterkant rondzwerft.
preset-productivity-zen = Productieve zen
preset-productivity-zen-about = Een slijmpje en een hart, stil, in een rustig hoekje.
preset-halloween-party = Halloweenfeest
preset-halloween-party-about = Drie spoken marcheren langs de onderrand.
preset-birthday-confetti = Verjaardagsconfetti
preset-birthday-confetti-about = Het regent harten en sterren. Zwaartekracht aan.
preset-studio-session = Studiosessie
preset-studio-session-about = Alle vijf demofiguren verspreid over het bureaublad.
preset-cursor-follower = Cursorvolger
preset-cursor-follower-about = Een kat volgt uw muis overal.
preset-companion = Gezelschap
preset-zen-slime = Zenslijmpje
preset-zen-heart = Zenhart
preset-ghost-n = Spook { $n }
demo-ghost = Spook
demo-slime = Slijmpje
demo-heart = Hart
demo-star = Ster
demo-cat = Kat
demo-name = { $name } (demo)
scene-shared-unnamed = Gedeelde scène
