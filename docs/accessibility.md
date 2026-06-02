# Accessibility

What animaEngine commits to, and what it deliberately doesn't.
Companion to [docs/design-system.md](design-system.md) — the design
system is the *what*, this document is the *who*.

## Commitments

### 1. WCAG-graded colour contrast

Every shipped theme is unit-tested against WCAG 2.1 contrast
thresholds. See [src/ui/theme.rs](../src/ui/theme.rs) `tests` module.

| Theme | Body text on surface | Bar | Semantic on elevated | Bar |
|-------|----------------------|-----|----------------------|-----|
| Dark | ≥ 4.5:1 | AA | ≥ 4.5:1 | AA |
| Light | ≥ 4.5:1 | AA | ≥ 4.5:1 | AA |
| Dark · High contrast | ≥ 7:1 | **AAA** | ≥ 4.5:1 | AA |
| Light · High contrast | ≥ 7:1 | **AAA** | ≥ 4.5:1 | AA |

CI runs the contrast tests on every PR. A regression that drops
*any* foreground/background pair below its target threshold fails the
build — colour drift can never silently make the UI harder to read.

### 2. Visible focus indicators

Standard themes draw the focus ring with a 2 px accent stroke; the
high-contrast variants use 3 px (still using `accent_base`, which is
guaranteed ≥ 7:1 on the surface they sit on). The focus ring is the
sole signal animaEngine sends to keyboard-only users about "you are
here" — it is never disabled, never hover-only, never colour-only
(the stroke width itself is a non-colour cue).

### 3. Reduced-motion default for HC

High-contrast themes also imply *no animation*: `Style::animation_time
= 0.0`. Users who enable HC are often using AT software with motion
sensitivity, and egui's built-in hover / tab-switch animations would
strobe under their assistive tech. We turn them off unconditionally,
not as a separate setting — the assumption is that anyone who wants
reduced motion will also want HC, and the reverse is rarely false.

### 4. AccessKit screen-reader bridge

The `accesskit` feature on `egui-winit` is enabled by default in our
Cargo.toml. This wires `AccessKit` into the egui input loop, which
on Linux means AT-SPI events get emitted for every widget egui paints
— buttons, sliders, text fields, the lot. Orca, Speakup, and other
ATs read the UI without any extra work from us.

Sprite content (the animated characters themselves) stays
deliberately outside this tree. They are visual decoration with no
semantic meaning; surfacing them as widgets would only pollute the
focus order. The accessible tree describes the *controls*, not the
canvas.

### 5. Discoverable keyboard model

Every action animaEngine handles has an entry in
[src/ui/keyboard.rs](../src/ui/keyboard.rs) with a label, a one-line
description, and its default key combo. The Appearance tab renders
this table read-only; the Ctrl+K command palette uses the same
metadata to fuzzy-search across actions, themes, and presets.

The keymap is not yet user-rebindable; the registry is in place so
0.3 can land that without UI surgery.

### 6. Icon-only buttons get tooltips

Any control rendered with a Phosphor glyph as its only visible label
(toggle button ⚙, trash button, palette close, tab switcher chips)
carries an `on_hover_text` tooltip that names the action. Egui
surfaces those tooltips through AccessKit, so screen readers see
"Delete entity" instead of "U+E4A6". The lint is informal — please
keep it.

## Non-goals (deliberately not done)

### High-contrast mode on the *sprites*

The overlay is fundamentally a visual product: cartoon-style PNG
characters animating on the user's desktop. We do not recolour or
silhouette-outline asset content based on the active theme. Users who
need to suppress sprite visuals entirely can:

- Use the toggle button ⚙ to enter pass-through mode where only the
  16×16 corner control is visible
- Toggle visibility per entity in the Inspector (V key)
- Hide the whole overlay via the tray menu or `Ctrl+Shift+H`

### Per-user audio cues

animaEngine renders no audio. Notifications appear as visual toasts;
the user's desktop notification daemon (e.g. `dunst`, GNOME Shell
notification stack) handles any system-level sounds for tray
activity.

### Voice control

Out of scope for 0.2. A voice layer would have to feed back through
the same `Action` enum used by keyboard handling, which keeps the
door open without adding the surface area now.

## Tooling

Run the accessibility-relevant tests in isolation:

```bash
cargo test --lib ui::theme    # contrast + HC palettes
cargo test --lib ui::keyboard # action metadata completeness
```

Manual screen-reader smoke test on Linux:

```bash
# Start Orca on an empty workspace
orca &
RUST_LOG=anima_engine=info cargo run
# Tab through the settings panel; Orca should speak each widget label.
```

If a control reads as "unlabelled" or "graphic", add `on_hover_text`
to its construction site in `src/ui/panels.rs` — that's almost always
the fix.
