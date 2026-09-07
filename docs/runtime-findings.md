# Runtime findings — interactive / visual defects

Defects found by **running the application and looking at it**, rather than by
static analysis, fuzzing or unit tests. Every entry here was observed on
screen; none of them would have been caught by the test suite, because the
test suite covers the parsers and the pure logic, not the rendered product.

Status legend: `OPEN` needs fixing · `FIXED` resolved, kept for history.

## How these were reproduced

A headless [sway](https://swaywm.org) session plus a virtual pointer, so the
app can be driven and screenshotted without touching a real desktop:

```sh
# 1. Compositor with a virtual output
WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 \
WLR_RENDERER=pixman sway -c <minimal config>

# 2. The app, with its XDG dirs sandboxed so a test never touches real config
XDG_CONFIG_HOME=... XDG_CACHE_HOME=... XDG_DATA_HOME=... \
ANIMA_USE_WAYLAND_NATIVE=1 ./target/debug/anima_engine

# 3. Screenshot
grim out.png
```

Two things make this harder than it looks, and are worth writing down:

- **`wlrctl` does not work here.** It creates its virtual pointer, sends one
  action and exits, so the seat gains and loses its pointer capability before
  a client can bind a `wl_pointer` — nothing is ever delivered. Input needs a
  pointer that stays alive for the whole script, and that pointer must exist
  *before* the app starts, or the app has no pointer to bind at startup.
- **`sway`'s `seat cursor` IPC commands report success but deliver nothing**
  when the seat has no pointer device, for the same reason.

Absolute positioning (`zwlr_virtual_pointer_v1::motion_absolute`) is required
to aim at a widget; `wlrctl`'s motion is relative only.

---

## Interaction

### R1 · Context menu closes on the release of the click that opened it — `FIXED`

Right-clicking an entity opens the context menu on button *press* and closes
it again on *release*, so a normal right-click makes it flash and vanish. The
menu is only usable while the button is physically held down.

Verified by holding the right button: the full menu renders correctly
(Duplicate / Reset transform / Toggle gravity / Bring forward / Send backward
/ Delete). On release it is gone.

This makes the context menu — six of the app's entity actions — effectively
unreachable in normal use.

Observed on the native Wayland path. **Not** verified on the winit/X11 path,
which sets the menu state from the same `Pressed` event and may share the
defect.

**Fixed.** Two compounding causes: the menu was anchored with its top-left
exactly at the click point, so the pointer sat on the rect boundary where
egui reports `contains_pointer() == false` (probed: `any_click=true`,
`contains_pointer=false`, `interact_pos == rect.min`); and dismissal did not
exclude the click that opened the menu. The menu is now offset 2px off the
cursor, and `ContextMenuState` carries an `armed` flag set once it has been
shown. The shared fix covers both backends. Verified in both directions:
a normal right-click leaves the menu up, a later click outside dismisses it.

### R2 · No entity drag on the native Wayland path — `FIXED`

`DragController` is never used anywhere under `src/wayland/`. Dragging a
character with the mouse — arguably the core interaction of a desktop-pet
application — is not implemented on the native backend at all.

Worse, `src/wayland/run.rs`'s own module doc has a "What doesn't (yet)"
section listing three limitations (FollowCursor in pass-through,
window-awareness physics, `XGrabKey` hotkeys) and **drag is not among them**,
so the documentation implies it works.

**Fixed.** `App::handle_mouse_input`'s behaviour is now mirrored on the
Wayland path — press selects and picks up, motion moves (invalidating the
Bounce rest position), release drops, and a press/release that never moved
pokes instead. The module doc now lists this under "What works". Verified by
dragging a character across a headless sway session.

### R3 · Left-click does not select an entity on Wayland — `FIXED`

`src/wayland/run.rs` only matches `egui::PointerButton::Secondary` when
resolving a click to an entity, so selection is right-click only. The winit
path selects on left-click (`src/app/input.rs`). The Inspector's own empty
state says *"Click an entity in the Scene tab, or press Tab to cycle"* — it
does not mention that clicking the sprite works differently per backend.

**Fixed** as part of R2 — left press now selects, and clicking empty space
deselects, matching winit.

---

## Content

### R4 · A 1.0.0 release greets every new user with "What's new in 0.4" — `FIXED`

`WHATS_NEW_VERSION` in `src/ui/whats_new.rs` is `"0.4.0"`, and the header
string is translated as "0.4" in **all ten locales**:

```
src/i18n/locales/en.ftl:192:whats-new-header = What's new in 0.4
… de, es, fr, it, ja, nl, pl, pt-BR, ro — all the same
```

The four highlights are 0.4-era features (keybindings tab, collapsed-section
persistence, error banners, AccessKit toggle). This is the first thing a new
user sees, on every tab.

**Fixed.** Highlights derived from `CHANGELOG.md`'s user-facing entries
between 0.4 and 1.0 rather than invented: the stable release behind three
candidates and an external audit, native Wayland reaching parity with X11,
config durability, and physical-pixel overlay geometry. The nine
non-English strings are machine translations and want a native-speaker
pass before release.

Fixing it surfaced two adjacent defects, R12 and R13 below, and one design
question worth stating: `should_show()` returns true for `last_seen ==
None`, which is exactly a **fresh install** — so a user who had never run
any version was shown a changelog for a release they were never present
for, on top of the onboarding tour they also get. A config created because
no file existed is now stamped with the current anchor, leaving the panel
to upgraders. A config that exists but fails to parse is not a fresh
install and still sees it.

---

## Rendering

### R5 · `→` renders as a missing-glyph box — `FIXED`

"AccessKit can be turned off from Appearance **□** Accessibility" — the arrow
in `whats-new-accessibility-toggle` has no glyph in the proportional font.

The app bundles no fonts of its own; it uses egui's defaults. Note that `↑`
*does* render in the keybindings chords (monospace), so this is narrower than
"arrows are broken" — it is the proportional face that lacks U+2192. Any
string using `→` in body text is affected.

**Fixed.** Reading the bundled cmap tables settles it: Ubuntu-Light,
NotoEmoji-Regular and emoji-icon-font cover *none* of U+2190..U+2193, while
the bundled Hack face covers all four — which is why the same arrows looked
fine in the monospace keybindings chords. The command palette's "↑↓ + Enter
to pick" footer was affected too, though never observed. `icons::install`
now appends the monospace list to the proportional family as a last-resort
fallback: no new asset, and it covers anything else Ubuntu-Light lacks.

### R6 · Sprites bleed through the settings panel — `BY DESIGN`

Entities positioned under the sidebar show through its background and land
behind the panel's text, on every tab.

**Not a defect.** The panel is deliberately frosted — `panels::settings`
builds its frame at alpha 235/255 with the comment *"the desktop reads
through behind the settings instead of a solid slab"*. Measured with the
near-white demo `star` (RGB 253,248,208) behind it:

| behind the panel | panel background | text contrast |
|---|---|---|
| nothing | (30, 34, 43) | 13.5:1 |
| the star | (77, 67, 44) | **8.1:1** |

Body text still clears WCAG AA (4.5:1) and AAA (7:1), so this is visual
noise rather than a legibility failure. Lowering the alpha is a one-line
change in `panels::settings` if the frosted look is ever judged not worth
the distraction — but that is a taste call, not a bug fix.

### R6b · Panel alpha does not blend uniformly across channels — `OPEN`

Found while measuring R6, and more interesting than R6 itself.

Solving the composite for the blend factor, per channel, using the same
sprite and panel:

```
star (253,248,208) over surface (30,34,43) measured as (77,67,44)
  R -> effective alpha 0.789
  G -> effective alpha 0.846
  B -> effective alpha 0.994      designed: 235/255 = 0.922
```

A single correct blend yields **one** alpha for all three channels. Three
different values means the alpha is being applied in the wrong colour space
— sRGB-encoded values blended as if linear, or alpha applied twice.

This is the family the external audit raised as M12 (the alpha/colour
contract per presenter, and `pick_alpha_mode` accepting `PostMultiplied`
while the pipeline stays premultiplied, "which can apply alpha a second
time"). The audit could only reason about it statically and scoped it to
the Windows presenter; this is a measurement showing it **on Linux**.

Not isolated further: the cause could be the egui blend state, the
`Bgra8UnormSrgb` surface format, or the premultiplication contract at the
layer-surface boundary. Worth pinning down before trusting any colour or
opacity value end-to-end.

### R7 · Keybindings rows wrap mid-token and overlap — `FIXED`

At a normal panel width (≈460 px on a 1600 px output):

- chords wrap across three lines mid-token — `Ctrl+` / `Shift` / `+H`
- the **"+ Add" button breaks its own label** into `+` / `Ad` / `d`
- that button then **overlaps the chord label**, so e.g. the `↑` of
  "Nudge selection up" is partly hidden behind it

**Fixed.** With that little room egui was breaking text *inside* items.
`TextWrapMode::Extend` on the chips, the add button and the
unbound/recording labels stops any single item breaking, so the row wraps
between whole items instead — which is what `horizontal_wrapped` is for.

### R8 · Banner text is clipped by its close button — `FIXED`

"Tip: V toggles visibility, G toggles gravity — no need to open this panel"
runs into the `✕` with no gap and is cut at the panel edge.

**Fixed.** The body label was added before the close button in a horizontal
layout, so a long hint claimed the whole row and the button was drawn over
it. The button now takes the right edge first and the body wraps into the
width actually left, inside a nested left-to-right layout so wrapped lines
stay left-aligned.

---

### R12 · "Settings split across three tabs" — there are five — `FIXED`

`onboarding-tabs` named Inspector, Scene and Appearance. Library and
Keybindings landed later and the hint was never updated, in any locale.

### R13 · Banner text silently decided the settings panel width — `FIXED`

The what's-new highlight labels did not wrap, so they asked for their
natural width and the `SidePanel` grew to satisfy them. Measured on a
1600px output:

| panel contents | width |
|---|---|
| no what's-new panel | **320px** — the configured `default_width` |
| the old 0.4 copy | 424px |
| longer 1.0 copy | 593px |

So the longest translated string in any locale was deciding how much of
the screen the settings panel took. Wrapping the labels returns it to the
configured 320px. Worth remembering when adding copy: `SidePanel` grows to
fit unwrapped content regardless of `default_width`.

## First-run experience

### R9 · Notices crowd out the settings — `FIXED`

The "What's new" panel renders at the top of **every** tab, not once. Together
with the persistent hint banners ("Settings split across three tabs…",
"Themes apply instantly…", "Press Ctrl+Shift+\` …"), roughly half the panel
height on first run is notices rather than settings.

**Fixed**, in two parts. R4's fresh-install change removes the changelog
for first-time users entirely, and the panel now renders only on the
default tab instead of above every tab body — it used to reappear each
time the user switched tab, competing with whatever they had navigated to.

### R10 · X11-only toggle is live in a Wayland session — `FIXED`

The Scene tab offers "Land on windows (X11)" as an active control under
native Wayland, where window-awareness is inert by design (no EWMH
equivalent). It is labelled `(X11)` but is not disabled or explained.

**Fixed.** Disabled on the backends that cannot serve it, with the reason
stated inline rather than only in a tooltip. Note the test is *not* the
display server: a Wayland session running the app through XWayland reads
EWMH fine, which is the common case on GNOME. The caller passes what its
backend actually has — `true` from winit, `false` from `run_native`.

---

## Fixed

### R11 · Soak metrics never recorded on the native Wayland path — `FIXED`

`SoakRecorder` was constructed and sampled only in the winit render loop, so
`ANIMA_SOAK_METRICS` was a silent no-op under `ANIMA_USE_WAYLAND_NATIVE` —
the file stayed empty with no indication the backend was being skipped.
Wired into `run_native`; rows now appear.

---

## Not defects — recorded so they are not re-investigated

- The demo `star` entity sits at `x = 1300`, so it is off-screen on outputs
  narrower than ~1400 px. The shipped default targets 1920×1080.
- The renderer refuses to start when the surface offers no transparent alpha
  mode (bare Xvfb, no compositor). That is deliberate and the error message
  says so — an opaque overlay would cover the desktop in black.

---

## Still unexamined

Areas never opened during this pass, listed so the next session knows where
the map ends: command palette (`Ctrl+K`), keyboard shortcuts end-to-end,
Shimeji pack import, drag-and-drop of files onto the overlay, preset
Append/Replace, multi-monitor visual behaviour, theme switching and the
non-dark themes, every locale other than English, and the whole winit/X11
path interactively — a compositing X server was not available here.
