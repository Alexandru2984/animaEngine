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
behavior-script = Script
behavior-script-path-label = Cale
behavior-script-path-hint = Relativă la biblioteca ta de resurse — { $path }
behavior-script-params = Parametri
behavior-script-param-name = nume
behavior-script-add-param = Adaugă
behavior-script-remove-param = Elimină acest parametru
script-failed-toast = Scriptul de comportament { $script } a eșuat: { $error }

scene-empty-headline = Scenă goală
scene-empty-hint = Trage un fișier PNG / GIF / WebP / MP4 peste overlay — sau încearcă un preset mai jos.
scene-drop-hint = Trage un fișier PNG / GIF / WebP peste overlay pentru a adăuga o entitate.
# "Add file…" (1.4): machine-translated, pending native review.
scene-add-file = Adaugă fișier…
scene-add-file-tooltip = Alege imagini sau clipuri de adăugat ca personaje.
file-chooser-title = Adaugă personaje
file-chooser-filter = Imagini și clipuri
file-chooser-unavailable-toast = Selectorul de fișiere nu s-a putut deschide. Are nevoie de xdg-desktop-portal.
scene-presets-header = Preseturi
scene-groups-header = Grupuri
scene-preset-append = Adaugă
scene-preset-replace = Înlocuiește
scene-preset-replace-tooltip = Șterge scena curentă înainte să adauge

monitor-section-header = Monitoare
monitor-mode-label = Distribuție
monitor-mode-per-monitor = Pe fiecare monitor
monitor-mode-span = Întinde pe toate monitoarele
monitor-mode-span-unsupported = Pe acest backend o suprapunere nu poate acoperi mai multe monitoare — folosește „per monitor”.
monitor-mode-single = Un singur monitor
scene-window-awareness = Aterizează pe ferestre (X11)
scene-window-awareness-tooltip = Personajele cu fizică activă aterizează și merg pe marginea de sus a ferestrelor deschise. Doar pe sesiuni X11 — Wayland nu expune pozițiile ferestrelor, deci acolo nu are efect.
scene-window-awareness-unavailable = Indisponibil pe Wayland nativ — niciun protocol nu expune pozițiile ferestrelor.
monitor-pin-label = Pinează pe monitor
monitor-pin-auto = Auto (urmează poziția)
monitor-pinned-toast = Entitate pinată pe { $name }
monitor-pin-cleared-toast = Entitatea urmează acum poziția
monitor-no-monitors-detected = Niciun monitor detectat

appearance-theme-header = Temă
appearance-theme-label = Temă
appearance-language-header = Limbă
appearance-language-no-font = Nu există niciun font instalat pentru acest scris — instalează întâi un pachet Noto CJK.
theme-dark = Întunecat
theme-light = Luminos
theme-dark-hc = Întunecat · Contrast ridicat
theme-light-hc = Luminos · Contrast ridicat

onboarding-tabs = Setările stau în cinci taburi — Inspector, Scenă, Bibliotecă, Aspect, Scurtături.
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

# Palette placeholder (1.4): machine-translated, pending native review.
palette-search-placeholder = Scrie pentru a căuta acțiuni, teme și preseturi…
palette-switch-theme = Schimbă pe tema { $theme }

settings-tab-library = Bibliotecă

# Asset library tab
library-empty-headline = Niciun asset indexat
library-empty-hint = Trage fișiere în { $path } sau setează ANIMA_ASSETS_DIR.
library-no-asset-root = Niciun director de assets găsit. Creează unul la { $path }
library-search-placeholder = Caută assets…
library-add-to-scene = Adaugă în scenă
library-kind-image = Imagine
library-kind-animated = Animat
library-kind-video = Video
library-asset-added-toast = { $name } adăugat în scenă
library-asset-add-failed-toast = Nu am putut adăuga { $name }
library-count = { $n } assets indexate

# ── Tab Scurtături (D.1) ───────────────────────────────────────────
settings-tab-keybindings = Scurtături
keybindings-unbound = (nelegat)
keybindings-add = Adaugă
keybindings-recording = Apasă o combinație… (Esc pentru anulare)
keybindings-conflict = Conflict cu { $action }
keybindings-reset-all = Resetează tot la implicit
keybindings-reset-one = Resetează la implicit
keybindings-remove-chord = Elimină această combinație
keybindings-help = Scurtăturile personalizate se salvează în config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names. Kept in English unless
# the convention is certain; see R39 in docs/runtime-findings.md.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Etichete acțiuni (D.1.7) ──────────────────────────────────────────
action-toggle-edit-mode = Comută modul editare
action-hide-overlay = Ascunde / arată suprapunerea
action-pause-all = Oprește toate animațiile
action-quit-with-save = Ieșire (salvează configurația)
action-save-now = Salvează configurația acum
action-open-command-palette = Paleta de comenzi
# Undo (1.5): machine-translated, pending native review.
action-undo = Anulează
action-redo = Refă

