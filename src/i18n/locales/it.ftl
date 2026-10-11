# Italiano — traduzione di base. Revisione madrelingua in sospeso.

app-name = animaEngine

settings-tab-inspector = Ispettore
settings-tab-scene = Scena
settings-tab-appearance = Aspetto
entity-count-zero = Nessun personaggio
entity-count-singular = { $n } personaggio
entity-count-plural = { $n } personaggi

inspector-section-position = Posizione
inspector-section-appearance = Aspetto
inspector-section-animation = Animazione
animation-easing-label = Easing
easing-linear = Lineare
easing-ease-in-quad = Entrata morbida
easing-ease-out-quad = Uscita morbida
easing-ease-in-out-quad = Entrata/uscita morbida
easing-sine = Seno
easing-bounce-out = Rimbalzo
inspector-section-behavior = Comportamento
inspector-section-talk = Dice
inspector-talk-hint = Una frase per riga. Un tocco sul personaggio in modalità modifica gliene fa dire una, e può anche parlare da solo.
inspector-talk-placeholder = Ciao!
inspector-talk-pace = Parla
inspector-talk-only-poked = Solo se toccato
inspector-talk-every = Circa ogni { $minutes } min
inspector-talk-say-now = Dinne una ora
placeholders-hint = In una frase, { $time }, { $day }, { $date } e { $name } vengono compilati quando viene detta.
placeholder-date = { $day } { $month }
weekday-0 = domenica
weekday-1 = lunedì
weekday-2 = martedì
weekday-3 = mercoledì
weekday-4 = giovedì
weekday-5 = venerdì
weekday-6 = sabato
month-1 = gennaio
month-2 = febbraio
month-3 = marzo
month-4 = aprile
month-5 = maggio
month-6 = giugno
month-7 = luglio
month-8 = agosto
month-9 = settembre
month-10 = ottobre
month-11 = novembre
month-12 = dicembre
inspector-sound-label = Suono al tocco
inspector-sound-none = Nessuno
inspector-sound-play = Riproduci
inspector-sound-hint = I file audio (.ogg, .wav, .flac, .mp3) messi in { $path } compaiono qui.
inspector-bubble-colour = Colore del fumetto
inspector-bubble-size = Dimensione del testo
inspector-bubble-reset = Predefinito
inspector-hours-label = Solo in certi orari
inspector-hours-hint = Fuori da questi orari non è sullo schermo: il fantasma di notte, il sole di giorno. In modalità modifica ci sono tutti, per poterli modificare.
inspector-hours-from = dalle
inspector-hours-to = alle
inspector-visible = Visibile
inspector-gravity = Gravità
inspector-scale = Scala
inspector-behavior-speed = Velocità
inspector-follow-target = Segue
inspector-follow-distance = Distanza
inspector-follow-nobody = (nessuno)
inspector-follow-none-free = Nessuno da seguire: gli altri non ci sono o seguono già questo.
inspector-tag-partner = Gioca con
inspector-tag-hint = Si rincorrono per un po’ — chi sta sotto insegue l’altro — poi riposano per un minuto e mezzo. L’altro non ha bisogno di alcuna impostazione.
inspector-behavior-comfort = Distanza di comfort
inspector-behavior-amplitude = Ampiezza
inspector-behavior-period = Periodo
inspector-double-click-reset-hint = Doppio clic per ripristinare il valore predefinito.
inspector-opacity = Opacità
inspector-fps = FPS
inspector-playing = In riproduzione
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nessuna selezione
inspector-nothing-selected-hint = Clicca un personaggio nella scheda Scena, o premi Tab per scorrerli.

behavior-idle = In riposo
behavior-walk = Cammina
behavior-follow = Segui il cursore
behavior-follow-character = Segui un personaggio
behavior-play-tag = Giocare ad acchiapparella
behavior-wander = Vagare entro limiti
behavior-bounce = Rimbalzo
behavior-bounce-axis = Asse
behavior-bounce-horizontal = Orizzontale
behavior-bounce-vertical = Verticale
behavior-bounce-both = Entrambi (cerchio)
behavior-script = Script
behavior-script-path-label = Percorso
behavior-script-path-hint = Relativo alla tua libreria di risorse — { $path }
behavior-script-params = Parametri
behavior-script-param-name = nome
behavior-script-add-param = Aggiungi
behavior-script-remove-param = Rimuovi questo parametro
script-failed-toast = Lo script di comportamento { $script } non è riuscito: { $error }
sound-failed-toast = Impossibile riprodurre il suono { $sound }: { $error }

