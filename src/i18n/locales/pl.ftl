# Polski — tłumaczenie bazowe. Wymaga przeglądu native speakera.

app-name = animaEngine

settings-tab-inspector = Inspektor
settings-tab-scene = Scena
settings-tab-appearance = Wygląd
entity-count-zero = Brak elementów
entity-count-singular = { $n } element
entity-count-plural = { $n } elementów

inspector-section-position = Pozycja
inspector-section-appearance = Wygląd
inspector-section-animation = Animacja
animation-easing-label = Easing
easing-linear = Liniowe
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Sinus
easing-bounce-out = Bounce out
inspector-section-behavior = Zachowanie
inspector-visible = Widoczny
inspector-gravity = Grawitacja
inspector-scale = Skala
inspector-behavior-speed = Speed
inspector-behavior-comfort = Comfort distance
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Period
inspector-double-click-reset-hint = Double-click to reset to default.
inspector-opacity = Krycie
inspector-fps = FPS
inspector-playing = Odtwarzanie
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nic nie wybrano
inspector-nothing-selected-hint = Kliknij element w zakładce Scena lub naciśnij Tab, aby je przejrzeć.

behavior-idle = Bezczynny
behavior-walk = Chodzi
behavior-follow = Podąża za kursorem
behavior-wander = Ograniczona wędrówka
behavior-bounce = Odbicie
behavior-bounce-axis = Oś
behavior-bounce-horizontal = Poziomo
behavior-bounce-vertical = Pionowo
behavior-bounce-both = Oba (okrąg)

scene-empty-headline = Pusta scena
scene-empty-hint = Przeciągnij plik PNG / GIF / WebP / MP4 na nakładkę — lub wypróbuj preset poniżej.
scene-drop-hint = Przeciągnij plik PNG / GIF / WebP na nakładkę, aby dodać element.
scene-presets-header = Presety
scene-preset-append = Dodaj
scene-preset-replace = Zastąp
scene-preset-replace-tooltip = Wyczyści obecną scenę przed dodaniem

monitor-section-header = Monitory
monitor-mode-label = Dystrybucja
monitor-mode-per-monitor = Na każdym monitorze
monitor-mode-span = Rozciągnij na wszystkich monitorach
monitor-mode-single = Pojedynczy monitor
scene-window-awareness = Land on windows (X11)
scene-window-awareness-tooltip = Physics-enabled characters land on and walk along the top edges of your open windows. X11 sessions only — Wayland offers no window positions, so this does nothing there.
monitor-pin-label = Przypnij do monitora
monitor-pin-auto = Auto (śledź pozycję)
monitor-pinned-toast = Element przypięty do { $name }
monitor-pin-cleared-toast = Element teraz śledzi swoją pozycję
monitor-no-monitors-detected = Nie wykryto żadnych monitorów

appearance-theme-header = Motyw
appearance-theme-label = Motyw
appearance-language-header = Język
theme-dark = Ciemny
theme-light = Jasny
theme-dark-hc = Ciemny · Wysoki kontrast
theme-light-hc = Jasny · Wysoki kontrast

onboarding-tabs = Ustawienia rozdzielono na trzy zakładki — Inspektor, Scena, Wygląd.
onboarding-quick-toggles = Wskazówka: V przełącza widoczność, G grawitację — bez otwierania tego panelu.
onboarding-theme = Motywy stosują się natychmiast — bez restartu.
onboarding-coach-step1 = Welcome! Your characters live on the desktop. Click the gear button in the top-right corner to enter edit mode.
onboarding-coach-step2 = Drop a PNG, GIF, WebP or MP4 anywhere on the screen to add it as a character. The side panel edits everything you select.
onboarding-coach-step3 = Ctrl+K opens the command palette. Ctrl+Shift+A toggles edit mode from anywhere, Ctrl+Shift+H hides the overlay.
onboarding-coach-next = Next
onboarding-coach-skip = Skip tour
onboarding-coach-done = Got it
palette-replace-row = Replace scene with: { $preset }
palette-append-row = Append preset: { $preset }
palette-footer-hint = Esc to close · Ctrl+K to toggle · ↑↓ + Enter to pick
onboarding-dismiss = Zamknij

menu-duplicate = Duplikuj
menu-reset-transform = Resetuj transformację
menu-toggle-gravity = Przełącz grawitację
menu-bring-forward = Przenieś na wierzch
menu-send-backward = Wyślij na spód
menu-delete = Usuń

toggle-enter-edit = Wejdź w tryb edycji
toggle-exit-edit = Wyjdź z trybu edycji

palette-search-placeholder = Wpisz, aby wyszukać motywy / presety…
palette-close-hint = Esc zamyka · Ctrl+K przełącza
palette-switch-theme = Przełącz na motyw { $theme }
palette-apply-preset = Zastosuj preset: { $preset }

settings-tab-library = Biblioteka

# Asset library tab
library-empty-headline = Brak zindeksowanych zasobów
library-empty-hint = Wrzuć pliki do ~/.local/share/animaEngine/assets/ lub ustaw ANIMA_ASSETS_DIR.
library-no-asset-root = Nie znaleziono katalogu zasobów. Utwórz go w ~/.local/share/animaEngine/assets/
library-search-placeholder = Szukaj zasobów…
library-add-to-scene = Dodaj do sceny
library-sort-recent = Ostatnie
library-sort-name = Nazwa
library-kind-image = Obraz
library-kind-animated = Animowane
library-kind-video = Wideo
library-asset-added-toast = Dodano { $name } do sceny
library-asset-add-failed-toast = Nie udało się dodać { $name }
library-count = Zindeksowano { $n } zasobów

