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

Reduce motion in Appearance (`reduced_motion`) is the setting of its
own, for any theme, and it reaches the characters too: no decorative
bobbing, no recoil from the cursor, no throw when one is let go
mid-drag, none of the little effects (hearts, dust, z's), and no fades —
they come and go at once (1.5). The selected row's pulse in the Scene
tab holds still. What a character does — walking, a behavior, a game of
tag — is left as it is: that is the scene, not decoration.

### 4. Screen readers, over AT-SPI (both backends since 1.4)

egui describes every widget it draws as an AccessKit tree.
[src/a11y.rs](../src/a11y.rs) publishes that tree on AT-SPI, the Linux
accessibility bus, and hands a screen reader's requests — focus this,
press that — back to egui. Orca and other ATs can read the settings
panel, the command palette and the tour, follow focus as Tab moves it,
and operate the controls. One bridge serves the X11 path and native
Wayland alike; each egui renderer owns one.

**Before 1.4 there was none.** From 0.2 on this page said that
egui-winit's `accesskit` feature was enough. It is not: egui-winit's
adapter exists only once `init_accesskit` is called, and nothing ever
called it. egui built its tree and dropped it, and no screen reader
ever saw the app, on either backend (R53 in
[runtime-findings.md](runtime-findings.md)).

The bridge is idle until an assistive technology turns AT-SPI on
(`org.a11y.Status.IsEnabled`): only then does egui build a tree. Without
one it costs a thread and a session-bus connection watching that flag.

Sprite content (the animated characters themselves) stays
deliberately outside this tree. They are visual decoration with no
semantic meaning; surfacing them as widgets would only pollute the
focus order. The accessible tree describes the *controls*, not the
canvas.

**What they say is read out (1.5).** A speech bubble is drawn, not a
widget, yet it can carry what someone needs to hear: a reminder, the
focus timer's break, a greeting. Every line a character starts saying —
its own, a reminder, the focus timer, a welcome back, a greeting, a
script's `say` — becomes a live label at the top of the tree, "Cat: Time
for a break!", which the bridge announces (`object:announcement`, polite)
the moment it appears ([src/announce.rs](../src/announce.rs),
`ui::speech::announce`). Each line is a new node, so a reminder said
again is read again. The Zzz… of a sleeping scene is not announced. With
the setting off, or no reader listening, nothing is built.

Toasts are announced the same way (`ui::toasts::announce`): each one a
live node of its own while it lasts — polite for news ("Scene:
Evening"), assertive for a warning or an error ("Rejected: …") — in
both modes, as they are shown in both. A banner's text is live too, read out
when the panel opens on it. Checked on both backends with an AT-SPI
listener for `object:announcement`.

**Larger bubbles (1.5).** Appearance → "Large text in bubbles"
(`large_bubbles`) makes every bubble's text 1.4 times larger, wrapping
wider to match; a character's own text size (the Inspector's "Says")
scales with it. A bubble given a dark colour gets light text, picked by
its luminance, so it stays readable.

**Names.** The icons are private-use characters from an icon font, for
which a reader has nothing to say. The bridge drops them from every
name and value, so "+  Add file…" reads "Add file…", and hides what is
left with nothing to read: labels that were only an icon, and the empty
backdrops egui gives its floating areas. Controls that are *only* an
icon are named where they are built — see section 6.

**The Appearance setting** — **Appearance → Accessibility → Generate
AccessKit tree updates** — decides whether a listening reader gets the
tree. Off, it gets the application's window with nothing in it, so the
last tree does not stay readable and nothing typed in the panels
reaches the bus. Applies from the next frame; persisted in `[global]`
as `accesskit_enabled = true/false`.

**Screen coordinates.** On X11 the reader gets the window's position.
On native Wayland a client is not told where its surfaces are; the
bridge reports the primary output's position in the layout, which is
right for the settings panel. Reading, navigating and pressing do not
depend on either.

### 5. Discoverable & rebindable keyboard model

Every action animaEngine handles has an entry in
[src/keybindings/action.rs](../src/keybindings/action.rs) carrying a
label, description, default chord set, and stable i18n key. Three
surfaces read that table:

- **Keybindings tab** in the settings sidebar — renders the live
  chord table, lets the user record new bindings, surfaces conflicts
  inline, and offers per-row + global "reset to defaults" buttons.
- **Ctrl+K command palette** — lists the actions above, each with its
  first chord beside it (the ones on the selected character only while
  one is selected), then "Add file…", the themes and the presets, and
  runs one from the keyboard: type to filter, ↑↓ + Enter. While it is
  open every key is its own, so nothing typed there fires a shortcut.
- **Config file** (`~/.config/animaengine/config.toml`) — the
  `[keybindings.map]` table mirrors the in-memory `BTreeMap<Action,
  Vec<KeyChord>>`. Chord strings round-trip through
  `KeyChord::FromStr` (`"Ctrl+Shift+A"`, `"Esc"`, `"ArrowUp"`, …) so
  hand-editing is supported.

Bindings introduced after a config was first written fall back to
their defaults at lookup time, so users upgrading from 0.3 don't
silently lose actions added in 0.4.

#### Without a mouse

Everything in edit mode can be reached from the keyboard (1.5, checked
on both backends):

- `Ctrl+Shift+A` opens edit mode. `Tab` moves through the settings
  panel, `Enter` or `Space` presses what has the focus, and `Esc` gives
  the keyboard back to the scene, still in edit mode.
- A character is picked in the Scene tab's list: `Tab` to its name and
  `Enter`; `Shift+Enter` adds it to the selection, or removes it. With
  the keyboard back on the scene, `N` steps through them and `Ctrl+A`
  selects them all.
- `Shift+F10`, or the Menu key, opens the right-click menu for the
  selection, its first item focused: `Tab` moves through it, `Enter` picks, `Esc` closes it.
  Save as GIF, Share, Arrange and Group are there.
- `Ctrl+K` runs any action by name.

`Tab` is the panel's, never a shortcut's. Cycle character was bound to
it until 1.5 and never cycled while the panel was open — on X11 the
toolkit took every `Tab`, on native Wayland only the first did — so it
moved to `N`; a config that still had it on `Tab` as it came moves with
it.

#### Global vs in-app chords

Global hotkeys are restricted to chords with at least one modifier
(Ctrl / Alt / Super) — registering a bare letter via `XGrabKey`
would steal the key from every focused app. The default global set
(`Ctrl+Shift+A` / `Ctrl+Shift+H` / `Ctrl+Shift+P`) maps onto
`ToggleEditMode` / `HideOverlay` / `PauseAll`; rebinding any of those
onto an unmodifier chord silently demotes it to in-app-only. The focus
timer, the snooze and muting the sounds (`ToggleFocus`, `ToggleSnooze`,
`ToggleMute`, 1.5) can be global too, once given a chord: they ship
unbound.

### 6. Icon-only controls get names

Any control rendered with a Phosphor glyph as its only visible label
(toggle button ⚙, trash button, ✕ dismiss, the per-row reset, tab
switcher chips) carries a tooltip that names the action — and, since
1.4, the same name for screen readers. A tooltip alone is not enough:
egui does not pass hover text to AccessKit, and until 1.4 these
buttons read as nothing. [src/ui/accessible.rs](../src/ui/accessible.rs)
has the helpers:

- `on_hover_name(text)` — the tooltip, and the name.
- `named(name)` — a name that says more than the tooltip, where a
  panel repeats one icon: "Delete: Ghost Demo", "Reset to default:
  Pause all animations", "Append preset: Cozy Companion".
- `name_combo(response, name, value)` — a combo box built without a
  label of its own, named after the label beside it, its current
  choice as the value.
- `name_text_field(response, name)` — a text field with only an icon
  beside it and a hint inside, like the palette's search box. A hint is
  not a name to a screen reader.
- `picked()` in place of `clicked()` on a combo box's options — it also
  closes the list. egui closes it on a pointer click only, so an option
  chosen from the keyboard or by a screen reader left it open.

A field with a separate label before it — `ui.label(…)`, then a
`DragValue` or `TextEdit` — has no name of its own either; tie the two
with egui's `.labelled_by(label.id)`, as the Inspector's z-index does.

The lint is informal — please keep it.

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

animaEngine renders no audio. Notifications appear as visual toasts,
in the bottom-right corner in either mode;
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
cargo test --lib keybindings  # action metadata, chord round-trip, conflict detect
```

```bash
cargo test --lib a11y         # the AT-SPI bridge: activation, requests, names
```

Manual screen-reader smoke test on Linux, on either backend:

```bash
# Start Orca on an empty workspace
orca &
RUST_LOG=anima_engine=info cargo run
# Enter edit mode, Tab through the settings panel; Orca should speak
# each widget's name.
```

Without Orca, any AT-SPI client shows what a reader gets — Python's
`gi.repository.Atspi` lists the application, walks its tree, and can
focus and press nodes, which is how 1.4 was checked.

If a control reads as "unlabelled", or as nothing, give it a name with
the helpers in section 6 at its construction site under
`src/ui/panels/` (one file per tab).