# Groups (1.5): machine-translated, pending native review.
action-group-selected = Grupează selecția
action-ungroup-selected = Desface grupurile selecției
menu-group = Grupează
menu-ungroup = Desface grupul
group-default-name = Grupul { $number }
group-copy-name = { $name } (copie)
toast-grouped = Grupate ca { $name }
toast-ungrouped = Grupuri desfăcute: { $count }
toast-nothing-to-ungroup = Nimic din selecție nu e într-un grup
scene-groups-empty-hint = Selectează mai multe personaje și apasă Ctrl+G — sau dă click dreapta pe ele și alege Grupează.
scene-group-members = Personaje: { $count }
scene-group-select-tooltip = Selectează personajele din acest grup
scene-group-rename = Redenumește grupul
scene-group-show = Arată grupul
scene-group-hide = Ascunde grupul
scene-group-ungroup = Desface grupul — personajele rămân

# Arrange and snap (1.5): machine-translated, pending native review.
arrange-align-left = Aliniază marginile din stânga
arrange-align-center = Centrează pe o linie verticală
arrange-align-right = Aliniază marginile din dreapta
arrange-align-top = Aliniază marginile de sus
arrange-align-middle = Centrează pe o linie orizontală
arrange-align-bottom = Aliniază marginile de jos
arrange-distribute-horizontally = Distribuie egal pe orizontală
arrange-distribute-vertically = Distribuie egal pe verticală
scene-snap = Lipește la tragere
scene-snap-tooltip = Marginile și centrele se lipesc de ale ecranelor și ale altor personaje. Ține Alt apăsat în timpul tragerii ca să le plasezi liber.

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
# Screen readers (1.4): machine-translated, pending native review.
appearance-accesskit-hint = Permite cititoarelor de ecran (Orca și altele, prin AT-SPI) să citească și să folosească panourile. Dezactivat, un cititor de ecran vede o fereastră goală. Notă: cât timp rulează unul, textul tastat în panouri ajunge pe magistrala AT-SPI, unde îl poate citi orice proces care rulează ca utilizatorul tău.
# Machine-translated, pending native review.
appearance-accesskit-unsupported = Cititoarele de ecran nu sunt încă suportate pe acest sistem.
appearance-reduced-motion-label = Redu mișcarea
appearance-reduced-motion-hint = Sare peste tranzițiile UI (glisarea panoului, fade-uri, pop-ul paletei) și oprește săltatul decorativ. Animațiile care transmit stare rulează în continuare.
appearance-hover-startle-label = Tresărire la hover
appearance-hover-startle-hint = Mascotele se feresc de cursor când se apropie de ele. Urmărirea cursorului e doar pe X11, deci pe Wayland nativ reacționează doar în modul editare.
# Full-screen apps (1.5): machine-translated, pending native review.
appearance-fullscreen-label = Când o aplicație e pe tot ecranul
appearance-fullscreen-hint = Jocuri, videoclipuri și prezentări care umplu ecranul. Merge pe X11 și pe compozitoarele Wayland precum sway și Hyprland; pe GNOME și KDE cu Wayland, doar pentru aplicațiile care rulează prin XWayland.
fullscreen-hide = Ascunde personajele
fullscreen-pause = Pune-le pe pauză
fullscreen-ignore = Lasă-le să meargă
# Pausing when away (1.5): machine-translated, pending native review.
appearance-away-label = Pauză când nu sunt la calculator
appearance-away-hint = Fără mouse sau tastatură atâta timp, personajele stau pe loc; pornesc din nou la următoarea ta acțiune.
away-never = Niciodată
away-after-minutes = După { $minutes } min
appearance-away-unavailable = Sesiunea asta nu poate ști când nu ești la calculator: e nevoie de GNOME, un server X adevărat sau un compositor Wayland cu notificări de inactivitate.
appearance-battery-label = Pauză pe baterie
appearance-battery-hint = Personajele stau pe loc cât timp calculatorul merge pe baterie.
# Start at login (1.5): machine-translated, pending native review.
appearance-autostart-label = Pornește la autentificare
appearance-autostart-hint = Pornește animaEngine când intri în sesiunea desktop.
appearance-autostart-reason = Ca să pornească animaEngine când intri în sesiune.
appearance-autostart-failed = Nu s-a putut schimba: { $error }
# Named scenes (1.5): machine-translated, pending native review.
scene-scenes-header = Scene
scene-scenes-hint = Salvează ce e pe ecran sub un nume și revino la el aici, din paleta de comenzi sau cu „Next scene” din tray.
scene-scene-name-hint = Numele scenei
scene-scene-save-new = Salvează ca scenă
scene-scene-load-tooltip = Treci la scena asta — Anulează te întoarce
scene-scene-save-over = Salvează ce e pe ecran ca { $name }
scene-scene-delete = Șterge scena
scene-scene-delete-confirm = Ștergi?
scene-scene-delete-cancel = Păstreaz-o
scene-scene-save-failed = Nesalvată: { $error }
toast-scene-loaded = Scenă: { $name }
toast-scene-failed = Scena nu s-a încărcat: { $error }
toast-no-scenes = Încă nu ai scene salvate — salvează una în tabul Scenă.
palette-load-scene = Treci la scena: { $name }

