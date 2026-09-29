# English — canonical reference. Every other locale ships the same keys.
# Keep alphabetical within sections so diffs stay easy to review.

# ── App chrome ────────────────────────────────────────────────────────
app-name = animaEngine

# ── Settings sidebar ──────────────────────────────────────────────────
settings-tab-inspector = Inspector
settings-tab-scene = Scene
settings-tab-appearance = Appearance
settings-tab-library = Library

# ── Asset library tab ────────────────────────────────────────────────
library-empty-headline = No assets indexed
library-empty-hint = Drop files into { $path } or set ANIMA_ASSETS_DIR to point at your collection.
library-no-asset-root = No asset directory found. Create one at { $path }
library-search-placeholder = Search assets…
library-add-to-scene = Add to scene
library-kind-image = Image
library-kind-animated = Animated
library-kind-video = Video
library-asset-added-toast = Added { $name } to the scene
library-asset-add-failed-toast = Couldn't add { $name }
library-count = { $n } assets indexed

entity-count-zero = No entities
entity-count-singular = { $n } entity
entity-count-plural = { $n } entities

# ── Inspector tab ─────────────────────────────────────────────────────
inspector-section-position = Position
inspector-section-appearance = Appearance
inspector-section-animation = Animation
animation-easing-label = Easing
easing-linear = Linear
easing-ease-in-quad = Ease in
easing-ease-out-quad = Ease out
easing-ease-in-out-quad = Ease in / out
easing-sine = Sine
easing-bounce-out = Bounce out
inspector-section-behavior = Behavior
inspector-visible = Visible
inspector-gravity = Gravity
inspector-scale = Scale
inspector-behavior-speed = Speed
inspector-behavior-comfort = Comfort distance
inspector-behavior-amplitude = Amplitude
inspector-behavior-period = Period
inspector-double-click-reset-hint = Double-click to reset to default.
inspector-opacity = Opacity
inspector-fps = FPS
inspector-playing = Playing
inspector-x = X
inspector-y = Y
inspector-z-index = z-index
inspector-nothing-selected-headline = Nothing selected
inspector-nothing-selected-hint = Click an entity in the Scene tab, or press Tab to cycle through them.

# ── Behaviors ─────────────────────────────────────────────────────────
behavior-idle = Idle
behavior-walk = Walk around
behavior-follow = Follow cursor
behavior-wander = Bounded wander
behavior-bounce = Bounce
behavior-bounce-axis = Axis
behavior-bounce-horizontal = Horizontal
behavior-bounce-vertical = Vertical
behavior-bounce-both = Both (circle)
behavior-script = Script
behavior-script-path-label = Path
behavior-script-path-hint = Relative to your asset library — { $path }
behavior-script-params = Parameters
behavior-script-param-name = name
behavior-script-add-param = Add
behavior-script-remove-param = Remove this parameter
script-failed-toast = Behavior script { $script } failed: { $error }

# ── Scene tab ─────────────────────────────────────────────────────────
scene-empty-headline = Empty scene
scene-empty-hint = Drop a PNG / GIF / WebP / MP4 onto the overlay — or try a preset below.
scene-drop-hint = Drop a PNG / GIF / WebP onto the overlay to add one.
scene-add-file = Add file…
scene-add-file-tooltip = Choose images or videos to add as characters.
file-chooser-title = Add characters
file-chooser-filter = Images and videos
file-chooser-unavailable-toast = The file chooser could not be opened. It needs xdg-desktop-portal.
scene-presets-header = Presets
scene-groups-header = Groups
scene-preset-append = Append
scene-preset-replace = Replace
scene-preset-replace-tooltip = Wipes the current scene before adding

# ── Monitor / scene distribution ──────────────────────────────────────
monitor-section-header = Monitors
monitor-mode-label = Distribution
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Span all monitors
monitor-mode-span-unsupported = One overlay cannot cover several monitors on this backend — use per-monitor instead.
monitor-mode-single = Single monitor
scene-window-awareness = Land on windows (X11)
scene-window-awareness-tooltip = Physics-enabled characters land on and walk along the top edges of your open windows. X11 sessions only — Wayland offers no window positions, so this does nothing there.
scene-window-awareness-unavailable = Unavailable on native Wayland — no protocol exposes window positions.
monitor-pin-label = Pin to monitor
monitor-pin-auto = Auto (follow position)
monitor-pinned-toast = Entity pinned to { $name }
monitor-pin-cleared-toast = Entity now follows its position
monitor-no-monitors-detected = No monitors detected

# ── Appearance tab ────────────────────────────────────────────────────
appearance-theme-header = Theme
appearance-theme-label = Theme
appearance-language-header = Language
appearance-language-no-font = No font for this script is installed — install a Noto CJK package first.
theme-dark = Dark
theme-light = Light
theme-dark-hc = Dark · High contrast
theme-light-hc = Light · High contrast

