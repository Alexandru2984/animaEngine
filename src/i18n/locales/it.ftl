# Italiano — traduzione di base. Revisione madrelingua in sospeso.

app-name = animaEngine

settings-tab-inspector = Ispettore
settings-tab-scene = Scena
settings-tab-appearance = Aspetto
entity-count-zero = Nessuna entità
entity-count-singular = { $n } entità
entity-count-plural = { $n } entità

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
inspector-visible = Visibile
inspector-gravity = Gravità
inspector-scale = Scala
inspector-behavior-speed = Velocità
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
inspector-nothing-selected-hint = Clicca un'entità nella scheda Scena, o premi Tab per scorrerle.

behavior-idle = In riposo
behavior-walk = Cammina
behavior-follow = Segui il cursore
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

scene-empty-headline = Scena vuota
scene-empty-hint = Trascina un PNG / GIF / WebP / MP4 sull'overlay — o prova un preset qui sotto.
scene-drop-hint = Trascina un PNG / GIF / WebP sull'overlay per aggiungere un'entità.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Aggiungi file…
scene-add-file-tooltip = Scegli immagini o video da aggiungere come personaggi.
file-chooser-title = Aggiungi personaggi
file-chooser-filter = Immagini e video
file-chooser-unavailable-toast = Impossibile aprire il selettore di file. Richiede xdg-desktop-portal.
scene-presets-header = Preset
scene-groups-header = Gruppi
scene-preset-append = Aggiungi
scene-preset-replace = Sostituisci
scene-preset-replace-tooltip = Cancella la scena attuale prima di aggiungere

monitor-section-header = Monitor
monitor-mode-label = Distribuzione
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Estendi su tutti i monitor
monitor-mode-span-unsupported = Su questo backend una sola sovrapposizione non può coprire più monitor: usa «per monitor».
monitor-mode-single = Monitor singolo
scene-window-awareness = Atterra sulle finestre (X11)
scene-window-awareness-tooltip = I personaggi con fisica attiva atterrano e camminano sul bordo superiore delle finestre aperte. Solo sessioni X11 — Wayland non espone le posizioni delle finestre, quindi lì non ha effetto.
scene-window-awareness-unavailable = Non disponibile su Wayland nativo: nessun protocollo espone la posizione delle finestre.
monitor-pin-label = Fissa al monitor
monitor-pin-auto = Auto (segue la posizione)
monitor-pinned-toast = Entità fissata a { $name }
monitor-pin-cleared-toast = L'entità segue ora la sua posizione
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
appearance-hover-startle-label = Sussulto al passaggio
appearance-hover-startle-hint = Le mascotte indietreggiano dal puntatore quando si avvicina. Il tracciamento del cursore è solo su X11, quindi su Wayland nativo reagisce solo in modalità modifica.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Quando un’app è a schermo intero
appearance-fullscreen-hint = Giochi, video e presentazioni che riempiono lo schermo. Funziona su X11 e sui compositor Wayland come sway e Hyprland; su GNOME e KDE con Wayland, solo per le app che girano tramite XWayland.
fullscreen-hide = Nascondi i personaggi
fullscreen-pause = Mettili in pausa
fullscreen-ignore = Lasciali andare avanti
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pausa quando sono via
appearance-away-hint = Senza mouse né tastiera per questo tempo, i personaggi restano fermi; riprendono al tuo prossimo input.
away-never = Mai
away-after-minutes = Dopo { $minutes } min
appearance-away-unavailable = Questa sessione non può sapere quando sei via: serve GNOME, un vero server X o un compositor Wayland con notifiche di inattività.
appearance-battery-label = Pausa a batteria
appearance-battery-hint = I personaggi restano fermi mentre il computer va a batteria.
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Scene
scene-scenes-hint = Salva ciò che è sullo schermo con un nome e tornaci qui, dalla palette dei comandi o con «Next scene» nell’area di notifica.
scene-scene-name-hint = Nome della scena
scene-scene-save-new = Salva come scena
scene-scene-load-tooltip = Passa a questa scena: Annulla torna indietro
scene-scene-save-over = Salva ciò che è sullo schermo come { $name }
scene-scene-delete = Elimina scena
scene-scene-delete-confirm = Eliminare?
scene-scene-delete-cancel = Tienila
scene-scene-save-failed = Non salvata: { $error }
toast-scene-loaded = Scena: { $name }
toast-scene-failed = Scena non caricata: { $error }
toast-no-scenes = Ancora nessuna scena salvata: salvane una nella scheda Scena.
palette-load-scene = Passa alla scena: { $name }

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Impossibile registrare le scorciatoie globali (tipico di una sessione Wayland nativa). Il menu nella tray e il pulsante ⚙ continuano a funzionare.
warning-hot-reload-disconnected = Il processo di ricarica a caldo si è fermato inaspettatamente; le modifiche alla configurazione in corso si applicheranno solo dopo un riavvio.
# Machine-translated, pending native review.
warning-config-unreadable = Non è stato possibile leggere il file di configurazione, quindi è stata caricata la scena predefinita. L'originale è conservato accanto come config.toml.bak-corrupt.
action-toggle-perf-overlay = Attiva/disattiva overlay prestazioni

# ── What's new (D.7) — placeholder pending native-speaker audit
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Novità della 1.4
whats-new-add-file = «Aggiungi file…», nella scheda Scena, aggiunge personaggi dal selettore di file del tuo desktop, senza trascinare nulla.
whats-new-palette = Ctrl+K ora esegue qualsiasi azione e ne mostra la scorciatoia accanto.
whats-new-screen-readers = Gli screen reader come Orca ora possono leggere e usare il pannello delle impostazioni.
onboarding-keybindings = Fai clic su una scorciatoia per rimuoverla; premi una combinazione per registrarne una nuova.
onboarding-perf-overlay = Premi Ctrl+Shift+` per aprire l’overlay prestazioni in tempo reale.
appearance-reset-onboarding = Ripristina i suggerimenti iniziali

scene-empty-action-browse-presets = Sfoglia i preset
library-empty-action-copy-path = Copia il percorso negli appunti

appearance-reset-onboarding-hint = Ripristina i suggerimenti chiusi e il pannello «Novità».

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Permesso per le scorciatoie negato — verranno usate le scorciatoie X11. Riprova dalla scheda Scorciatoie.
portal-denied-native-toast = Permesso per le scorciatoie negato — il menu nella tray e le scorciatoie del compositor continuano a funzionare.

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