scene-empty-headline = Scena vuota
scene-empty-hint = Trascina un PNG / GIF / WebP / MP4 sull'overlay — o prova un preset qui sotto.
scene-drop-hint = Trascina un PNG / GIF / WebP sull'overlay per aggiungere un personaggio.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Aggiungi file…
scene-add-file-tooltip = Scegli immagini o video da aggiungere come personaggi.
scene-paste = Incolla
scene-paste-tooltip = Aggiunge i personaggi copiati a questa scena. Copiati: { $count }
scene-paste-empty = Incolla un’immagine o dei file copiati altrove, oppure dei personaggi: selezionali e premi Ctrl+C, o fai clic destro su di essi e scegli Copia, in questa scena o in un’altra.
file-chooser-title = Aggiungi personaggi
file-chooser-filter = Immagini e video
share-chooser-title = Condividi una scena
gif-chooser-title = Salva una GIF
character-chooser-title = Condividi un personaggio
backup-chooser-title = Backup di animaEngine
restore-chooser-title = Ripristina un backup
restore-chooser-filter = Backup di animaEngine
import-chooser-title = Importa una scena
import-chooser-filter = Scene e personaggi di animaEngine
file-chooser-unavailable-toast = Impossibile aprire il selettore di file. Richiede xdg-desktop-portal.
scene-presets-header = Preset
scene-groups-header = Gruppi
scene-characters-header = Personaggi
scene-together-header = Insieme
scene-preset-append = Aggiungi
scene-preset-replace = Sostituisci
scene-preset-replace-tooltip = Cancella la scena attuale prima di aggiungere

monitor-section-header = Monitor
monitor-mode-label = Distribuzione
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Estendi su tutti i monitor
monitor-mode-span-unsupported = Su questo backend un solo overlay non può coprire più monitor: usa «per monitor».
monitor-mode-single = Monitor singolo
scene-window-awareness = Atterra sulle finestre (X11)
scene-window-awareness-tooltip = I personaggi con fisica attiva atterrano e camminano sul bordo superiore delle finestre aperte. Solo sessioni X11 — Wayland non espone le posizioni delle finestre, quindi lì non ha effetto.
scene-window-awareness-unavailable = Non disponibile su Wayland nativo: nessun protocollo espone la posizione delle finestre.
monitor-pin-label = Fissa al monitor
monitor-pin-auto = Auto (segue la posizione)
monitor-pinned-toast = Personaggio fissato a { $name }
monitor-pin-cleared-toast = Il personaggio segue ora la sua posizione
monitor-no-monitors-detected = Nessun monitor rilevato

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Lingua
appearance-language-no-font = Nessun font installato per questa scrittura: installa prima un pacchetto Noto CJK.
theme-dark = Scuro
theme-light = Chiaro
theme-dark-hc = Scuro · Contrasto elevato
theme-light-hc = Chiaro · Contrasto elevato

onboarding-tabs = Le impostazioni sono su cinque schede — Ispettore, Scena, Libreria, Aspetto, Scorciatoie.
onboarding-quick-toggles = Suggerimento: V alterna la visibilità, G la gravità — senza aprire questo pannello.
onboarding-theme = I temi si applicano subito — nessun riavvio richiesto.
onboarding-coach-step1 = Benvenuto! I tuoi personaggi vivono sul desktop. Fai clic sull’ingranaggio in alto a destra per entrare in modalità modifica.
onboarding-coach-step2 = Trascina un PNG, GIF, WebP o MP4 ovunque sullo schermo per aggiungerlo come personaggio. Il pannello laterale modifica tutto ciò che selezioni.
onboarding-coach-step3 = Ctrl+K apre la palette dei comandi. Ctrl+Shift+A attiva la modalità modifica da ovunque, Ctrl+Shift+H nasconde l’overlay.
onboarding-coach-next = Avanti
onboarding-coach-skip = Salta il tour
onboarding-coach-done = Capito
palette-replace-row = Sostituisci la scena con: { $preset }
palette-append-row = Aggiungi preset: { $preset }
palette-footer-hint = Esc chiude · Ctrl+K alterna · ↑↓ + Invio sceglie
onboarding-dismiss = Chiudi