# ── Onboarding hints ──────────────────────────────────────────────────
onboarding-tabs = Settings live across five tabs — Inspector, Scene, Library, Appearance, Keybindings.
onboarding-quick-toggles = Tip: V toggles visibility, G toggles gravity — no need to open this panel.
onboarding-theme = Themes apply instantly — no restart needed.
onboarding-coach-step1 = Welcome! Your characters live on the desktop. Click the gear button in the top-right corner to enter edit mode.
onboarding-coach-step2 = Drop a PNG, GIF, WebP or MP4 anywhere on the screen to add it as a character. The side panel edits everything you select.
onboarding-coach-step3 = Ctrl+K opens the command palette. Ctrl+Shift+A toggles edit mode from anywhere, Ctrl+Shift+H hides the overlay.
onboarding-coach-next = Next
onboarding-coach-skip = Skip tour
onboarding-coach-done = Got it
palette-replace-row = Replace scene with: { $preset }
palette-append-row = Append preset: { $preset }
palette-footer-hint = Esc to close · Ctrl+K to toggle · ↑↓ + Enter to pick
onboarding-dismiss = Dismiss

# ── Context menu ──────────────────────────────────────────────────────
menu-duplicate = Duplicate
menu-reset-transform = Reset transform
menu-toggle-gravity = Toggle gravity
menu-bring-forward = Bring forward
menu-send-backward = Send backward
menu-delete = Delete

# ── Toggle button ─────────────────────────────────────────────────────
toggle-enter-edit = Enter edit mode
toggle-exit-edit = Exit edit mode

# ── Command palette ───────────────────────────────────────────────────
palette-search-placeholder = Type to search actions, themes and presets…
palette-switch-theme = Switch to { $theme } theme

# ── Keybindings tab (D.1) ─────────────────────────────────────────────
settings-tab-keybindings = Keybindings
keybindings-unbound = (unbound)
keybindings-add = Add
keybindings-recording = Press a chord… (Esc to cancel)
keybindings-conflict = Conflicts with { $action }
keybindings-reset-all = Reset all to defaults
keybindings-reset-one = Reset to default
keybindings-remove-chord = Remove this binding
keybindings-help = Custom shortcuts persist in config.toml
# Modifier names as this language's keyboards print them. Display only:
# config.toml always stores the English names.
key-mod-ctrl = Ctrl
key-mod-shift = Shift
key-mod-alt = Alt
key-mod-super = Super

# ── Action labels (D.1.7) ─────────────────────────────────────────────
# Used by the Keybindings tab and the command palette. Keep aligned
# with `Action::label()` defaults in src/keybindings.rs.
action-toggle-edit-mode = Toggle edit mode
action-hide-overlay = Hide / show overlay
action-pause-all = Pause all animations
action-quit-with-save = Quit (save config)
action-save-now = Save config now
action-open-command-palette = Command palette
action-undo = Undo
action-redo = Redo

# Groups (1.5)
action-group-selected = Group selection
action-ungroup-selected = Ungroup selection
menu-group = Group
menu-ungroup = Ungroup
group-default-name = Group { $number }
group-copy-name = { $name } copy
toast-grouped = Grouped as { $name }
toast-ungrouped = Groups dissolved: { $count }
toast-nothing-to-ungroup = Nothing selected is in a group
scene-groups-empty-hint = Select several characters and press Ctrl+G — or right-click them and choose Group.
scene-group-members = Characters: { $count }
scene-group-select-tooltip = Select the characters in this group
scene-group-rename = Rename group
scene-group-show = Show group
scene-group-hide = Hide group
scene-group-ungroup = Ungroup — the characters stay

# Arrange and snap (1.5)
arrange-align-left = Align left edges
arrange-align-center = Centre on one vertical line
arrange-align-right = Align right edges
arrange-align-top = Align top edges
arrange-align-middle = Centre on one horizontal line
arrange-align-bottom = Align bottom edges
arrange-distribute-horizontally = Space evenly across
arrange-distribute-vertically = Space evenly down
scene-snap = Snap while dragging
scene-snap-tooltip = Edges and centres snap to the monitors’ and to other characters’. Hold Alt while dragging to place freely.

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

