# Română — traducere completă întreținută de mentenarii proiectului.

app-name = animaEngine

settings-tab-inspector = Inspector
settings-tab-scene = Scenă
settings-tab-appearance = Aspect
entity-count-zero = Nicio entitate
entity-count-singular = { $n } entitate
entity-count-plural = { $n } entități

inspector-section-position = Poziție
inspector-section-appearance = Aspect
inspector-section-animation = Animație
animation-easing-label = Easing
easing-linear = Linear
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Sinus
easing-bounce-out = Bounce out
inspector-section-behavior = Comportament
inspector-visible = Vizibil
inspector-gravity = Gravitație
inspector-scale = Scară
inspector-behavior-speed = Viteză
inspector-behavior-comfort = Distanță de confort
inspector-behavior-amplitude = Amplitudine
inspector-behavior-period = Perioadă
inspector-double-click-reset-hint = Dublu-click pentru a reveni la valoarea implicită.
inspector-opacity = Opacitate
inspector-fps = FPS
inspector-playing = Redă
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nimic selectat
inspector-nothing-selected-hint = Apasă pe o entitate din tab-ul Scenă, sau apasă Tab pentru a parcurge entitățile.

behavior-idle = Pe loc
behavior-walk = Plimbare
behavior-follow = Urmărește cursorul
behavior-wander = Rătăcire delimitată
behavior-bounce = Săritură
behavior-bounce-axis = Axă
behavior-bounce-horizontal = Orizontală
behavior-bounce-vertical = Verticală
behavior-bounce-both = Ambele (cerc)

scene-empty-headline = Scenă goală
scene-empty-hint = Trage un fișier PNG / GIF / WebP / MP4 peste overlay — sau încearcă un preset mai jos.
scene-drop-hint = Trage un fișier PNG / GIF / WebP peste overlay pentru a adăuga o entitate.
scene-presets-header = Preseturi
scene-preset-append = Adaugă
scene-preset-replace = Înlocuiește
scene-preset-replace-tooltip = Șterge scena curentă înainte să adauge

monitor-section-header = Monitoare
monitor-mode-label = Distribuție
monitor-mode-per-monitor = Pe fiecare monitor
monitor-mode-span = Întinde pe toate monitoarele
monitor-mode-single = Un singur monitor
scene-window-awareness = Aterizează pe ferestre (X11)
scene-window-awareness-tooltip = Personajele cu fizică activă aterizează și merg pe marginea de sus a ferestrelor deschise. Doar pe sesiuni X11 — Wayland nu expune pozițiile ferestrelor, deci acolo nu are efect.
monitor-pin-label = Pinează pe monitor
monitor-pin-auto = Auto (urmează poziția)
monitor-pinned-toast = Entitate pinată pe { $name }
monitor-pin-cleared-toast = Entitatea urmează acum poziția
monitor-no-monitors-detected = Niciun monitor detectat

appearance-theme-header = Temă
appearance-theme-label = Temă
appearance-language-header = Limbă
theme-dark = Întunecat
theme-light = Luminos
theme-dark-hc = Întunecat · Contrast ridicat
theme-light-hc = Luminos · Contrast ridicat

onboarding-tabs = Setările sunt împărțite pe trei tab-uri — Inspector, Scenă, Aspect.
onboarding-quick-toggles = Sfat: V comută vizibilitatea, G comută gravitația — fără să deschizi acest panou.
onboarding-theme = Temele se aplică instant — niciun restart necesar.
onboarding-coach-step1 = Bun venit! Personajele trăiesc pe desktop. Apasă butonul cu rotiță din colțul din dreapta-sus ca să intri în modul de editare.
onboarding-coach-step2 = Trage un PNG, GIF, WebP sau MP4 oriunde pe ecran ca să-l adaugi ca personaj. Panoul lateral editează tot ce selectezi.
onboarding-coach-step3 = Ctrl+K deschide paleta de comenzi. Ctrl+Shift+A comută modul de editare de oriunde, Ctrl+Shift+H ascunde overlay-ul.
onboarding-coach-next = Înainte
onboarding-coach-skip = Sari peste tur
onboarding-coach-done = Am înțeles
palette-replace-row = Înlocuiește scena cu: { $preset }
palette-append-row = Adaugă preset-ul: { $preset }
palette-footer-hint = Esc închide · Ctrl+K comută · ↑↓ + Enter alege
onboarding-dismiss = Închide

