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
animation-easing-label = Wygładzanie
easing-linear = Liniowe
easing-ease-in-quad = Płynne wejście
easing-ease-out-quad = Płynne wyjście
easing-ease-in-out-quad = Płynne wejście/wyjście
easing-sine = Sinus
easing-bounce-out = Odbicie
inspector-section-behavior = Zachowanie
inspector-visible = Widoczny
inspector-gravity = Grawitacja
inspector-scale = Skala
inspector-behavior-speed = Prędkość
inspector-behavior-comfort = Dystans komfortu
inspector-behavior-amplitude = Amplituda
inspector-behavior-period = Okres
inspector-double-click-reset-hint = Kliknij dwukrotnie, aby przywrócić wartość domyślną.
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
scene-window-awareness = Ląduj na oknach (X11)
scene-window-awareness-tooltip = Postacie z włączoną fizyką lądują na górnych krawędziach otwartych okien i chodzą po nich. Tylko sesje X11 — Wayland nie udostępnia pozycji okien, więc tam to nic nie robi.
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
onboarding-coach-step1 = Witaj! Twoje postacie żyją na pulpicie. Kliknij koło zębate w prawym górnym rogu, aby wejść w tryb edycji.
onboarding-coach-step2 = Upuść PNG, GIF, WebP lub MP4 w dowolnym miejscu ekranu, aby dodać go jako postać. Panel boczny edytuje wszystko, co zaznaczysz.
onboarding-coach-step3 = Ctrl+K otwiera paletę poleceń. Ctrl+Shift+A przełącza tryb edycji z dowolnego miejsca, Ctrl+Shift+H ukrywa nakładkę.
onboarding-coach-next = Dalej
onboarding-coach-skip = Pomiń przewodnik
onboarding-coach-done = Rozumiem
palette-replace-row = Zastąp scenę przez: { $preset }
palette-append-row = Dodaj preset: { $preset }
palette-footer-hint = Esc zamyka · Ctrl+K przełącza · ↑↓ + Enter wybiera
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
settings-tab-keybindings = Skróty
keybindings-unbound = (nieprzypisany)
keybindings-add = Dodaj
keybindings-recording = Naciśnij kombinację… (Esc anuluje)
keybindings-conflict = Konflikt z { $action }
keybindings-reset-all = Przywróć wszystkie wartości domyślne
keybindings-help = Własne skróty są zapisywane w config.toml

# ── Action labels (D.1.7) — placeholder pending D.4 native-speaker audit
action-toggle-edit-mode = Przełącz tryb edycji
action-hide-overlay = Ukryj / pokaż nakładkę
action-pause-all = Wstrzymaj wszystkie animacje
action-quit-with-save = Zakończ (zapisz konfigurację)
action-save-now = Zapisz konfigurację teraz
action-open-command-palette = Paleta poleceń
action-cycle-entity = Przejdź do następnej postaci
action-delete-selected = Usuń zaznaczoną postać
action-nudge-up = Przesuń zaznaczenie w górę
action-nudge-down = Przesuń zaznaczenie w dół
action-nudge-left = Przesuń zaznaczenie w lewo
action-nudge-right = Przesuń zaznaczenie w prawo
action-center-on-screen = Wyśrodkuj zaznaczenie na ekranie
action-toggle-visible = Przełącz widoczność
action-toggle-gravity = Przełącz grawitację
action-toggle-playback = Odtwarzaj/wstrzymaj
action-duplicate-selected = Duplikuj zaznaczenie
action-reset-transform = Zresetuj skalę / nieprzezroczystość
action-bring-forward = Przenieś zaznaczenie do przodu
action-send-backward = Przenieś zaznaczenie do tyłu
action-fps-up = Zwiększ FPS
action-fps-down = Zmniejsz FPS
action-opacity-up = Zwiększ nieprzezroczystość
action-opacity-down = Zmniejsz nieprzezroczystość
action-cycle-monitor = Przełącz przypięcie do monitora
action-show-entity-info = Pokaż informacje o postaci
action-show-help = Pokaż pomoc klawiatury

# ── Accessibility section (D.3) — placeholder pending D.4 native-speaker audit
appearance-accessibility-header = Dostępność
appearance-accesskit-label = Generuj aktualizacje drzewa AccessKit
appearance-accesskit-hint = Zasila czytniki ekranu AT-SPI (Orca itd.). Zostaw włączone, chyba że chcesz mniejsze zużycie zasobów albo twój pulpit nie ma magistrali AT-SPI. Uwaga: tekst wpisywany w panelach pojawia się też na magistrali AT-SPI, gdzie może go odczytać każdy proces twojego użytkownika.
appearance-reduced-motion-label = Ogranicz ruch
appearance-reduced-motion-hint = Pomija przejścia interfejsu (wysuwanie panelu, przenikanie, wyskakiwanie palety) i zatrzymuje dekoracyjne bujanie. Animacje przekazujące stan nadal działają.