# ── Accessibility section in Appearance tab (D.3) ─────────────────────
appearance-accessibility-header = Accessibility
appearance-accesskit-label = Generate AccessKit tree updates
appearance-accesskit-hint = Lets screen readers (Orca and others, over AT-SPI) read and operate the panels. Off, a screen reader sees an empty window. Note: while one is running, text you type in the panels is on the AT-SPI bus, where any process running as your user can read it.
appearance-accesskit-unsupported = Screen readers are not supported on this system yet.
appearance-reduced-motion-label = Reduce motion
appearance-reduced-motion-hint = Skips UI transitions (panel slide, fades, palette pop) and stops decorative bouncing. Animations that convey state still play.
appearance-hover-startle-label = Startle on hover
appearance-hover-startle-hint = Mascots scoot away from the mouse pointer when it comes near them. Cursor tracking is X11-only, so on native Wayland this only reacts in edit mode.
appearance-fullscreen-label = When an app is full screen
appearance-fullscreen-hint = Games, videos and presentations that fill the screen. Works on X11 and on Wayland compositors such as sway and Hyprland; on GNOME and KDE under Wayland, only for apps that run through XWayland.
fullscreen-hide = Hide the characters
fullscreen-pause = Pause them
fullscreen-ignore = Keep them going
# Pausing when away (1.5)
appearance-away-label = Pause when I’m away
appearance-away-hint = With no mouse or keyboard input for this long, the characters hold still; they carry on at your next input.
away-never = Never
away-after-minutes = After { $minutes } min
appearance-away-unavailable = This session cannot tell when you are away: that takes GNOME, a real X server, or a Wayland compositor with idle notifications.
appearance-battery-label = Pause on battery
appearance-battery-hint = Hold the characters still while the computer runs on battery.
# Named scenes (1.5)
scene-scenes-header = Scenes
scene-scenes-hint = Save what is on screen under a name, and switch back to it here, from the command palette, or with “Next scene” in the tray.
scene-scene-name-hint = Scene name
scene-scene-save-new = Save as a scene
scene-scene-load-tooltip = Switch to this scene — Undo switches back
scene-scene-save-over = Save what is on screen as { $name }
scene-scene-delete = Delete scene
scene-scene-delete-confirm = Delete?
scene-scene-delete-cancel = Keep it
scene-scene-save-failed = Not saved: { $error }
toast-scene-loaded = Scene: { $name }
toast-scene-failed = Scene not loaded: { $error }
toast-no-scenes = No saved scenes yet — save one in the Scene tab.
palette-load-scene = Switch to scene: { $name }

# ── Persistent warning banners (D.5) ──────────────────────────────────
warning-global-hotkeys-unavailable = Global hotkeys couldn't register (typical on a native Wayland session). The tray menu and the ⚙ button still work.
warning-hot-reload-disconnected = The hot-reload worker stopped unexpectedly; in-flight config edits won't apply until you restart the app.
warning-config-unreadable = Your config file could not be read, so the default scene was loaded. The original is kept next to it as config.toml.bak-corrupt.
action-toggle-perf-overlay = Toggle perf overlay

# ── What's new panel (D.7) ────────────────────────────────────────────
whats-new-header = What's new in 1.4
whats-new-add-file = Add file… in the Scene tab adds characters through your desktop's file chooser — no dragging needed.
whats-new-palette = Ctrl+K now runs any action, with its shortcut shown beside it.
whats-new-screen-readers = Screen readers such as Orca can now read and operate the settings panel.

# ── New onboarding hints (D.7) ────────────────────────────────────────
onboarding-keybindings = Click any chord to remove it; press a key combo to record a new one.
onboarding-perf-overlay = Press Ctrl+Shift+` to open the live perf overlay.
appearance-reset-onboarding = Reset onboarding hints

# ── Empty-state CTAs (D.8) ────────────────────────────────────────────
scene-empty-action-browse-presets = Browse presets
library-empty-action-copy-path = Copy path to clipboard

# ── Tooltips (D.9) ────────────────────────────────────────────────────
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
shimeji-no-library-toast = No asset library root — create { $path } first.
crash-report-found-toast = The previous session crashed. A report was saved at { $path } — please attach it to a GitHub issue.

# ── Group composition hint (C.9) ──────────────────────────────────────
inspector-group-hint = Composed by group { $group }: { $transform }
inspector-multi-selected = { $count } selected. This panel edits { $name }; dragging, shortcuts and the right-click menu act on all of them.

# ── App-layer toasts (V.6 — F1 closure) ──────────────────────────────
toast-config-saved = Config saved
toast-config-reloaded = Config reloaded from disk
toast-config-reload-failed = Config not reloaded — the file is invalid or still being written. Keeping the current scene.
toast-config-reload-discarded = Config not reloaded — you edited the scene while it was loading.
toast-save-failed = Save failed: { $error }
toast-rejected = Rejected: { $reason }
toast-added = Added { $name }
toast-load-failed = Load failed: { $error }
toast-entity-load-failed = { $name }: { $error }
toast-theme-switched = Theme: { $theme }
toast-preset-entry-failed = Couldn’t add preset entry: { $error }
toast-preset-loaded = Loaded preset: { $name }
toast-duplicated = Duplicated { $name }
toast-duplicated-many = Characters duplicated: { $count }
toast-duplicate-failed = Duplicate failed: { $error }
toast-deleted = Deleted { $name }
toast-deleted-many = Characters deleted: { $count }
toast-playback-resumed = Playback resumed
toast-playback-paused = Playback paused
toast-undone = Undone
toast-redone = Redone
toast-nothing-to-undo = Nothing to undo
toast-nothing-to-redo = Nothing to redo
inspector-wander-box = Wander box
toast-perf-snapshot = Perf snapshot: { $path }
toast-perf-snapshot-failed = Snapshot failed: { $error }
