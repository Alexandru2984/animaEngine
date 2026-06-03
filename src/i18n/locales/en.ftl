# English — canonical reference. Every other locale ships the same keys.
# Keep alphabetical within sections so diffs stay easy to review.

# ── App chrome ────────────────────────────────────────────────────────
app-name = animaEngine

# ── Settings sidebar ──────────────────────────────────────────────────
settings-tab-inspector = Inspector
settings-tab-scene = Scene
settings-tab-appearance = Appearance
entity-count-zero = No entities
entity-count-singular = { $n } entity
entity-count-plural = { $n } entities

# ── Inspector tab ─────────────────────────────────────────────────────
inspector-section-position = Position
inspector-section-appearance = Appearance
inspector-section-animation = Animation
inspector-section-behavior = Behavior
inspector-visible = Visible
inspector-gravity = Gravity
inspector-scale = Scale
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

# ── Scene tab ─────────────────────────────────────────────────────────
scene-empty-headline = Empty scene
scene-empty-hint = Drop a PNG / GIF / WebP / MP4 onto the overlay — or try a preset below.
scene-drop-hint = Drop a PNG / GIF / WebP onto the overlay to add one.
scene-presets-header = Presets
scene-preset-append = Append
scene-preset-replace = Replace
scene-preset-replace-tooltip = Wipes the current scene before adding

# ── Monitor / scene distribution ──────────────────────────────────────
monitor-section-header = Monitors
monitor-mode-label = Distribution
monitor-mode-per-monitor = Per monitor
monitor-mode-span = Span all monitors
monitor-mode-single = Single monitor
monitor-pin-label = Pin to monitor
monitor-pin-auto = Auto (follow position)
monitor-pinned-toast = Entity pinned to { $name }
monitor-pin-cleared-toast = Entity now follows its position
monitor-no-monitors-detected = No monitors detected

# ── Appearance tab ────────────────────────────────────────────────────
appearance-theme-header = Theme
appearance-theme-label = Theme
appearance-language-header = Language
appearance-keyboard-header = Keyboard
appearance-keyboard-note = Read-only for 0.2.0 — rebinding lands in a follow-up release.
theme-dark = Dark
theme-light = Light
theme-dark-hc = Dark · High contrast
theme-light-hc = Light · High contrast

# ── Onboarding hints ──────────────────────────────────────────────────
onboarding-tabs = Settings split across three tabs — Inspector, Scene, Appearance.
onboarding-quick-toggles = Tip: V toggles visibility, G toggles gravity — no need to open this panel.
onboarding-theme = Themes apply instantly — no restart needed.
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
palette-search-placeholder = Type to search themes / presets…
palette-close-hint = Esc to close · Ctrl+K to toggle
palette-switch-theme = Switch to { $theme } theme
palette-apply-preset = Apply preset: { $preset }