menu-duplicate = Duplică
menu-reset-transform = Resetează transformul
menu-toggle-gravity = Comută gravitația
menu-bring-forward = Adu în față
menu-send-backward = Trimite în spate
menu-delete = Șterge

toggle-enter-edit = Intră în mod editare
toggle-exit-edit = Ieși din mod editare

palette-search-placeholder = Scrie pentru a căuta teme / preseturi…
palette-close-hint = Esc pentru a închide · Ctrl+K pentru a comuta
palette-switch-theme = Schimbă pe tema { $theme }
palette-apply-preset = Aplică presetul: { $preset }

settings-tab-library = Bibliotecă

# Asset library tab
library-empty-headline = Niciun asset indexat
library-empty-hint = Trage fișiere în ~/.local/share/animaEngine/assets/ sau setează ANIMA_ASSETS_DIR.
library-no-asset-root = Niciun director de assets găsit. Creează unul la ~/.local/share/animaEngine/assets/
library-search-placeholder = Caută assets…
library-add-to-scene = Adaugă în scenă
library-sort-recent = Recente
library-sort-name = Nume
library-kind-image = Imagine
library-kind-animated = Animat
library-kind-video = Video
library-asset-added-toast = { $name } adăugat în scenă
library-asset-add-failed-toast = Nu am putut adăuga { $name }
library-count = { $n } assets indexate

# ── Tab Comenzi taste (D.1) ───────────────────────────────────────────
settings-tab-keybindings = Comenzi taste
keybindings-unbound = (nelegat)
keybindings-add = Adaugă
keybindings-recording = Apasă o combinație… (Esc pentru anulare)
keybindings-conflict = Conflict cu { $action }
keybindings-reset-all = Resetează tot la implicit
keybindings-help = Comenzile personalizate se salvează în config.toml

# ── Etichete acțiuni (D.1.7) ──────────────────────────────────────────
action-toggle-edit-mode = Comută modul editare
action-hide-overlay = Ascunde / arată suprapunerea
action-pause-all = Oprește toate animațiile
action-quit-with-save = Ieșire (salvează configurația)
action-save-now = Salvează configurația acum
action-open-command-palette = Paleta de comenzi
action-cycle-entity = Treci la următoarea entitate
action-delete-selected = Șterge entitatea selectată
action-nudge-up = Mută selecția în sus
action-nudge-down = Mută selecția în jos
action-nudge-left = Mută selecția la stânga
action-nudge-right = Mută selecția la dreapta
action-center-on-screen = Centrează selecția pe ecran
action-toggle-visible = Comută vizibilitatea
action-toggle-gravity = Comută gravitația
action-toggle-playback = Comută redare / pauză
action-duplicate-selected = Duplică selecția
action-reset-transform = Resetează scară / opacitate
action-bring-forward = Adu selecția în față
action-send-backward = Trimite selecția în spate
action-fps-up = Crește FPS-ul
action-fps-down = Scade FPS-ul
action-opacity-up = Crește opacitatea
action-opacity-down = Scade opacitatea
action-cycle-monitor = Schimbă monitorul entității
action-show-entity-info = Arată detaliile entității
action-show-help = Arată ajutor pentru taste

# ── Secțiune accesibilitate în tab-ul Aspect (D.3) ────────────────────
appearance-accessibility-header = Accesibilitate
appearance-accesskit-label = Generează actualizări AccessKit
appearance-accesskit-hint = Alimentează cititoarele de ecran AT-SPI (Orca etc.). Lasă activ dacă nu vrei să reduci consumul sau dacă desktop-ul tău nu rulează un bus AT-SPI. Notă: textul tastat în panouri apare și pe magistrala AT-SPI, unde orice proces care rulează ca utilizatorul tău îl poate citi.
appearance-reduced-motion-label = Redu mișcarea
appearance-reduced-motion-hint = Sare peste tranzițiile UI (glisarea panoului, fade-uri, pop-ul paletei) și oprește săltatul decorativ. Animațiile care transmit stare rulează în continuare.

