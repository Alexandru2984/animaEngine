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
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Sinus
easing-bounce-out = Bounce out
inspector-section-behavior = Gedrag
inspector-visible = Zichtbaar
inspector-gravity = Zwaartekracht
inspector-scale = Schaal
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

monitor-section-header = Monitors
monitor-mode-label = Verdeling
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Uitstrekken over alle monitors
monitor-mode-single = Enkele monitor
monitor-pin-label = Vastpinnen aan monitor
monitor-pin-auto = Auto (volgt positie)
monitor-pinned-toast = Entiteit vastgepind aan { $name }
monitor-pin-cleared-toast = Entiteit volgt nu zijn positie
monitor-no-monitors-detected = Geen monitors gedetecteerd

appearance-theme-header = Thema
appearance-theme-label = Thema
appearance-language-header = Taal
appearance-keyboard-header = Toetsenbord
appearance-keyboard-note = Alleen-lezen in 0.2.0 — opnieuw toewijzen komt in een latere release.
theme-dark = Donker
theme-light = Licht
theme-dark-hc = Donker · Hoog contrast
theme-light-hc = Licht · Hoog contrast

onboarding-tabs = Instellingen verspreid over drie tabbladen — Inspector, Scène, Weergave.
onboarding-quick-toggles = Tip: V wisselt zichtbaarheid, G wisselt zwaartekracht — zonder dit paneel te openen.
onboarding-theme = Thema's worden direct toegepast — geen herstart nodig.
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