menu-duplicate = Duplica
menu-copy = Copia
menu-save-gif = Salva come GIF…
menu-share-character = Condividi il personaggio…
menu-cut = Taglia
menu-reset-transform = Reimposta trasformazione
menu-toggle-gravity = Attiva/disattiva gravità
menu-bring-forward = Porta in primo piano
menu-send-backward = Manda in fondo
menu-delete = Elimina

toggle-enter-edit = Entra in modalità modifica
toggle-exit-edit = Esci dalla modalità modifica

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Cerca azioni, temi e preset…
palette-switch-theme = Passa al tema { $theme }

settings-tab-library = Libreria

# Asset library tab
library-empty-headline = Nessun asset indicizzato
library-empty-hint = Trascina file in { $path } o imposta ANIMA_ASSETS_DIR.
library-no-asset-root = Nessuna directory di asset trovata. Creane una in { $path }
library-search-placeholder = Cerca asset…
library-add-to-scene = Aggiungi alla scena
library-kind-image = Immagine
library-kind-animated = Animato
library-kind-video = Video
library-asset-added-toast = { $name } aggiunto alla scena
library-asset-add-failed-toast = Impossibile aggiungere { $name }
library-count = { $n } asset indicizzati

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Scorciatoie
keybindings-unbound = (non assegnata)
keybindings-add = Aggiungi
keybindings-recording = Premi una combinazione… (Esc annulla)
keybindings-conflict = In conflitto con { $action }
keybindings-reset-all = Ripristina tutto ai valori predefiniti
keybindings-reset-one = Ripristina il valore predefinito
keybindings-remove-chord = Rimuovi questa associazione
keybindings-help = Le scorciatoie personalizzate vengono salvate in config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Attiva/disattiva modalità modifica
action-hide-overlay = Nascondi / mostra l’overlay
action-pause-all = Metti in pausa tutte le animazioni
action-toggle-focus = Avvia o ferma il timer di concentrazione
action-toggle-snooze = Sospendi per 15 minuti o risveglia
action-toggle-mute = Disattiva o riattiva i suoni
action-quit-with-save = Esci (salvando la configurazione)
action-save-now = Salva subito la configurazione
action-open-command-palette = Palette dei comandi
# Undo (1.5): machine-translated, pending native review.
action-undo = Annulla
action-redo = Ripeti

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Raggruppa la selezione
action-ungroup-selected = Separa la selezione
menu-group = Raggruppa
menu-ungroup = Separa
group-default-name = Gruppo { $number }
group-copy-name = { $name } (copia)
toast-grouped = Raggruppati come { $name }
toast-ungrouped = Gruppi sciolti: { $count }
toast-nothing-to-ungroup = Niente di selezionato è in un gruppo
scene-groups-empty-hint = Seleziona più personaggi e premi Ctrl+G, oppure fai clic destro su di essi e scegli Raggruppa.
scene-group-members = Personaggi: { $count }
scene-group-select-tooltip = Seleziona i personaggi di questo gruppo
scene-group-rename = Rinomina il gruppo
scene-group-show = Mostra il gruppo
scene-group-hide = Nascondi il gruppo
scene-group-ungroup = Separa: i personaggi restano

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Allinea i bordi sinistri
arrange-align-center = Centra su una linea verticale
arrange-align-right = Allinea i bordi destri
arrange-align-top = Allinea i bordi superiori
arrange-align-middle = Centra su una linea orizzontale
arrange-align-bottom = Allinea i bordi inferiori
arrange-distribute-horizontally = Distribuisci in orizzontale
arrange-distribute-vertically = Distribuisci in verticale
scene-snap = Aggancia durante il trascinamento
scene-snap-tooltip = Bordi e centri si agganciano a quelli degli schermi e degli altri personaggi. Tieni premuto Alt mentre trascini per posizionare liberamente.
scene-bump = I personaggi si scontrano
scene-bump-tooltip = Con la gravità attiva (G), i personaggi atterrano sulla testa degli altri; chi cammina torna indietro quando si incontrano.
scene-greet = I personaggi si salutano quando si incontrano
scene-greet-tooltip = Due che si avvicinano — camminando, seguendosi, atterrando l’uno sull’altro — si fermano un attimo e si salutano, con un cuore. Non troppo spesso.
greet-hi = Ciao!
greet-hello = Salve!
greet-hey = Ehi!

