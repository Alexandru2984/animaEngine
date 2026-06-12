# Français — traduction de base. Relecture par locuteur natif à faire.

app-name = animaEngine

settings-tab-inspector = Inspecteur
settings-tab-scene = Scène
settings-tab-appearance = Apparence
entity-count-zero = Aucune entité
entity-count-singular = { $n } entité
entity-count-plural = { $n } entités

inspector-section-position = Position
inspector-section-appearance = Apparence
inspector-section-animation = Animation
animation-easing-label = Easing
easing-linear = Linéaire
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Sinus
easing-bounce-out = Bounce out
inspector-section-behavior = Comportement
inspector-visible = Visible
inspector-gravity = Gravité
inspector-scale = Échelle
inspector-behavior-speed = Speed
inspector-behavior-comfort = Comfort distance
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Period
inspector-double-click-reset-hint = Double-click to reset to default.
inspector-opacity = Opacité
inspector-fps = FPS
inspector-playing = Lecture
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Rien de sélectionné
inspector-nothing-selected-hint = Cliquez sur une entité dans l'onglet Scène, ou appuyez sur Tab pour les parcourir.

behavior-idle = Au repos
behavior-walk = Se promener
behavior-follow = Suivre le curseur
behavior-wander = Errance bornée
behavior-bounce = Rebond
behavior-bounce-axis = Axe
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Les deux (cercle)

scene-empty-headline = Scène vide
scene-empty-hint = Déposez un PNG / GIF / WebP / MP4 sur l'overlay — ou essayez un preset ci-dessous.
scene-drop-hint = Déposez un PNG / GIF / WebP sur l'overlay pour ajouter une entité.
scene-presets-header = Presets
scene-preset-append = Ajouter
scene-preset-replace = Remplacer
scene-preset-replace-tooltip = Efface la scène actuelle avant d'ajouter

monitor-section-header = Écrans
monitor-mode-label = Distribution
monitor-mode-per-monitor = Un par écran
monitor-mode-span = Étendre sur tous les écrans
monitor-mode-single = Un seul écran
scene-window-awareness = Land on windows (X11)
scene-window-awareness-tooltip = Physics-enabled characters land on and walk along the top edges of your open windows. X11 sessions only — Wayland offers no window positions, so this does nothing there.
monitor-pin-label = Épingler à l'écran
monitor-pin-auto = Auto (suit la position)
monitor-pinned-toast = Entité épinglée à { $name }
monitor-pin-cleared-toast = L'entité suit maintenant sa position
monitor-no-monitors-detected = Aucun écran détecté

appearance-theme-header = Thème
appearance-theme-label = Thème
appearance-language-header = Langue
theme-dark = Sombre
theme-light = Clair
theme-dark-hc = Sombre · Contraste élevé
theme-light-hc = Clair · Contraste élevé

onboarding-tabs = Les réglages se répartissent sur trois onglets — Inspecteur, Scène, Apparence.
onboarding-quick-toggles = Astuce : V bascule la visibilité, G la gravité — sans ouvrir ce panneau.
onboarding-theme = Les thèmes s'appliquent instantanément — pas de redémarrage.
onboarding-coach-step1 = Welcome! Your characters live on the desktop. Click the gear button in the top-right corner to enter edit mode.
onboarding-coach-step2 = Drop a PNG, GIF, WebP or MP4 anywhere on the screen to add it as a character. The side panel edits everything you select.
onboarding-coach-step3 = Ctrl+K opens the command palette. Ctrl+Shift+A toggles edit mode from anywhere, Ctrl+Shift+H hides the overlay.
onboarding-coach-next = Next
onboarding-coach-skip = Skip tour
onboarding-coach-done = Got it
onboarding-dismiss = Fermer

menu-duplicate = Dupliquer
menu-reset-transform = Réinitialiser la transformation
menu-toggle-gravity = Basculer la gravité
menu-bring-forward = Mettre au premier plan
menu-send-backward = Renvoyer à l'arrière
menu-delete = Supprimer

toggle-enter-edit = Entrer en mode édition
toggle-exit-edit = Quitter le mode édition

palette-search-placeholder = Rechercher des thèmes / presets…
palette-close-hint = Esc pour fermer · Ctrl+K pour basculer
palette-switch-theme = Passer au thème { $theme }
palette-apply-preset = Appliquer le preset : { $preset }

settings-tab-library = Bibliothèque

# Asset library tab
library-empty-headline = Aucun asset indexé
library-empty-hint = Déposez des fichiers dans ~/.local/share/animaEngine/assets/ ou définissez ANIMA_ASSETS_DIR.
library-no-asset-root = Aucun dossier d'assets trouvé. Créez-en un dans ~/.local/share/animaEngine/assets/
library-search-placeholder = Rechercher des assets…
library-add-to-scene = Ajouter à la scène
library-sort-recent = Récents
library-sort-name = Nom
library-kind-image = Image
library-kind-animated = Animé
library-kind-video = Vidéo
library-asset-added-toast = { $name } ajouté à la scène
library-asset-add-failed-toast = Impossible d'ajouter { $name }
library-count = { $n } assets indexés

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
