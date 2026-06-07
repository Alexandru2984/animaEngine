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
monitor-pin-label = Przypnij do monitora
monitor-pin-auto = Auto (śledź pozycję)
monitor-pinned-toast = Element przypięty do { $name }
monitor-pin-cleared-toast = Element teraz śledzi swoją pozycję
monitor-no-monitors-detected = Nie wykryto żadnych monitorów

appearance-theme-header = Motyw
appearance-theme-label = Motyw
appearance-language-header = Język
appearance-keyboard-header = Klawiatura
appearance-keyboard-note = Tylko do odczytu w 0.2.0 — zmiana skrótów pojawi się w kolejnym wydaniu.
theme-dark = Ciemny
theme-light = Jasny
theme-dark-hc = Ciemny · Wysoki kontrast
theme-light-hc = Jasny · Wysoki kontrast

onboarding-tabs = Ustawienia rozdzielono na trzy zakładki — Inspektor, Scena, Wygląd.
onboarding-quick-toggles = Wskazówka: V przełącza widoczność, G grawitację — bez otwierania tego panelu.
onboarding-theme = Motywy stosują się natychmiast — bez restartu.
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
appearance-accesskit-hint = Powers AT-SPI screen readers (Orca etc.). Leave on unless you want a tighter footprint or your desktop doesn't run an AT-SPI bus.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Global hotkeys couldn't register (typical on a native Wayland session). The tray menu and the ⚙ button still work.
warning-hot-reload-disconnected = The hot-reload worker stopped unexpectedly; in-flight config edits won't apply until you restart the app.
action-toggle-perf-overlay = Toggle perf overlay