action-cycle-entity = Passa al personaggio successivo
action-delete-selected = Elimina il personaggio selezionato
action-nudge-up = Sposta la selezione in alto
action-nudge-down = Sposta la selezione in basso
action-nudge-left = Sposta la selezione a sinistra
action-nudge-right = Sposta la selezione a destra
action-center-on-screen = Centra la selezione sullo schermo
action-toggle-visible = Attiva/disattiva visibilità
action-toggle-gravity = Attiva/disattiva gravità
action-toggle-playback = Riproduci/Pausa
action-duplicate-selected = Duplica la selezione
action-copy-selected = Copia la selezione
action-cut-selected = Taglia la selezione
action-paste = Incolla
action-reset-transform = Ripristina scala / opacità
action-bring-forward = Porta la selezione in avanti
action-send-backward = Manda la selezione indietro
action-fps-up = Aumenta FPS
action-fps-down = Riduci FPS
action-opacity-up = Aumenta opacità
action-opacity-down = Riduci opacità
action-cycle-monitor = Cambia il monitor agganciato
action-show-entity-info = Mostra info del personaggio
action-show-help = Mostra la guida della tastiera

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Accessibilità
appearance-accesskit-label = Genera gli aggiornamenti dell’albero AccessKit
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Permette agli screen reader (Orca e altri, tramite AT-SPI) di leggere e usare i pannelli. Disattivato, uno screen reader vede una finestra vuota. Nota: mentre uno è in esecuzione, il testo digitato nei pannelli passa sul bus AT-SPI, dove qualsiasi processo del tuo utente può leggerlo.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Gli screen reader non sono ancora supportati su questo sistema.
appearance-reduced-motion-label = Riduci il movimento
appearance-reduced-motion-hint = Salta le transizioni dell’interfaccia (scorrimento del pannello, dissolvenze, comparsa della palette) e ferma l’oscillazione decorativa. Le animazioni che comunicano uno stato restano attive.
appearance-large-bubbles-label = Testo grande nei fumetti
appearance-large-bubbles-hint = Il testo di tutti i fumetti è più grande, per leggerlo da lontano o con ipovisione.
appearance-hover-startle-label = Sussulto al passaggio
appearance-hover-startle-hint = Le mascotte indietreggiano dal puntatore quando si avvicina. Il tracciamento del cursore è solo su X11, quindi su Wayland nativo reagisce solo in modalità modifica.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Quando un’app è a schermo intero
appearance-fullscreen-hint = Giochi, video e presentazioni che riempiono lo schermo. Funziona su X11 e sui compositor Wayland come sway e Hyprland; su GNOME e KDE con Wayland, solo per le app che girano tramite XWayland.
appearance-apps-label = Nascondi anche per queste app
appearance-apps-hint = Finché una di esse è la finestra in primo piano — una videochiamata, una diretta, un gioco — i personaggi si nascondono, come per un’app a schermo intero. X11 e compositor Wayland come sway e Hyprland; su GNOME e KDE con Wayland, solo le app che girano tramite XWayland.
appearance-apps-remove = Rimuovi { $app }
appearance-apps-add = Aggiungi un’app aperta…
appearance-apps-none-open = Nessuna app visibile a questa sessione è aperta.
appearance-behind-windows-label = Dietro le finestre
appearance-behind-windows-hint = I personaggi vivono sul desktop, sotto le finestre, invece che sopra. La modalità modifica li porta davanti; se il pulsante ⚙ è coperto, aprila con Ctrl+Shift+A o dall’area di notifica.
appearance-tap-label = Si possono toccare i personaggi
appearance-tap-hint = Anche fuori dalla modalità modifica: un tocco su un personaggio lo punzecchia — un salto e una sua frase. I clic altrove arrivano sempre alle tue finestre, e l’app in cui scrivi tiene la tastiera.
appearance-effects-label = Piccoli effetti
appearance-effects-hint = Cuori quando tocchi un personaggio, una nuvola di polvere dove uno atterra con forza e delle z quando si addormentano. Nessuno con Riduci il movimento.
appearance-bedtime-label = Ora di dormire
appearance-bedtime-hint = I personaggi si appisolano tra queste ore, chiunque sia alla tastiera; la modalità modifica li sveglia finché dura. La notte conta come pausa per i promemoria.
appearance-moods-label = Umore secondo l’ora
appearance-moods-hint = Più vivaci al mattino, più lenti la sera — il passo e l’animazione seguono l’orologio — e, con l’ora di dormire attiva, uno sbadiglio nella mezz’ora prima.
appearance-sounds-mute = Disattiva i suoni
appearance-sounds-hint = Tocchi, promemoria, il timer di concentrazione e gli script. Disattivati, non si sente nulla e il dispositivo audio resta chiuso.
appearance-sounds-volume = Volume
appearance-sounds-bedtime = Silenzio all’ora di dormire
appearance-sounds-bedtime-hint = Nessun suono mentre i personaggi dormono all’ora di dormire.
mood-yawn = *sbadiglio*
appearance-bedtime-from = dalle
appearance-bedtime-to = alle
fullscreen-hide = Nascondi i personaggi
fullscreen-pause = Mettili in pausa
fullscreen-ignore = Lasciali andare avanti
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pausa quando sono via
appearance-away-hint = Senza mouse né tastiera per questo tempo, i personaggi si fermano, atterrano e si appisolano; il tuo prossimo input li sveglia.
away-never = Mai
away-after-minutes = Dopo { $minutes } min
appearance-away-unavailable = Questa sessione non può sapere quando sei via: serve GNOME, un vero server X o un compositor Wayland con notifiche di inattività.
appearance-battery-label = Pausa a batteria
appearance-battery-hint = I personaggi restano fermi mentre il computer va a batteria.
appearance-welcome-label = Salutami quando torno
appearance-welcome-hint = Dopo un po’ lontano dal computer, un personaggio ti saluta al ritorno — «Buongiorno!» dopo la notte. Non all’ora di dormire.
appearance-backup = Fai un backup…
appearance-backup-hint = Tutto in un solo file — impostazioni, scene salvate e le loro immagini — da conservare o da portare su un altro computer.
appearance-restore = Ripristina…
appearance-restore-hint = Recupera un backup: le sue scene si aggiungono alle tue e le sue impostazioni sostituiscono queste (le vecchie restano in config.toml.before-restore).
welcome-back = Eccoti di nuovo!
welcome-morning = Buongiorno!
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Avvia all’accesso
appearance-autostart-hint = Avvia animaEngine quando accedi al desktop.
appearance-autostart-reason = Per avviare animaEngine all’accesso.
appearance-autostart-failed = Impossibile cambiarlo: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Scene
scene-scenes-hint = Salva ciò che è sullo schermo con un nome e tornaci qui, dalla palette dei comandi o con «Scena successiva» nell’area di notifica.
scene-scene-name-hint = Nome della scena
scene-scene-save-new = Salva come scena
scene-scene-load-tooltip = Passa a questa scena: Annulla torna indietro
scene-scene-save-over = Salva ciò che è sullo schermo come { $name }
scene-scene-share = Condividi { $name } come un solo file, immagini comprese
scene-import = Importa…
scene-import-tooltip = Apri una scena che qualcuno ti ha condiviso (un file .animascene). Funziona anche trascinare il file sull’overlay.
scene-scene-delete = Elimina scena
scene-scene-delete-confirm = Eliminare?
scene-scene-delete-cancel = Tienila
scene-scene-save-failed = Non salvata: { $error }
toast-scene-loaded = Scena: { $name }
toast-scene-failed = Scena non caricata: { $error }
toast-scene-shared = Scena pronta da condividere: { $path }
toast-scene-shared-scripts = Gli script di comportamento non vengono condivisi; personaggi fermi nel file: { $count }
toast-scene-share-failed = Condivisione non riuscita: { $error }
toast-gif-saved = GIF salvata: { $path }
toast-gif-failed = Impossibile salvare la GIF: { $error }
toast-character-shared = Personaggio pronto da condividere: { $path }
toast-character-shared-script = Il suo script di comportamento non viene condiviso: nel file resta fermo.
toast-character-added = { $name } aggiunto
toast-characters-added = Personaggi aggiunti: { $count }
toast-backup-saved = Backup salvato: { $path }
toast-backup-failed = Backup o ripristino non riuscito: { $error }
toast-restored = Ripristinato: le impostazioni, e scene aggiunte: { $count }
toast-scene-imported = Scena importata: { $name }
toast-scene-import-failed = Importazione non riuscita: { $error }
toast-no-scenes = Ancora nessuna scena salvata: salvane una nella scheda Scena.
palette-load-scene = Passa alla scena: { $name }
palette-snooze-minutes = Sospendi per { $minutes } minuti
palette-focus-start = Avvia il timer di concentrazione
palette-focus-stop = Ferma il timer di concentrazione
palette-snooze-morning = Sospendi fino al mattino
tray-next-scene = Scena successiva
tray-show-overlay = Mostra l’overlay
tray-hide-overlay = Nascondi l’overlay
tray-snoozing-until = Sospensione fino alle { $until }
tray-wake = Risveglia ora
tray-quit = Esci
palette-mute = Disattiva i suoni
palette-unmute = Riattiva i suoni
# Reminders (1.5): machine-translated, pending native review.
scene-reminders-header = Promemoria
scene-reminders-hint = Un personaggio lo dice ogni tanto, in un fumetto. Il tempo lontano dal computer conta come pausa e lo fa ripartire.
scene-focus-header = Timer di concentrazione
scene-focus-hint = Lavora per un po’, poi un personaggio ti dice che è ora di fare una pausa — e quando riprendere. Nel frattempo i promemoria aspettano. Anche dalla palette dei comandi o con «Avvia il timer di concentrazione» nell’area di notifica.
scene-focus-work = Lavoro
scene-focus-break = Pausa
scene-focus-start = Avvia
scene-focus-stop = Ferma
scene-focus-working = Concentrazione fino alle { $until }
scene-focus-on-break = In pausa fino alle { $until }
reminder-sound-label = Suono con i promemoria e il timer di concentrazione
reminder-sound-none = Nessuno
reminder-sound-speaker = Il suono al tocco di chi parla
reminder-notify-label = Anche come notifiche del desktop
reminder-notify-hint = I promemoria e il timer arrivano anche nelle notifiche del desktop — subito, quando i personaggi non si vedono, invece di aspettarli.
focus-said-started = Tempo di concentrazione — fino alle { $until }.
focus-said-break = Pausa! Si riprende alle { $until }.
focus-said-back = Pausa finita — si torna al lavoro fino alle { $until }.
scene-reminder-every = ogni { $minutes } min
scene-reminder-anyone = qualsiasi personaggio
scene-reminder-delete = Elimina promemoria
scene-reminder-text-hint = Cosa dire
scene-reminder-minutes = Ogni (minuti)
scene-reminder-timed = A un’ora precisa
scene-reminder-at = alle
scene-reminder-on-date = in una data
scene-reminder-days = Giorni
scene-reminder-date = Data
scene-reminder-who = Chi lo dice
scene-reminder-add = Aggiungi promemoria
reminder-break = È ora di una breve pausa!
reminder-water = Bevi un bicchiere d’acqua.
reminder-stretch = Alzati e fai stretching.
# Scenes by time of day (1.5): machine-translated, pending native review.
scene-schedule-header = Cambia automaticamente
scene-schedule-hint = Passa a una scena salvata a orari fissi: Lavoro alle 09:00 nei giorni feriali, per esempio. Non mentre modifichi: in quel caso aspetta.
scene-schedule-needs-scene = Salva una scena qui sopra per passarci secondo un orario.
scene-schedule-row = { $time } · { $days } → { $scene }
scene-schedule-every-day = ogni giorno
scene-schedule-weekdays = giorni feriali
scene-schedule-weekends = fine settimana
scene-schedule-at = Alle
scene-schedule-minute = Minuto
scene-schedule-days = Giorni
scene-schedule-scene = Scena
scene-schedule-add = Aggiungi
scene-schedule-delete = Elimina regola

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Impossibile registrare le scorciatoie globali (tipico di una sessione Wayland nativa). Il menu nell’area di notifica e il pulsante ⚙ continuano a funzionare.
warning-hot-reload-disconnected = Il processo di ricarica a caldo si è fermato inaspettatamente; le modifiche alla configurazione in corso si applicheranno solo dopo un riavvio.
# Machine-translated, pending native review.
warning-config-unreadable = Non è stato possibile leggere il file di configurazione, quindi è stata caricata la scena predefinita. L'originale è conservato accanto come config.toml.bak-corrupt.
action-toggle-perf-overlay = Attiva/disattiva overlay prestazioni

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Novità della 1.5
whats-new-alive = I personaggi prendono vita: toccali e lascia che si salutino, giochino ad acchiapparella o si seguano a vicenda.
whats-new-your-day = Promemoria a un’ora precisa, un timer di concentrazione, l’ora di dormire e la sospensione, e un saluto quando torni.
whats-new-editing = Annulla con Ctrl+Z, seleziona più personaggi insieme, raggruppali e allineali.
whats-new-files = Le scene hanno nome, immagine e orario; condividi una scena o un solo personaggio, oppure fai un backup di tutto in un unico file.
onboarding-keybindings = Fai clic su una scorciatoia per rimuoverla; premi una combinazione per registrarne una nuova.
onboarding-perf-overlay = Premi Ctrl+Shift+` per aprire l’overlay prestazioni in tempo reale.
appearance-reset-onboarding = Ripristina i suggerimenti iniziali

scene-empty-action-browse-presets = Sfoglia i preset
library-empty-action-copy-path = Copia il percorso negli appunti

appearance-reset-onboarding-hint = Ripristina i suggerimenti chiusi e il pannello «Novità».

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Permesso per le scorciatoie negato — verranno usate le scorciatoie X11. Riprova dalla scheda Scorciatoie.
portal-denied-native-toast = Permesso per le scorciatoie negato — il menu nell’area di notifica e le scorciatoie del compositor continuano a funzionare.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Scorciatoie globali tramite:
keybindings-backend-tooltip = Quale meccanismo consegna le tre scorciatoie globali (modifica, nascondi, pausa) mentre altre app hanno il focus. Determinato all’avvio; le scorciatoie interne non sono interessate.
keybindings-portal-restart-hint = Le modifiche ai trigger valgono dal prossimo avvio (il desktop ricorda la tua approvazione).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } scollegato — { $n } personaggi agganciati ora seguono la loro posizione.
monitor-plugged-toast = Monitor { $name } collegato.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Importa pacchetto Shimeji
library-import-shimeji-hint = Trascina la cartella del pacchetto sull’overlay o incolla qui il suo percorso. Gli sprite vengono copiati nella libreria.
library-import-shimeji-button = Importa
shimeji-imported-toast = { $name } importato ({ $n } parti saltate — vedi il log)
shimeji-import-failed-toast = Importazione non riuscita: { $reason }
shimeji-no-library-toast = Nessuna cartella libreria — crea prima { $path }.
crash-report-found-toast = La sessione precedente si è chiusa in modo imprevisto. Un rapporto è stato salvato in { $path } — allegalo a un issue su GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composto dal gruppo { $group }: { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } selezionati. Questo pannello modifica { $name }; il trascinamento, le scorciatoie e il menu del clic destro agiscono su tutti.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configurazione salvata
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Configurazione ricaricata dal disco
toast-config-reload-failed = Configurazione non ricaricata: il file non è valido o è ancora in scrittura. La scena attuale resta invariata.
toast-config-reload-discarded = Configurazione non ricaricata: hai modificato la scena durante il caricamento.
toast-save-failed = Salvataggio non riuscito: { $error }
toast-rejected = Rifiutato: { $reason }
toast-added = { $name } aggiunto
toast-load-failed = Caricamento non riuscito: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Tema: { $theme }
toast-preset-entry-failed = Impossibile aggiungere la voce del preset: { $error }
toast-preset-loaded = Preset caricato: { $name }
toast-duplicated = { $name } duplicato
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Personaggi duplicati: { $count }
toast-duplicate-failed = Duplicazione non riuscita: { $error }
toast-deleted = { $name } eliminato
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Personaggi eliminati: { $count }
toast-copied = { $name } copiato
toast-copied-many = Personaggi copiati: { $count }
toast-cut = { $name } tagliato
toast-cut-many = Personaggi tagliati: { $count }
toast-pasted = { $name } incollato
toast-pasted-many = Personaggi incollati: { $count }
toast-paste-failed = Incolla non riuscito: { $error }
toast-nothing-to-paste = Niente da incollare: copia prima un personaggio o un’immagine
pasted-image-name = Immagine incollata
toast-playback-resumed = Riproduzione ripresa
toast-playback-paused = Riproduzione in pausa
# Undo (1.5): machine-translated, pending native review.
toast-undone = Annullato
toast-redone = Ripetuto
toast-nothing-to-undo = Niente da annullare
toast-nothing-to-redo = Niente da ripetere
inspector-wander-box = Area di vagabondaggio
toast-perf-snapshot = Istantanea prestazioni: { $path }
toast-perf-snapshot-failed = Istantanea non riuscita: { $error }