# ── Keybindings tab (D.1) — placeholder pending D.4 native-speaker audit
settings-tab-keybindings = Keybindings
keybindings-unbound = (unbound)
keybindings-add = Add
keybindings-recording = Press a chord… (Esc to cancel)
keybindings-conflict = Conflicts with { $action }
keybindings-reset-all = Reset all to defaults
keybindings-help = Custom shortcuts persist in config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Toggle edit mode
action-hide-overlay = Hide / show overlay
action-pause-all = Pause all animations
action-quit-with-save = Quit (save config)
action-save-now = Save config now
action-open-command-palette = Command palette
action-cycle-entity = Cycle to next entity
action-delete-selected = Delete selected entity
action-nudge-up = Nudge selection up
action-nudge-down = Nudge selection down
action-nudge-left = Nudge selection left
action-nudge-right = Nudge selection right
action-center-on-screen = Center selection on screen
action-toggle-visible = Toggle visibility
action-toggle-gravity = Toggle gravity
action-toggle-playback = Toggle play/pause
action-duplicate-selected = Duplicate selection
action-reset-transform = Reset scale / opacity
action-bring-forward = Bring selection forward
action-send-backward = Send selection backward
action-fps-up = Increase FPS
action-fps-down = Decrease FPS
action-opacity-up = Increase opacity
action-opacity-down = Decrease opacity
action-cycle-monitor = Cycle entity monitor pin
action-show-entity-info = Show entity info
action-show-help = Show keyboard help

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Accessibility
appearance-accesskit-label = Generate AccessKit tree updates
appearance-accesskit-hint = Powers AT-SPI screen readers (Orca etc.). Leave on unless you want a tighter footprint or your desktop doesn't run an AT-SPI bus. Note: text you type in panels also appears on the AT-SPI bus, where any process running as your user can read it.
appearance-reduced-motion-label = Reduce motion
appearance-reduced-motion-hint = Skips UI transitions (panel slide, fades, palette pop) and stops decorative bouncing. Animations that convey state still play.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Global hotkeys couldn't register (typical on a native Wayland session). The tray menu and the ⚙ button still work.
warning-hot-reload-disconnected = The hot-reload worker stopped unexpectedly; in-flight config edits won't apply until you restart the app.
action-toggle-perf-overlay = Toggle perf overlay

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = What's new in 0.4
whats-new-keybindings = Rebindable keyboard shortcuts — open the new Keybindings tab.
whats-new-collapse-state = Inspector sections remember their open/closed state across sessions.
whats-new-error-banners = Failure surfaces (silent before) now toast or banner — you'll see them.
whats-new-accessibility-toggle = AccessKit can be turned off from Appearance → Accessibility.
onboarding-keybindings = Click any chord to remove it; press a key combo to record a new one.
onboarding-perf-overlay = Press Ctrl+Shift+` to open the live perf overlay.
appearance-reset-onboarding = Reset onboarding hints

scene-empty-action-browse-presets = Browse presets
library-empty-action-copy-path = Copy path to clipboard

appearance-reset-onboarding-hint = Brings back the dismissed progressive hints and the "What's new" panel.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Shortcut permission was declined — using X11 hotkeys instead. Retry from the Keybindings tab.
portal-denied-native-toast = Shortcut permission was declined — the tray menu and compositor bindings still work.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Global shortcuts via:
keybindings-backend-tooltip = Which mechanism delivers the three global chords (toggle edit, hide, pause) while other apps have focus. Resolved at startup; in-app shortcuts are unaffected.
keybindings-portal-restart-hint = Trigger changes apply at the next launch (the desktop remembers your approval).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } disconnected — { $n } pinned entities now follow their position.
monitor-plugged-toast = Monitor { $name } connected.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Import Shimeji pack
library-import-shimeji-hint = Drop a pack folder onto the overlay, or paste its path here. Sprites are copied into the library.
library-import-shimeji-button = Import
shimeji-imported-toast = Imported { $name } ({ $n } parts skipped — see log)
shimeji-import-failed-toast = Import failed: { $reason }
shimeji-no-library-toast = No asset library root — create ~/.local/share/animaEngine/assets/ first.
crash-report-found-toast = The previous session crashed. A report was saved at { $path } — please attach it to a GitHub issue.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composed by group { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Config saved
toast-save-failed = Save failed: { $error }
toast-rejected = Rejected: { $reason }
toast-added = Added { $name }
toast-load-failed = Load failed: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Theme: { $theme }
toast-preset-entry-failed = Couldn’t add preset entry: { $error }
toast-preset-loaded = Loaded preset: { $name }
toast-duplicated = Duplicated { $name }
toast-duplicate-failed = Duplicate failed: { $error }
toast-deleted = Deleted { $name }
toast-playback-resumed = Playback resumed
toast-playback-paused = Playback paused
toast-wayland-no-library = Asset library not wired on the Wayland path yet
inspector-wander-box = Wander box
toast-perf-snapshot = Perf snapshot: { $path }
toast-perf-snapshot-failed = Snapshot failed: { $error }