# ── Avertismente persistente (D.5) ────────────────────────────────────
warning-global-hotkeys-unavailable = Scurtăturile globale nu s-au putut înregistra (tipic pe sesiune Wayland nativă). Meniul din tray și butonul ⚙ funcționează în continuare.
warning-hot-reload-disconnected = Procesul de reîncărcare la cald s-a oprit pe neașteptate; modificările pe config nu se vor aplica până la repornire.
# Machine-translated, pending native review.
warning-config-unreadable = Fișierul de configurare nu a putut fi citit, așa că s-a încărcat scena implicită. Originalul e păstrat alături ca config.toml.bak-corrupt.
action-toggle-perf-overlay = Comută suprapunerea de performanță

# ── Panou "What's new" (D.7) ──────────────────────────────────────────
# 1.4 highlights: machine-translated, pending native review.
whats-new-header = Noutăți în 1.4
whats-new-add-file = „Adaugă fișier…” din tab-ul Scenă adaugă personaje prin selectorul de fișiere al desktopului tău — fără să tragi nimic.
whats-new-palette = Ctrl+K rulează acum orice acțiune și îi arată scurtătura alături.
whats-new-screen-readers = Cititoarele de ecran precum Orca pot acum citi și folosi panoul de setări.

# ── Hint-uri onboarding noi (D.7) ─────────────────────────────────────
onboarding-keybindings = Apasă × pe o combinație ca s-o elimini; apasă o combinație nouă ca s-o înregistrezi.
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
shimeji-no-library-toast = Nu există rădăcină de bibliotecă — creează întâi { $path }.
crash-report-found-toast = Sesiunea anterioară s-a închis neașteptat. Raportul a fost salvat la { $path } — atașează-l unui issue pe GitHub.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Compus de grupul { $group }: { $transform }
# Multiple selection (1.5): machine-translated, pending native review.
inspector-multi-selected = { $count } selectate. Panoul acesta editează { $name }; tragerea, scurtăturile și meniul de clic dreapta se aplică tuturor.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Configurație salvată
# Hot-reload toasts: machine-translated, pending native review.
toast-config-reloaded = Configurație reîncărcată de pe disc
toast-config-reload-failed = Configurația nu a fost reîncărcată — fișierul e invalid sau încă se scrie. Scena curentă rămâne.
toast-config-reload-discarded = Configurația nu a fost reîncărcată — ai editat scena cât se încărca.
toast-save-failed = Salvarea a eșuat: { $error }
toast-rejected = Respins: { $reason }
toast-added = Adăugat { $name }
toast-load-failed = Încărcarea a eșuat: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Temă: { $theme }
toast-preset-entry-failed = Nu s-a putut adăuga intrarea din preset: { $error }
toast-preset-loaded = Preset încărcat: { $name }
toast-duplicated = Duplicat { $name }
# Multiple selection (1.5): machine-translated, pending native review.
toast-duplicated-many = Personaje duplicate: { $count }
toast-duplicate-failed = Duplicarea a eșuat: { $error }
toast-deleted = Șters { $name }
# Multiple selection (1.5): machine-translated, pending native review.
toast-deleted-many = Personaje șterse: { $count }
toast-playback-resumed = Redare reluată
toast-playback-paused = Redare pe pauză
# Undo (1.5): machine-translated, pending native review.
toast-undone = Anulat
toast-redone = Refăcut
toast-nothing-to-undo = Nimic de anulat
toast-nothing-to-redo = Nimic de refăcut
inspector-wander-box = Cutie de hoinăreală
toast-perf-snapshot = Snapshot de performanță: { $path }
toast-perf-snapshot-failed = Snapshot-ul a eșuat: { $error }