# ── Avertismente persistente (D.5) ────────────────────────────────────
warning-global-hotkeys-unavailable = Comenzile globale nu s-au putut înregistra (tipic pe sesiune Wayland nativă). Meniul din tray și butonul ⚙ funcționează în continuare.
warning-hot-reload-disconnected = Procesul de reîncărcare la cald s-a oprit pe neașteptate; modificările pe config nu se vor aplica până la repornire.
action-toggle-perf-overlay = Comută suprapunerea de performanță

# ── Panou "What's new" (D.7) ──────────────────────────────────────────
whats-new-header = Noutăți în 0.4
whats-new-keybindings = Comenzi taste rebindable — deschide noul tab Comenzi taste.
whats-new-collapse-state = Secțiunile Inspector își amintesc starea deschisă/închisă între sesiuni.
whats-new-error-banners = Erorile (tăcute înainte) apar acum ca toast sau banner — le vezi.
whats-new-accessibility-toggle = AccessKit poate fi dezactivat din Aspect → Accesibilitate.

# ── Hint-uri onboarding noi (D.7) ─────────────────────────────────────
onboarding-keybindings = Apasă × pe un chord ca să-l elimini; apasă o combinație ca să înregistrezi una nouă.
onboarding-perf-overlay = Apasă Ctrl+Shift+` ca să deschizi overlay-ul live de performanță.
appearance-reset-onboarding = Resetează hint-urile de bun venit

# ── Acțiuni stări goale (D.8) ─────────────────────────────────────────
scene-empty-action-browse-presets = Răsfoiește preseturi
library-empty-action-copy-path = Copiază calea

# ── Tooltips (D.9) ────────────────────────────────────────────────────
appearance-reset-onboarding-hint = Reactivează hint-urile descărcate și panoul "Noutăți".

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Permisiunea pentru scurtături a fost refuzată — folosim scurtăturile X11. Reîncearcă din tabul Scurtături.
portal-denied-native-toast = Permisiunea pentru scurtături a fost refuzată — meniul din tray și bindurile compositorului funcționează în continuare.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Scurtături globale prin:
keybindings-backend-tooltip = Mecanismul care livrează cele trei scurtături globale (edit, ascundere, pauză) cât timp alte aplicații au focus. Stabilit la pornire; scurtăturile din aplicație nu sunt afectate.
keybindings-portal-restart-hint = Schimbările de combinații se aplică la următoarea pornire (desktop-ul ține minte aprobarea).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitorul { $name } a fost deconectat — { $n } entități fixate își urmează acum poziția.
monitor-plugged-toast = Monitorul { $name } a fost conectat.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Importă pachet Shimeji
library-import-shimeji-hint = Trage un folder de pachet peste overlay sau lipește calea aici. Sprite-urile se copiază în bibliotecă.
library-import-shimeji-button = Importă
shimeji-imported-toast = Importat { $name } ({ $n } părți sărite — vezi log-ul)
shimeji-import-failed-toast = Import eșuat: { $reason }
shimeji-no-library-toast = Nu există rădăcină de bibliotecă — creează întâi ~/.local/share/animaEngine/assets/.
crash-report-found-toast = Sesiunea anterioară s-a închis neașteptat. Raportul a fost salvat la { $path } — atașează-l unui issue pe GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Compus de grupul { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configurație salvată
toast-save-failed = Salvarea a eșuat: { $error }
toast-rejected = Respins: { $reason }
toast-added = Adăugat { $name }
toast-load-failed = Încărcarea a eșuat: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Temă: { $theme }
toast-preset-entry-failed = Nu s-a putut adăuga intrarea din preset: { $error }
toast-preset-loaded = Preset încărcat: { $name }
toast-duplicated = Duplicat { $name }
toast-duplicate-failed = Duplicarea a eșuat: { $error }
toast-deleted = Șters { $name }
toast-playback-resumed = Redare reluată
toast-playback-paused = Redare pe pauză
toast-wayland-no-library = Biblioteca de asset-uri nu e încă disponibilă pe calea Wayland
inspector-wander-box = Cutie de hoinăreală
toast-perf-snapshot = Snapshot de performanță: { $path }
toast-perf-snapshot-failed = Snapshot-ul a eșuat: { $error }