# ── Warning banners (D.5) — placeholder pending native-speaker audit
warning-global-hotkeys-unavailable = Nie udało się zarejestrować globalnych skrótów (typowe dla natywnej sesji Wayland). Menu w zasobniku i przycisk ⚙ nadal działają.
warning-hot-reload-disconnected = Proces przeładowywania na gorąco zatrzymał się niespodziewanie; trwające zmiany konfiguracji zadziałają dopiero po restarcie aplikacji.
action-toggle-perf-overlay = Przełącz nakładkę wydajności

# ── What's new (D.7) — placeholder pending native-speaker audit
whats-new-header = Co nowego w 0.4
whats-new-keybindings = Skróty klawiszowe z możliwością zmiany — otwórz nową kartę Skróty.
whats-new-collapse-state = Sekcje Inspektora pamiętają stan otwarcia/zamknięcia między sesjami.
whats-new-error-banners = Miejsca błędów (wcześniej ciche) pokazują teraz toasty lub banery — zobaczysz je.
whats-new-accessibility-toggle = AccessKit można wyłączyć w Wygląd → Dostępność.
onboarding-keybindings = Kliknij skrót, aby go usunąć; naciśnij kombinację, aby nagrać nowy.
onboarding-perf-overlay = Naciśnij Ctrl+Shift+`, aby otworzyć nakładkę wydajności na żywo.
appearance-reset-onboarding = Przywróć wskazówki startowe

scene-empty-action-browse-presets = Przeglądaj presety
library-empty-action-copy-path = Skopiuj ścieżkę do schowka

appearance-reset-onboarding-hint = Przywraca zamknięte wskazówki i panel „Co nowego”.

# ── Portal shortcuts (T.3) ────────────────────────────────────────────
portal-denied-x11-fallback-toast = Odmówiono uprawnień do skrótów — w użyciu są skróty X11. Spróbuj ponownie z karty Skróty.
portal-denied-native-toast = Odmówiono uprawnień do skrótów — menu w zasobniku i skróty kompozytora nadal działają.

# ── Keybindings backend status (T.4) ─────────────────────────────────
keybindings-backend-label = Skróty globalne przez:
keybindings-backend-tooltip = Który mechanizm dostarcza trzy globalne skróty (edycja, ukrycie, pauza), gdy fokus mają inne aplikacje. Ustalane przy starcie; skróty wewnątrz aplikacji pozostają bez zmian.
keybindings-portal-restart-hint = Zmiany skrótów zadziałają od następnego uruchomienia (pulpit pamięta twoją zgodę).

# ── Monitor hotplug (T.9) ─────────────────────────────────────────────
monitor-unplugged-toast = Monitor { $name } odłączony — { $n } przypiętych postaci podąża teraz za swoją pozycją.
monitor-plugged-toast = Monitor { $name } podłączony.

# ── Shimeji import (U.4) ──────────────────────────────────────────────
library-import-shimeji-header = Importuj paczkę Shimeji
library-import-shimeji-hint = Przeciągnij folder paczki na nakładkę albo wklej tu jego ścieżkę. Sprite’y zostaną skopiowane do biblioteki.
library-import-shimeji-button = Importuj
shimeji-imported-toast = Zaimportowano { $name } (pominięto { $n } części — zobacz log)
shimeji-import-failed-toast = Import nie powiódł się: { $reason }
shimeji-no-library-toast = Brak folderu biblioteki — najpierw utwórz ~/.local/share/animaEngine/assets/.
crash-report-found-toast = Poprzednia sesja zakończyła się awarią. Raport zapisano w { $path } — dołącz go do zgłoszenia na GitHubie.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Złożone przez grupę { $group }: { $transform }

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Konfiguracja zapisana
toast-save-failed = Zapis nie powiódł się: { $error }
toast-rejected = Odrzucono: { $reason }
toast-added = Dodano { $name }
toast-load-failed = Wczytywanie nie powiodło się: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Motyw: { $theme }
toast-preset-entry-failed = Nie udało się dodać pozycji presetu: { $error }
toast-preset-loaded = Wczytano preset: { $name }
toast-duplicated = Zduplikowano { $name }
toast-duplicate-failed = Duplikowanie nie powiodło się: { $error }
toast-deleted = Usunięto { $name }
toast-playback-resumed = Odtwarzanie wznowione
toast-playback-paused = Odtwarzanie wstrzymane
toast-wayland-no-library = Biblioteka zasobów nie jest jeszcze dostępna na ścieżce Wayland
inspector-wander-box = Obszar wędrówki
toast-perf-snapshot = Migawka wydajności: { $path }
toast-perf-snapshot-failed = Migawka nie powiodła się: { $error }
