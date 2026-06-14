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

scene-empty-headline = Scena vuota
scene-empty-hint = Trascina un PNG / GIF / WebP / MP4 sull'overlay — o prova un preset qui sotto.
scene-drop-hint = Trascina un PNG / GIF / WebP sull'overlay per aggiungere un'entità.
scene-presets-header = Preset
scene-preset-append = Aggiungi
scene-preset-replace = Sostituisci
scene-preset-replace-tooltip = Cancella la scena attuale prima di aggiungere

monitor-section-header = Monitor
monitor-mode-label = Distribuzione
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Estendi su tutti i monitor
monitor-mode-single = Monitor singolo
scene-window-awareness = Atterra sulle finestre (X11)
scene-window-awareness-tooltip = I personaggi con fisica attiva atterrano e camminano sul bordo superiore delle finestre aperte. Solo sessioni X11 — Wayland non espone le posizioni delle finestre, quindi lì non ha effetto.
monitor-pin-label = Fissa al monitor
monitor-pin-auto = Auto (segue la posizione)
monitor-pinned-toast = Entità fissata a { $name }
monitor-pin-cleared-toast = L'entità segue ora la sua posizione
monitor-no-monitors-detected = Nessun monitor rilevato

appearance-theme-header = Tema
appearance-theme-label = Tema
appearance-language-header = Lingua
theme-dark = Scuro
theme-light = Chiaro
theme-dark-hc = Scuro · Contrasto elevato
theme-light-hc = Chiaro · Contrasto elevato

onboarding-tabs = Le impostazioni sono divise su tre schede — Ispettore, Scena, Aspetto.
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

palette-search-placeholder = Cerca temi / preset…
palette-close-hint = Esc per chiudere · Ctrl+K per alternare
palette-switch-theme = Passa al tema { $theme }
palette-apply-preset = Applica preset: { $preset }

settings-tab-library = Libreria

# Asset library tab
library-empty-headline = Nessun asset indicizzato
library-empty-hint = Trascina file in ~/.local/share/animaEngine/assets/ o imposta ANIMA_ASSETS_DIR.
library-no-asset-root = Nessuna directory di asset trovata. Creane una in ~/.local/share/animaEngine/assets/
library-search-placeholder = Cerca asset…
library-add-to-scene = Aggiungi alla scena
library-sort-recent = Recenti
library-sort-name = Nome
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
keybindings-help = Le scorciatoie personalizzate vengono salvate in config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Attiva/disattiva modalità modifica
action-hide-overlay = Nascondi / mostra l’overlay
action-pause-all = Metti in pausa tutte le animazioni
action-quit-with-save = Esci (salvando la configurazione)
action-save-now = Salva subito la configurazione
action-open-command-palette = Palette dei comandi
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
appearance-accesskit-hint = Alimenta gli screen reader AT-SPI (Orca ecc.). Lascialo attivo, a meno che tu non voglia ridurre le risorse o il tuo desktop non abbia un bus AT-SPI. Nota: il testo digitato nei pannelli appare anche sul bus AT-SPI, dove ogni processo del tuo utente può leggerlo.
appearance-reduced-motion-label = Riduci il movimento
appearance-reduced-motion-hint = Salta le transizioni dell’interfaccia (scorrimento del pannello, dissolvenze, comparsa della palette) e ferma l’oscillazione decorativa. Le animazioni che comunicano uno stato restano attive.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Impossibile registrare le scorciatoie globali (tipico di una sessione Wayland nativa). Il menu nella tray e il pulsante ⚙ continuano a funzionare.
warning-hot-reload-disconnected = Il processo di ricarica a caldo si è fermato inaspettatamente; le modifiche alla configurazione in corso si applicheranno solo dopo un riavvio.
action-toggle-perf-overlay = Attiva/disattiva overlay prestazioni

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Novità della 0.4
whats-new-keybindings = Scorciatoie da tastiera riassegnabili — apri la nuova scheda Scorciatoie.
whats-new-collapse-state = Le sezioni dell’Inspector ricordano il loro stato aperto/chiuso tra le sessioni.
whats-new-error-banners = Le superfici di errore (prima silenziose) ora mostrano toast o banner — le vedrai.
whats-new-accessibility-toggle = AccessKit si può disattivare da Aspetto → Accessibilità.
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
shimeji-no-library-toast = Nessuna cartella libreria — crea prima ~/.local/share/animaEngine/assets/.
crash-report-found-toast = La sessione precedente si è chiusa in modo imprevisto. Un rapporto è stato salvato in { $path } — allegalo a un issue su GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composto dal gruppo { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configurazione salvata
toast-save-failed = Salvataggio non riuscito: { $error }
toast-rejected = Rifiutato: { $reason }
toast-added = { $name } aggiunto
toast-load-failed = Caricamento non riuscito: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Tema: { $theme }
toast-preset-entry-failed = Impossibile aggiungere la voce del preset: { $error }
toast-preset-loaded = Preset caricato: { $name }
toast-duplicated = { $name } duplicato
toast-duplicate-failed = Duplicazione non riuscita: { $error }
toast-deleted = { $name } eliminato
toast-playback-resumed = Riproduzione ripresa
toast-playback-paused = Riproduzione in pausa
toast-wayland-no-library = La libreria degli asset non è ancora disponibile sul percorso Wayland
inspector-wander-box = Area di vagabondaggio
toast-perf-snapshot = Istantanea prestazioni: { $path }
toast-perf-snapshot-failed = Istantanea non riuscita: { $error }
