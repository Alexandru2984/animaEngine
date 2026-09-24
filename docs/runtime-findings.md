# Runtime findings — interactive / visual defects

Defects found by **running the application and looking at it**, rather than by
static analysis, fuzzing or unit tests. Every entry here was observed on
screen; none of them would have been caught by the test suite, because the
test suite covers the parsers and the pure logic, not the rendered product.

Status legend: `OPEN` needs fixing · `FIXED` resolved, kept for history ·
`FIXED, unverified` patched on reasoning the rig cannot confirm — the
entry says why · `BY DESIGN` observed, deliberate, not changing ·
`RETRACTED` reported here in error, kept so the mistake isn't repeated.

**Current state: nothing is `OPEN`.** R1–R5, R7–R21, R23–R36 and R38–R44 are
`FIXED`; R6 and R22 are `BY DESIGN`; R6b is `RETRACTED`. R22 was the last
one open and is now explained rather than fixed — the `ERROR` line at
startup is one enumerated adapter failing a probe, and the evidence is in
its entry. R37 is `FIXED, unverified` and wants one drag on a real X11
session. Nothing on the list is unexplored any more.

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

### The winit/X11 path is drivable too

Earlier rounds recorded it as untestable: under Xvfb + picom the surface
offers only `Opaque`, and the renderer refuses by design rather than
painting the desktop black. That conclusion was about *Xvfb*, not about
X11.

Run it as an X client of **sway's own XWayland** instead and it works —
that is an xcb surface rather than a wayland one, a different WSI path
entirely:

```sh
# sway starts XWayland lazily, so ask a client rather than guessing :N.
# Never :0 — that is the real session, and the overlay would land on it.
swaymsg exec "sh -c 'echo \$DISPLAY > .../xdisplay'"

env -u WAYLAND_DISPLAY -u ANIMA_USE_WAYLAND_NATIVE DISPLAY=:2 \
    VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json ./anima_engine
```

**The driver has to be pinned to lavapipe.** On the same surface the
NVIDIA and RADV ICDs both still answer `Available alpha modes: [Opaque]`;
only `lvp` offers `[PreMultiplied, Inherit]`. That is worth remembering
next to R22, which could not distinguish "this machine has three GPUs"
from "this surface has no transparent mode" — here the three disagree on
one surface, which is evidence the adapter really is the variable.

Input works through the same virtual devices, since XWayland forwards the
seat. One difference: the window must be **clicked once** before it takes
keyboard focus — sway does not focus it on map, so a keystroke sent first
goes nowhere and looks exactly like a dead shortcut.

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

### R6b · Panel alpha "does not blend uniformly across channels" — `RETRACTED (measurement error)`

**There is no bug here. This entry was wrong, and it is kept only so the
mistake isn't made a second time.**

It originally read: solving the composite per channel gave effective alphas
of 0.789 / 0.846 / 0.994 against a designed 235/255 = 0.922, therefore the
alpha was being applied in the wrong colour space or applied twice — and it
was filed as a Linux-side confirmation of the external audit's M12.

Two independent errors produced that result.

**1. The wrong colour space — in the analysis, not in the app.** The
composite was solved in **sRGB byte space**. But the render target is an
sRGB format and the pipeline blends premultiplied:

- `wgpu_renderer.rs:322` — `caps.formats.iter().find(|f| f.is_srgb())`
- `wgpu_renderer.rs:409` — `blend: BlendState::PREMULTIPLIED_ALPHA_BLENDING`

With an `...Srgb` target the hardware decodes to linear, blends, and
re-encodes. Solving from raw screenshot bytes as though the blend were
linear-in-bytes is simply the wrong equation, and its error grows with the
distance between the two colours — which is exactly why the three channels
disagreed by so much, and why blue (where sprite and panel nearly match)
looked "correct" while red and green looked broken.

**2. The two pixels were never the same pixel.** The original numbers
compared the *brightest* star pixel found in one screenshot against a
*different coordinate* in another. The star is a gradient, so the "source"
colour fed into the equation was never the colour actually behind the
sampled point.

Re-measured properly — same coordinates in both frames, solved in linear
space — the channels with real signal land on the designed value:

```
 pixel        alpha R / G / B        designed 0.9216
 (1350,520)   0.921 / 0.923 / 0.867
 (1365,515)   0.926 / 0.928 / 0.948
 (1380,530)   0.919 / 0.922 / 1.000
```

R and G agree at **0.919–0.928**. Blue stays noisy because the sprite is
yellow: panel blue (43) and sprite blue (47) are nearly equal, so the
solve's denominator approaches zero and quantisation error explodes. That
is instability in the measurement, not spread in the blend.

A fully controlled capture (animation paused) was attempted to remove the
last variable and **failed**: pausing playback stops animation frames but
not behaviour movement, so the sprites still shift between captures. Anyone
retrying this needs a static scene — gravity and behaviours off — not just
paused playback.

The audit's M12 concern about `pick_alpha_mode` accepting `PostMultiplied`
while the pipeline stays premultiplied is **still open on its own terms**
and still scoped to Windows. It gained no Linux evidence here.

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

### R14 · Every modifier chord was dead on native Wayland — `FIXED`

The command palette had never been opened in a test. `Ctrl+K` did nothing.

`egui_render.rs` built its `RawInput` with a hardcoded
`modifiers: egui::Modifiers::default()`, so `input.modifiers` was
**permanently all-false** on the native Wayland path. Individual
`Event::Key`s carried the correct modifiers, which is why this survived
review — the events look right; the field egui actually answers
`input.modifiers` from is a different one.

Blast radius is wider than the palette. Anything reading `input.modifiers`
was affected, and that includes egui's own `TextEdit` chords, so **every
text field in the app** had no Ctrl+A, Ctrl+C/V/X, Ctrl+Z and no
shift-selection. It also reached the keybindings tab, which reads
`i.modifiers` when capturing a new chord (`keybindings_tab.rs`) — so
recording "Ctrl+Shift+X" on this backend stored a bare "X". Confirmed both ways: typing `dark`, pressing Ctrl+A, then
typing `zz` left `darkzz` before the fix and `zz` after.

**Fixed** by plumbing the live seat state through (`LayerWindow::modifiers`)
and, on top of that, preferring the modifier snapshot carried by the newest
key event in the frame (`effective_modifiers`). The second half matters on
its own: sampling only live state loses a chord whose press *and* release
both land inside one frame, which any long frame — a video or GIF decode
stall — makes possible. Unit tests cover both directions.

Two things this cost, worth writing down:

- **`wtype` is the keyboard's version of the `wlrctl` trap.** It creates a
  virtual keyboard, sends its keys and exits, so the seat's keyboard
  capability blinks in and out. The app binds `wl_keyboard` from
  `SeatHandler::new_capability`, which is asynchronous and never completes
  in time, so **not one key is delivered** and nothing is logged. The rig
  now holds a `wtype -s 3600000 -k Shift_L -k Shift_L` open for the whole
  run purely to keep the capability up.
- **`wtype` releases modifiers immediately**, faster than a frame, so
  `-M ctrl -k k -m ctrl` reproduces the stall case rather than normal
  typing. Use `-M ctrl -P k -s 800 -p k -m ctrl` to emulate a human hold.

### R15 · Inactive tab icons fail contrast in the Light theme — `FIXED`

Found on the first look at a non-dark theme, reached through the command
palette that R14 had just made usable.

The settings tab bar is **icon-only**, so each glyph is the whole label of
its control and owes WCAG 1.4.11's 3:1 for non-text UI components. Inactive
icons paint in `palette.fg_muted`:

```
                design    as rendered
 dark   #6B7280  3.29:1      3.17-3.37:1   pass
 light  #9CA3AF  2.29:1      2.11:1        fail
```

The rendered figures sit slightly below the design ones because the frosted
panel blends toward the desktop behind it.

There were already contrast assertions in `theme.rs`, but only for the two
*high-contrast* palettes — the ordinary Light and Dark themes had none for
`fg_muted`, so nothing caught this.

**Fixed.** Light `fg_muted` is now `#7C838F` (3.44:1 design, 3.04–3.17:1
rendered — parity with dark), and a test asserts ≥3:1 for both ordinary
palettes. Verified the test fails on the old value and passes on the new.

`fg_muted` has exactly one use outside `theme.rs`, this tab bar, so the
change is contained.

### R16 · Keybinding rows paint over each other — `FIXED`

Found while checking German for layout damage; it turned out not to be a
locale problem at all — English had it just as badly.

The tab was an `egui::Grid` whose middle column is a `horizontal_wrapped`
run of chord chips plus the "+ Add" button. The cell reserved height for
fewer lines than it went on to paint, so the *next* row's stripe was drawn
straight over the tail of the previous one.

Not cosmetic: for every action with two chords the "+ Add" button was
entirely covered — invisible and unclickable — and a chord chip was sliced
in half.

**Fixed** by dropping the grid for one `Frame` per action. A frame reserves
its background shape and fills it after laying out its contents, so the
stripe can never be shorter than what it sits behind. The label column now
wraps inside a fixed width, which also stops German widening the whole side
panel past its English width.

### R17 · Ten complete locales, English on screen — `FIXED`

The behaviour picker — Idle / Walk around / Follow cursor / Bounded wander
/ Bounce — drew hardcoded English in **every** locale, while
`behavior-idle`, `-walk`, `-follow`, `-wander` and `-bounce` sat translated
in all ten `.ftl` files, unused. Same for the Appearance theme row, the
inspector's X / Y labels, and two Dismiss tooltips.

`every_locale_covers_every_en_key` proved the translations existed. Nothing
proved the app ever asked for them, so this was invisible to the suite.

**Fixed**, and the gap closed with `every_en_key_is_referenced_in_the_source`
— it scans the crate for each English key as a string literal. Sound here
because keys are only ever named by literal; `Action::i18n_key` and friends
return `&'static str` out of a match.

That test then found four more keys, all dead rather than hardcoded:
`palette-close-hint` and `palette-apply-preset` were superseded by
`palette-footer-hint` and the Replace/Append rows, and
`library-sort-name` / `library-sort-recent` describe a sort control that
does not exist (`library.rs` contains no "sort"). Removed from every locale
rather than allowlisted, so the test stays strict.

### R18 · Clicking the settings panel throws away the selection — `FIXED`

Select a character, click any blank part of the settings panel, and the
Inspector drops back to "Nothing selected" — the click fell through to the
scene's hit test, found nothing, and deselected. The panel you are reading
is the thing that clears itself.

**This one was mine**, introduced with the R2/R3 fix that gave this backend
left-click selection and drag. The winit path never had the bug because
`App::window_event` hands every event to egui first and returns early when
it is consumed; the native Wayland loop reads the raw event list and had no
equivalent.

**Fixed** with `WaylandEguiRenderer::owns_pointer()`
(`Context::is_pointer_over_area()`) gating the two press arms. Only presses
are gated — motion and release stay live so a drag begun on a sprite still
tracks and still finishes if the pointer crosses the panel. Verified all
three ways: selection survives a panel click, sprites still select, and a
dragged character lands on the exact target.

### R19 · 27 of 28 keybindings do nothing on native Wayland — `FIXED`

Select a character, press the arrow keys bound to "Nudge selection". It
does not move. Verified with the character confirmed selected — the
Inspector showed "Cat Demo" throughout — so this is not a selection
problem.

The cause is a parity gap, not a bug in the key path. Keys arrive fine
(R14 fixed that). But `src/wayland/run.rs` consults the keybinding table
in exactly one place and matches exactly one action:

```rust
if let Some(Action::ToggleEditMode) = config.keybindings.lookup(chord) {
```

The winit/X11 path routes through `App::dispatch_action`, which handles
**27** actions. Native Wayland handles **one**.

So the Keybindings tab lists ~28 rebindable actions, lets the user rebind
them, and persists the result to `config.toml` — and on this backend all
but one are decorative. Nudge, delete, cycle, centre, the visible/gravity/
playback toggles, opacity, FPS, duplicate, z-order, save, quit: none of
them fire.

Ctrl+K is the exception that hides it, and only by accident — the command
palette reads `ctx.input()` inside egui rather than going through the
table at all.

**Fixed** by the split this entry predicted, measured rather than guessed:
of the 26 arms, **18** touch only the scene, the selection and the dirty
flag, and **8** genuinely need a window, a renderer or the event loop.

The 18 moved to `keybindings::shared::dispatch_shared`, which both backends
now call; it returns `false` for anything it does not own so each caller
still runs its own arms. Native Wayland went from 1 handled action to 19.

Verified on the rig rather than by reading: five presses of Right moved a
character exactly 50 px, `V` took it from 1199 visible pixels to 0 and back
to 1168, and `Tab` moved the selection from Cat Demo to Ghost Demo.

Still per-backend, because they differ for real: quit-with-save,
save-now, edit-mode toggle, delete, centre-on-screen, duplicate (it goes
through the renderer's texture cache), cycle-monitor and the perf overlay.
Those remain unavailable on native Wayland and are worth a follow-up.

This sat inside the project's standing X11/Wayland parity exception
(`CONTRIBUTING.md`, granted 2026-06-21).

### R20 · `Single` monitor mode ignores the chosen monitor on Wayland — `FIXED`

Set `monitor_mode = single` with `name = "HEADLESS-1"` (the right-hand
output of two) and the overlay renders entirely on the *left* one. Counted
directly: 5704 non-black pixels on the left, **zero** on the right.

The primary layer surface is created with no output:

```rust
let layer = layer_shell.create_layer_surface(
    &qh, wl_surface, Layer::Overlay, Some("anima_engine"),
    // Any output — the compositor picks, same as the X11 path
    // never explicitly positions its primary window either.
    None,
);
```

That justification is **stale**. The X11 path *does* position its primary
window now — `app/windows.rs` says so explicitly, and records that not
doing it was the bug which made "selecting a non-primary monitor in
`Single` mode silently do nothing". The same bug is still here, kept alive
by a comment pointing at behaviour that has since changed. Same shape as
the `_arc_used` placeholder whose stated reason had also stopped being
true.

**Fixed.** A layer surface's output is fixed at creation and the output
list has not arrived by then, so `try_create` now takes the monitor name
`Single` asks for, does one extra round-trip to learn the outputs, and
replaces the surface with one bound to the requested output. The
replacement is built only when a specific monitor was asked for and the
first surface is not on it — every other mode keeps the compositor's
choice and costs nothing.

The swap happens *before* the wgpu surface is built, so the careful
drop-order invariant documented around `build_wgpu_surface` is untouched:
nothing references the discarded surface.

Verified on the rig, exactly inverting the original measurement — with
`Single`/`HEADLESS-1` on two outputs there are now **0** non-black pixels
on the left and 6222 on the right. A stale monitor name warns and falls
back to the compositor's choice rather than failing to start, and a normal
single-output run is unchanged.

### R21 · `Span` covers one monitor on Wayland, but says "all" — `FIXED`

With two outputs and `monitor_mode = span`, the renderer initialises at
1600×1000 — one output — and a character placed at x = 2100 never appears.

Unlike R20 this is arguably inherent: a `wlr-layer-shell` surface belongs
to an output, so there is no single whole-desktop surface to create. The
code knows: `monitor.rs:313` says "`Span` draws **one** window sized to a
single monitor".

The defect is that nothing tells the user. The picker offers **"Span all
monitors"** and `docs/engine-features.md` promises "one overlay spanning
all monitors" — on this backend it silently means "one monitor, and your
characters on the others vanish".

**Fixed the honest way**, as R10 did for window-awareness: the mode is
offered disabled on backends that cannot span, with a tooltip pointing at
per-monitor instead. `span_supported` is plumbed separately from
`window_awareness_supported` even though both are false on the same
backend today — they are different capabilities, and conflating them would
mislead whoever adds the next backend.

Deliberately still available with a **single** monitor, where "span all"
and "cover this one" are the same thing and the label is not a lie. It only
misleads once a second monitor exists. `engine-features.md` now says X11
only rather than promising it flatly.

### R22 · A recovered startup condition is logged at ERROR — `BY DESIGN`

`get_physical_device_surface_capabilities: ERROR_SURFACE_LOST_KHR` is
logged at `ERROR` on every launch, single- and multi-output alike, and the
renderer initialises fine on the very next line.

**Correction on first writing this up:** the message is not ours. It does
not appear anywhere in `src/`, so it is wgpu's own log reaching the
terminal through our `tracing_subscriber`. That changes the fix from "move
one log line" to a judgement call:

- narrowing the default `EnvFilter` to quieten that wgpu target would stop
  the noise, but suppressing a graphics library's `ERROR` wholesale is a
  good way to miss a real device failure later;
- leaving it means every launch prints an `ERROR` the code handled, which
  is exactly how people learn to skim past the level that matters.

**Investigated, not resolved.** The hypothesis was multi-GPU adapter
probing: this machine exposes three Vulkan devices (RADV, NVIDIA,
llvmpipe), and wgpu asks each whether it supports the surface, so one
incompatible adapter answering `SURFACE_LOST` while another succeeds would
explain it entirely — and make it upstream noise rather than ours.

It could not be confirmed. Restricting `VK_DRIVER_FILES` to a single ICD
does make the message disappear, but every single-ICD configuration then
fails for an unrelated reason on this rig — llvmpipe and RADV both report
only `Opaque` composite alpha under headless sway, so the renderer refuses
before it would have logged anything. The two observations are therefore
not comparable, and "it goes away with one driver" proves nothing.

**Confirmed since, and the hypothesis was right.** The earlier attempt
compared two things that were not comparable, because every single-ICD
run refused before it could be observed. The way out was noticing that
the error is logged at 0.13 s and the alpha-mode refusal happens at
~0.45 s — so a run that *fails* still logs it, and single-ICD
configurations can be compared after all. Driving the winit path through
sway's own XWayland added a second, independent surface to compare
against.

Five runs on one machine, one moment, same set of drivers installed:

| surface | drivers offered | `SURFACE_LOST` |
|---|---|---|
| Wayland (layer shell) | all | **1** |
| Wayland | lavapipe only | 0 |
| Wayland | **RADV only** | **1** |
| Wayland | NVIDIA only | 0 |
| X11 (XWayland) | all | 0 |

So it is **RADV**, answering a surface-capabilities probe on the
layer-shell surface. In the all-drivers run the error is logged at
0.130 s and `Using "llvmpipe"` follows at 0.139 s: wgpu asks each
enumerated adapter about the surface, one says `SURFACE_LOST`, and
selection moves on to one that works. Exactly the multi-adapter story
above — upstream noise from adapter selection, not a failure of the
adapter actually used.

Why RADV says that *here* is visible in its own run: `eglInitialize`
fails with `DRI2: failed to get driver name`, so in this headless rig
RADV enumerates as a Vulkan device while having no usable render node.
An adapter that is present but cannot reach the hardware is precisely one
that should fail a surface query.

**Not filtered, and the original reasoning is unchanged:** a graphics
library's `ERROR` is what you want to still be reading the day a device
really is lost, and this costs one line at startup. What has changed is
that nobody needs to chase it again.

One thing this does *not* establish: whether the maintainer's real
desktop shows it at all. There RADV is the working GPU rather than a
broken enumeration entry, so it should not — but that is one launch and a
`grep SURFACE_LOST` away, and is worth doing rather than assuming.

### R23 · Japanese renders as boxes, end to end — `FIXED`

Start the app with `LANG=ja_JP.UTF-8` and **every Japanese character in
the UI is a missing-glyph box**. Not one label is readable. Latin text in
the same panels — `PNG`, `Ctrl+Shift+A`, `portal (GlobalShortcuts)`,
`config.toml` — renders fine, so this is purely a script-coverage gap.

Polish was checked alongside it and is perfect, diacritics included (ó, ą,
ę, ś, ż, ł), so Latin-extended coverage is not the issue. Japanese is the
only CJK locale shipped, so it is the only one affected — and it is
affected completely.

This is the same family as R5, where `→` had no glyph in the proportional
font, but the consequence is a different order of magnitude: R5 cost one
arrow, this costs an entire advertised language. The README promises "ten
UI locales"; nine of them work.

The bundled stack is egui's Ubuntu-Light → NotoEmoji → emoji-icon-font,
plus the Hack monospace face appended by the R5 fix. None carries CJK, and
no amount of re-ordering fixes that — the glyphs are simply not there.

Options, none of them free:

- **Bundle a CJK face.** Correct everywhere, offline, deterministic. Noto
  Sans JP is several MB even subset, against a 24 MB binary, and it only
  solves Japanese — Chinese or Korean later would want more.
- **Load a system font at runtime.** The test machine has 31 CJK faces
  installed, so on a desktop where someone actually reads Japanese one is
  almost certainly present. Needs a font-discovery dependency (none in the
  tree) and degrades to today's behaviour when nothing is found — which
  argues for pairing it with the next option.
- **Say so.** Whatever else is done, the language picker should not offer a
  language that cannot be drawn. Disabling it where no CJK face is
  available is the same honesty R10 applied to window-awareness.

**Fixed** with the second and third together. A short list of well-known
CJK font paths is probed and the first hit loaded — no font-discovery
dependency, which would have been a large tree for one lookup per session.
Loaded *only* when the active locale needs it, since the face is ~19 MB and
holding that for someone reading English buys nothing, and re-installed
when the language changes so switching at runtime works rather than
silently keeping the Latin-only stack.

When no CJK face exists, the picker offers those languages disabled with a
tooltip naming the missing package, instead of letting someone select a UI
they can no longer read — including the picker itself.

Verified on the rig: `LANG=ja_JP.UTF-8` now renders "インスペクター",
"何も選択されていません" and the full hint text.

### R24 · Romanian uses three words for the same thing — `FIXED`

The Keybindings panel titles itself **"Comenzi taste"**, the banner under
it says **"Scurtături"**, and the body text mixes both with
**"combinație"**. Counted across `ro.ftl`: "scurtături" 8×, "comenzi" 6×,
"combinație" 4× — for one concept, on one screen. **"chord" is left in
English** twice, which is jargon even in English.

Nothing renders wrong; the diacritics are all correct. This is the
machine-translation debt catching up, and it is worth a finding rather
than another generic "the nine locales want a native pass" line, because
it is now demonstrable rather than assumed.

**Correction to the count above.** Six of those "comenzi" are *correct*:
"paleta de comenzi" is the command palette, which really is a list of
commands. The inconsistency is narrower than first written — "comenzi"
used for *keybindings* while "scurtături" is used for the same thing
elsewhere.

**Fixed** on that narrower reading. "Scurtături" is the term for the
feature (matching the Windows and GNOME Romanian conventions),
"combinație" for the key sequence itself — a distinction worth keeping,
not collapsing — and "chord" is gone. The command palette strings are
untouched.

The other eight non-English locales have had no such check and very likely
carry the same kind of drift. Terminology is the maintainer's call; this
is one line per string to revise if a different word is preferred.

### R25 · Typing in any text field also fires the global shortcut — `FIXED`

On native Wayland, open the command palette and type `vi` into its search
box. The palette receives the text — and the selected character *also*
turns invisible and dumps its info to the log, because `v` and `i` are
bound to Toggle visible and Show entity info.

Every text field on this backend has the problem: the palette's search,
a numeric position field being typed into, the keybinding capture widget
that is supposed to be reading a chord. `d` duplicates, `g` toggles
gravity, `Delete` deletes.

The winit path never had it. `App::window_event` hands each event to
`egui_winit` first and returns early when it reports the event consumed,
and for key events `egui_winit` reports exactly `wants_keyboard_input()`.
The native loop instead reads egui's drained event list itself, so it
never saw that answer and had no gate at all.

**Fixed** by asking the same question the winit path asks implicitly: the
keybinding table is consulted only when `Context::wants_keyboard_input()`
is false. Ctrl+K still closes the palette, because
`panels::command_palette` handles that chord from `ctx.input` rather than
through the table — which is also why it works on the winit path, where
the outer dispatcher likewise never sees it.

Found while verifying R19's follow-up, not by looking for it: the palette
was open on screen from an earlier step when a test key was sent.

### R26 · Every Ctrl chord was `Ctrl+Super` — `FIXED`

Found while checking R25's fix: with the keyboard gate in place, `v` and
`Home` fired but **`Ctrl+M` and `Ctrl+Shift+A` did nothing at all**, on a
backend where `Ctrl+K` (the command palette) worked fine.

`Ctrl+K` works because `panels::command_palette` reads `ctx.input`
directly. Everything else goes through `KeyChord::from_egui`, which built
its modifier mask as:

```rust
ModifierMask::from_state(mods.ctrl, mods.shift, mods.alt, mods.mac_cmd || mods.command)
```

Off macOS, egui defines `command` as an alias for `ctrl` — and this path
sets it that way itself (`wayland/keyboard.rs`). So holding Ctrl produced
`ctrl && command`, the mask came out `CTRL | SUPER`, and the lookup
matched nothing in the table. Every Ctrl-prefixed action on the native
Wayland backend was dead, which is the same class of silence as R19 and
was hidden by it: R19 made the whole table unreachable there, so nobody
had yet pressed a Ctrl chord that *should* have worked.

**It was not only a Wayland problem.** The Keybindings tab captures the
chord the user pressed through the same `from_egui`, on both backends. So
rebinding anything to a Ctrl chord recorded `Ctrl+Super+X`, displayed it,
and wrote it to `config.toml` — a chord that cannot be typed on Linux,
because `egui::Modifiers` has no Super field for this path to set. The
binding was silently unusable from the moment it was saved.

**Fixed** by reading only `mac_cmd` as Super. `command` is left to egui's
own widgets, which is what it is for; macOS still round-trips Super
because `mac_cmd` is set there for the real Cmd key.

**Existing configs are not migrated.** A `Ctrl+Super+X` in `config.toml`
could also have been hand-written, and Super chords *do* fire on the winit
path, so rewriting them would break a legitimate binding to repair a
broken one. Anything rebound to a Ctrl chord before this fix needs
rebinding once; it never worked, so nothing is lost.

### R27 · Five more actions were silently dead on native Wayland — `FIXED` (four of five)

With R19 landed and R26 fixed, all twenty-eight rebindable actions were
pressed one at a time on the native Wayland backend for the first time.
Twenty-two work. **Five did nothing at all**: `Q` (quit and save), `S`
(save config now), `Del` / `Bksp` (delete), `D` (duplicate) and
`Ctrl+Shift+`` ` (perf overlay).

Four of those five are actively promised *by the app itself on that
backend*: the in-app help screen — `H`, which does work there — prints
"D — Duplicate", "Del/Bksp — Delete", "S — Save config" and "Q — Save and
exit". So the overlay told the user about four shortcuts it then ignored.

**Fixed for those four.** None of them is shareable with the winit path —
each reaches for something only its own loop owns — but every piece they
needed was already present in the native loop: `sync_and_save`,
`layer.state.close_requested`, and `handle_menu_action`, which the
right-click menu already used for delete and duplicate. Routing the
keyboard through the same handler as the menu is deliberate, so the two
cannot drift apart. Quit sets the close flag and lets the existing
shutdown path do the save, rather than writing the scene twice.

**`TogglePerfOverlay` is not fixed, and this is why.** It is not a missing
call: the native loop has no `PerfSampler` at all, by an explicit choice
recorded at `run.rs:135`. Showing the overlay there means threading a
sampler plus GPU stats through that loop's hot path with `begin_frame`,
`end_frame` and four `scope` sites — a feature port into a second render
loop rather than a defect repair, and one this rig cannot validate,
because everything here renders on lavapipe and the frame timings would
be fiction. The Keybindings tab still offers the action, so the promise
is still outstanding; it wants either the port or an honest
"not on this backend" note in the tab, the way R10 and R21 handled
window-awareness and span.

Two notes on method, both mistakes worth not repeating:

- The first sweep was **invalid** and nearly produced a false finding.
  `Tab` is bound to Cycle entity, but it is also egui's focus key, so
  pressing it moved keyboard focus into the settings panel and every
  later key in the batch went to a widget instead of the shortcut table —
  the panel had quietly switched to the Keybindings tab. Sweeps must not
  contain `Tab`, and focus has to be cleared first.
- `FpsUp` / `FpsDown` looked dead too. They are not: the rig's own virtual
  keyboard had no entry for `[` and `]` and was silently logging
  `unknown key`. Always check the rig's log before believing the app's
  silence.

### R28 · Hot-reload was absent on native Wayland while the README called it stable — `FIXED`

Run the overlay on the native Wayland backend, edit
`~/.config/animaengine/config.toml`, wait. Nothing happens — no log line,
no toast, no moved character. There is not one reference to hot-reload
anywhere under `src/wayland/`.

Meanwhile the README lists it as a feature ("edit `config.toml` while the
app runs; changes are decoded off the UI thread and applied seamlessly")
and its backend-parity table says **stable / stable**. Same family as R10,
R21 and R27: the documentation promising a backend something it does not
do.

Unlike those, there was no structural obstacle. Reloading is reading a
file and building a `Scene`; nothing about it is X11- or Wayland-specific.
So the mechanism moved to `src/config_watch.rs`, owned by neither backend,
and the native loop uses it — rather than a third copy of logic in a loop
that already duplicates enough of the other one.

Two details worth keeping:

- The watcher is told about the app's **own** saves. Without that, saving
  the config changes its mtime, the next poll sees a "change", and the app
  reloads the file it just wrote — discarding the selection and every
  texture for nothing.
- The unsaved-edit check happens **twice**: before starting a reload and
  again when it lands. The worker decodes every asset in the scene, so it
  is not instant, and an edit made while it ran would otherwise be
  overwritten — which is exactly the bug the same review found on the
  winit path.

The winit path was deliberately **not** migrated to the shared watcher in
the same change. It works today and its version is entangled with `App`'s
warning banners; replacing something healthy is a different risk from
giving a second backend a feature it never had. The duplication is
recorded rather than resolved.

Verified on the rig: three successive external edits produced three
`Hot-reload applied: 5 entities` lines and the characters moved, with
exactly one worker spawned per edit and none from the app's own writes.

The parity table's **perf overlay** row was wrong in the same way and is
now corrected rather than implemented — see R27 for why that one is a
port rather than a patch.

### R29 · The perf-overlay chord had never worked, on either backend — `FIXED`

Found while closing R27's other half. With a `PerfSampler` finally wired
into the native Wayland loop, `Ctrl+Shift+`` ` still did nothing —
but rebinding the same action to a plain backtick showed the overlay
immediately. So the sampler was fine and the *chord* was never arriving.

Holding Shift changes the key's identity before either backend sees it:

- Wayland gets a **resolved keysym** from sctk, so Shift+`` ` `` is
  `asciitilde`, not `grave`;
- winit reports the **logical character**, so the same press is `'~'`.

Both translation tables listed only the unshifted form. `grave` was
mapped, `asciitilde` was not, and the press stopped being an event at all
— not a chord that missed, an event that was never generated. The letters
had been right all along, because they list `a | A` and `'a'..='z' |
'A'..='Z'`; only the punctuation was half-mapped.

So `Ctrl+Shift+`` `, the **default** binding for the perf overlay, has
never fired since it was introduced. That is also why nobody had noticed
the native loop was missing a sampler: the shortcut that would have shown
it up did not work on the backend that had one either.

The same hole swallowed any Shift+punctuation chord a user tried to
create. In the rebinder it is worse than silent: the capture widget waits
for a key it can convert, so pressing Shift+`[` simply never completes the
recording.

**Fixed** by listing the shifted keysym and character alongside the plain
one for every punctuation key we bind — `~`, `{`, `}`, `_` — so the key
identity stops depending on the modifier, which is the thing the modifier
mask already records. `+` and `=` were already separate `SymbolKey`s with
both bound by default, and are deliberately left alone: merging them would
change what existing configs mean.

Worth noting for the next round: this was findable only because the fix
for something else made the failure *visible*. R27 hid R29, and R19 hid
R26 the same way. A silent backend is a good place for a second bug.

### R30 · Leaving edit mode with the keyboard threw away your changes — `FIXED`

Found by exercising the Keybindings tab through the actual UI, to check
R26's and R29's fixes end to end rather than only in unit tests. Record a
chord, watch it appear in the list, press `Ctrl+Shift+A` to leave edit
mode — and it is gone. Leave with the **⚙ button** instead and it
persists.

The save lived in the ⚙ button's handler, not in `set_edit_mode`, so the
keyboard path — which is the one the UI itself recommends, "Press Escape
or click ⚙ button to exit" — never reached it. The winit path is immune
by construction: its save sits inside `App::toggle_edit_mode`, which both
of its entry points call.

The two log lines are what gave it away: `Edit mode off (Wayland)` from
the shortcut versus `Edit mode off (Wayland, toggle button)` from the
button, with `Config saved to config.toml` following only the second.

Scope is wider than rebinding. Every edit-mode change goes through the
same flag — dragging a character, nudging, opacity, gravity, z-order —
so any of them made and then dismissed by keyboard stayed in memory
only. Nothing is lost at *quit*, since the shutdown path persists a dirty
config; what is lost is everything in between, including a crash.

And since R28 it compounds: a dirty scene also **blocks hot-reload**, on
purpose, so that an in-flight edit is never overwritten. A session left
dirty by the keyboard therefore ignores every external change to
`config.toml`, silently, until something else happens to save.

**Fixed** by giving both paths one `flip_edit_mode` helper that flips,
logs and persists — the structural equivalent of what `toggle_edit_mode`
already does on the other backend. The two log spellings are kept,
deliberately: they are what made the asymmetry visible.

Verified on the rig in both directions: record `Shift+[`, leave by
keyboard, and the config now gains the chord with a `Config saved` line
behind it.

### R31 · A chord's ✕ could drift onto the next line, above a different chord — `FIXED`

Seen while screenshotting the Keybindings tab for R30. **Pause all
animations** renders, at the default panel width, as:

```
Pause all animations   Ctrl+Shift+P
                    ✕  Space  ✕
```

Each chord was a `Label` plus a separate `small_button`, two widgets, and
`horizontal_wrapped` wraps *between* widgets — so a narrow column can put
a chord at the end of one line and its own ✕ at the start of the next,
immediately left of a different chord. The ✕ a user would naturally take
for Space's is Ctrl+Shift+P's. Removal is silent and unconfirmed, so the
misclick costs a binding with no way to notice.

Not exotic: it happens in the **default** configuration, on the default
panel width, in the first three rows of the tab. The long localized
labels the column was designed around (German's
"Bearbeitungsmodus umschalten") only make it likelier.

The same look also turned up a second, older mismatch: the onboarding
coach mark has always read *"Click any chord to remove it; press a key
combo to record a new one."* The chord itself was an inert `Label`. Only
the ✕ ever did anything.

**Fixed** by making the chord and its ✕ one widget — a small button
labelled `Ctrl+Shift+P ✕`. It cannot be split by wrapping, because there
is nothing left to split, and clicking the chord now removes it, which is
what the onboarding text had been promising all along. Conflict colouring
and the monospace face are unchanged.

Verified on the rig: `Pause all animations` now wraps as two whole chips
on separate lines, and clicking the `Shift+[ ✕` chip on another row
removed exactly that chord, leaving `Ctrl+Shift+H` in place.

### R32 · The Library tab told users to create a directory the app never reads — `FIXED`

The empty state says:

> Drop files into `~/.local/share/animaEngine/assets/` or set
> `ANIMA_ASSETS_DIR` to point at your collection.

The app had just logged, one second earlier:

```
Created asset library at …/data/animaengine/assets
```

**`animaengine`, not `animaEngine`.** `directories::ProjectDirs`
lower-cases the project name on Linux, and Linux filesystems are
case-sensitive, so a user following the instruction literally creates a
second directory beside the real one and watches the tab stay empty
forever.

The path was also hard-coded from `$HOME`, so it ignored `XDG_DATA_HOME`
**and** `ANIMA_ASSETS_DIR` — the very variable the same sentence tells you
about. Anyone who had set either was pointed somewhere unrelated, and the
"Copy path to clipboard" button beneath copied the same wrong string.

A third defect sat in the other empty state, the one shown when no asset
directory exists at all: `library-no-asset-root` has always carried a
`{ $path }` placeholder and was rendered through `t()` with no argument,
so the placeholder went on screen instead of a path.

The comment on the function said it all: *"kept in sync with
`library-no-asset-root` i18n and the doc in `docs/config.md`"*. One path,
copied into a Rust literal, ten `.ftl` files and four docs, kept in
agreement by hand. It wasn't.

**Fixed** by making `asset_library` answer the question — `asset_root_hint`
resolves it exactly as the loader does, including the env override — and
having both the sentence and the clipboard use that one value. The ten
locale strings were *parameterised, not translated*: the literal path
appears verbatim in all ten, so replacing it with `{ $path }` is
mechanical and invents nothing. The four current docs are corrected;
archived release notes are left alone, because they record what was said
at the time.

The new test asserts the real directory name is lower-case. That is the
thing that silently diverged, and it comes from a dependency, so it can
diverge again.

Found by opening the Library tab and reading it against the log — the
same habit that produced R31, one tab over.

### R33 · The language picker drew Japanese as three empty boxes — `FIXED`

Open **Appearance → Language** with an English UI. Nine languages read
normally; the tenth is `□ □ □`.

R23 taught the app to load a CJK face *when the active locale needs one*,
which fixed a Japanese UI. But every entry in the picker is labelled with
its own name — `日本語`, not "Japanese" — and that list is drawn while the
UI is still English, so the face is not loaded and the label cannot be
drawn. The one row a Japanese reader has to find in order to switch was
the one row rendered unreadable.

Not the case R23 already handles. That one is "no CJK font installed",
where the picker correctly offers those languages disabled with a tooltip
naming the missing package. Here the font **was** present — 19 MB of Noto
CJK, and `fc-list :lang=ja` reports 31 faces — so the entry was enabled,
selectable, and illegible.

**Fixed** by loading the face while the dropdown is open, which is a
deliberate act by someone who is looking for a language. R23's actual
concern is preserved: a user reading English who never opens the picker
still never pays the ~19 MB.

Two details the first attempt got wrong, both caught on the rig:

- The closure runs **every frame** the list is open, and a rebuild
  re-reads and re-parses 19 MB. Asking sixty times a second would have
  traded an unreadable label for a visible stall, so the forced install is
  guarded by a flag that mirrors what is actually on the context.
- That flag has to be cleared on the paths that leave CJK *out*, or
  switching back to English would leave it set and the next open would
  skip a rebuild it genuinely needs.

Verified as a full cycle, counting loads in the log: none at startup, one
when the picker opens with `日本語` now legible, a second on selecting it
and the whole UI in Japanese, none on reopening the picker while Japanese
is active, none on switching back, and a third when the picker is opened
again from English.

### R34 · Three more messages printed `{$path}` at the user — `FIXED`

Selecting **Behavior → Script** in the Inspector draws a path field with
this underneath it:

> Relative to your asset library — **{$path}**

The same defect R32 fixed one panel over, and the reason it was still here
is that R32 fixed *an instance*. A Fluent message that declares `{ $var }`
and is rendered through `t()` has nothing to substitute, so the
placeholder goes on screen verbatim — and neither the compiler nor the
existing locale tests can see it, because the key is a string and `t` and
`t_args` differ only in whether an argument was passed.

Sweeping all 34 placeholder-carrying messages against every `t("…")` call
in the tree turned up three keys across four call sites:

- `behavior-script-path-hint` — the one above;
- `shimeji-no-library-toast`, on **both** backends — "No asset library
  root — create `{ $path }` first", which is nothing but a path, so the
  message was pure placeholder;
- `shimeji-import-failed-toast` — correct at one call site and plain at
  the other, which is the shape this family takes: somebody adds a second
  error path and copies the `t()` next to it.

All four now pass the value, and the two path ones go through R32's
`asset_root_hint`, so the whole app names one directory.

**The test is the point.** It parses `en.ftl`, collects every message with
a placeholder, and fails if any is rendered by a bare `t("key")` anywhere
under `src/`. Reading the sources from a test is unusual; this class is
otherwise invisible to the compiler, and all four of these sat in code
paths — an empty state, two failure toasts — that nobody looks at twice.
Confirmed it actually bites by putting one of the defects back: it fails
and names the key.

### R35 · `Shift`+digit could not be bound either — `FIXED` (superseded by R40)

R29 fixed the shifted punctuation and stopped there. The number row has
the identical hole: on a US layout `Shift+1` is `!` on the winit side and
the keysym `exclam` on the Wayland side, and neither was mapped.

Verified the same way: open the Keybindings tab, press **+ Add**, hold
`Shift+1`. The widget goes on saying *"Press a chord… (Esc to cancel)"*
forever, because nothing it can convert ever arrives.

Nothing binds a digit by default, so this costs nobody a working
shortcut — but the rebinder invites the user to press any chord, and a
third of the keyboard silently refused. Each digit now lists its shifted
form alongside the plain one, the same rule R29 applied: the key keeps its
identity and `Shift` lives in the modifier mask, which is where the chord
already records it.

**A limitation worth stating plainly, because this fix does not remove
it.** These tables are US-layout-shaped, and always have been —
`bracketleft`, `grave` and now `exclam` are only those keys on a US
layout. On AZERTY the unshifted number row gives `&é"'(-è_çà`, none of
which is mapped, so those keys cannot be bound at all. Fixing *that*
means keying chords off the physical key rather than the symbol, which
changes what every stored chord means and is a different piece of work.
Recorded here rather than implied away.

**Correction, from R40.** The fix above was wrong in a way this entry
did not see. Listing `!`→1 … `&`→7 is the *US* shifted row, and on AZERTY
`&` is the **unshifted** 1 key — so the "fix" made that key record as 7.
Binding the wrong key is worse than refusing it. And the last paragraph
overstated the cost of doing it properly: keying the number row off the
physical position does **not** change what stored chords mean, because
they were always written with US names and on a US layout position and
symbol agree. R40 does that and removes these aliases.

### R36 · A chord you could record on one backend, the other refused — `FIXED` (superseded by R40)

The first thing the newly-drivable X11 path turned up. Record `Shift+[` in
the Keybindings tab: on native Wayland it lands as `Shift+[`, on
winit/X11 the widget waits for a chord that never arrives. R35's
`Shift+1` behaved the same way.

Only the **rebinder** goes through egui. Dispatch on the winit path reads
winit events directly, so R29 and R35 already made those chords *fire*
there — it was recording one that was impossible, which is a worse
failure, because the user cannot even ask for the binding.

The cause is that egui reports a shifted symbol as **its own key**:
`Key::OpenCurlyBracket`, not `OpenBracket` with Shift; `Exclamationmark`,
not `Num1`. `KeyCode::from_egui` knew none of them and returned `None`.
Native Wayland never hit it because that backend builds the egui key
itself out of the keysym, and R29/R35 had already folded `braceleft` and
`exclam` onto the plain keys before egui saw anything. Two paths into one
function, and only one of them was exercised.

**Fixed** by folding egui's shifted variants onto the same `KeyCode`, so
both backends record one chord for one physical press. The new test
asserts exactly that equivalence rather than each side separately —
checking them apart is what let this hide.

**What is still not recordable on winit, and cannot be fixed here:**
egui's `Key` has no variant for `~`, `@`, `#`, `$`, `%`, `^`, `&`, `*`,
`(`, `)` or `_`, so egui produces no key event at all for those presses
and the rebinder has nothing to catch. `Shift`+`` ` ``, the perf
overlay's own default, is in that list — it *fires* on both backends, but
on winit it cannot be re-recorded if the user ever clears it. Native
Wayland has no such gap, because it does its own keysym folding. Removing
it means either upstream variants or capturing from winit instead of
egui, which is a larger change than this.

**Correction, from R40: that last paragraph is false.** egui-winit emits
`logical_key.or(physical_key)` — when a symbol has no egui variant it
falls back to the physical key rather than emitting nothing (its own
comment calls this the fallback for non-Latin layouts). So `~`, `@` and the
rest *were* recordable on winit; the limitation described above never
existed. It was written from the shape of egui's `Key` enum without
reading the one function that decides what gets emitted. The aliases this
entry added (`OpenCurlyBracket`→`[`, `Exclamationmark`→1) are US-only and
are removed by R40, which falls back to the physical key instead.

### R37 · A dropped file did not land where you dropped it (X11) — `FIXED, unverified`

The last unexamined surface, reached by writing an XDND drag source for
the rig (nothing packaged does this headlessly).

The drop itself works: the file is validated, decoded and added. But on
X11 the character appears at the **last known cursor position**, not at
the drop point — `handle_dropped_file` used `self.mouse_x / mouse_y`.

That position cannot be right during a drag. XDND's drag source holds a
pointer grab, so the target receives no `MotionNotify` at all; position
travels as `XdndPosition` client messages instead. winit parses those —
its own source says *"this event occurs every time the mouse moves while
a file's being dragged over our window"* — but `DroppedFile` carries a
path and nothing else, and winit's comment admits the API has nowhere to
put the coordinates. In pass-through mode the overlay's input region is
the ⚙ corner alone, so the last motion it ever saw may be from minutes
earlier, or never.

**Native Wayland never had this.** `wl_data_device` delivers `motion`
during the drag and the backend caches it in `last_drag_pos`, with the
screen centre as fallback. One more case of the two backends being
written at different times against different protocols.

**Fixed** by asking the X server directly at drop time — `QueryPointer`
on the root, whose coordinates are already the global desktop space
`mouse_x` lives in. It falls back to the old behaviour when the query
fails, so the worst case is unchanged.

**Marked unverified on purpose.** The rig cannot demonstrate the
improvement, and the reason is worth keeping: under **XWayland**,
`XQueryPointer` only knows what XWayland has been told, and a surface
with an empty input region is sent no pointer events — so it answers with
a stale position, exactly like the bug. Measured: with the overlay in
pass-through and the virtual pointer moved to (400, 850), `xdotool
getmouselocation` still reported (1567, 34). On a **real** X server the
pointer is the server's own state and input shapes only route events, so
the query is authoritative there. The fix is correct by construction for
the platform it targets and inert on the one that can be tested, which is
an uncomfortable combination — it wants one drag on a real X11 session
before being trusted.

## Swept and clean

Checks that found nothing, recorded so the next round does not repeat
them. Each is the *class* behind a real defect, re-run across the whole
surface after the fix.

**Backend asymmetry** — the class behind R19, R27 and R28, where one
loop handled something the other silently ignored. Every shared
enum is now handled on both paths: all 28 rebindable `Action`s (bar the
perf overlay's own sampler, R27), the six `MenuAction`s, both
`PaletteOutcome`s, `LibraryOutcome`, and all eight `AnimaEvent`s.

**Names promised to the user vs names the code uses** — the class behind
R32 and R34. All six `ANIMA_*` environment variables read by the code are
documented and none is documented that is not read; the only CLI flags
parsed are `--help` and `--recover`, exactly what the stability policy
guarantees (`--frobnicate` and `--recovr` in the source are test fixtures
for the unknown-flag rejection); every key in `docs/config.md` maps to a
real field.

No test guards the config-key half, deliberately. The schema does **not**
set `deny_unknown_fields`, because the stability policy promises a config
written by a newer release still loads in an older one — so "a documented
key that does not exist" cannot be detected by loading it, and the
alternative, a shadow schema in the test, would rot faster than it would
catch anything.

## Still unexamined

Verified since, on the headless rig:

- **Scripted behaviors run live.** A character with `type = "script"` moved
  453 px in two seconds against 440 expected at `speed = 220`. The asset
  library is also created on first run now, which it was not.
- **Multi-monitor (PerMonitor).** With two outputs the app spawns an extra
  layer surface on the second and a character placed at x = 2100 renders
  there, not on the primary.
- **Both high-contrast themes.** Tab icons measure 14.7–15.9:1 on dark HC
  and 8.6–11.7:1 on light HC — comfortably past AAA, unlike the ordinary
  light theme, which needed R15.
- **Keyboard shortcuts end to end** — which is how R19 was found. All
  twenty-eight rebindable actions have now been pressed individually on
  native Wayland; twenty-six work, one (`HideOverlay`) is global-hotkey
  only by design on both backends, and one (the perf overlay) is R27's
  open half.

Also verified: **preset Append and Replace** both behave — Append took the
scene from 5 entities to 6, Replace took it to 1, and the footer even
pluralises "1 entity" correctly.

Also swept: **all ten locales**. Every Latin-script one renders correctly,
diacritics included; Japanese was R23. Romanian turned up a translation
consistency problem rather than a rendering one — R24.

**Shimeji pack import** was exercised for the first time, including its
hardening. A legitimate pack imports (actions.xml parsed, `Stand` → idle,
`Walk` → walk, frames copied, missing states reported with reasons), and
three hostile packs are all refused with nothing from outside the pack
reaching disk: an `actions.xml` that is a symlink to `/etc/passwd`, a
sprite path of `../../../../etc/hostname`, and a FIFO where a sprite
should be. The threat-model claims there hold up.

Drag-and-drop was the last item here and is now exercised, through an
XDND drag source written for the rig — see R37. The list is empty.

**The winit/X11 path is no longer on this list.** It was, for several
rounds, on the strength of an Xvfb result; running it under sway's own
XWayland drives it fine (see the rig section above), and doing so
immediately produced R36. Everything fixed in the R25–R35 range that
touches shared code has now been re-checked there: the perf-overlay
chord, the chord chips, the script-path hint and `Shift`+digit all behave
the same on both backends.

The rig now drives the keyboard as well as the pointer, which is what made
R14 findable. Two traps in doing so are written up under R14; the short
version is that both `wlrctl` and `wtype` create their virtual device, use
it and exit, and the app can never bind a device that transient.

A third trap surfaced while verifying R25, and it survives the usual fix.
Keeping a spare `wtype` alive holds the seat's keyboard capability up, but
**every new `wtype` process uploads a fresh keymap**, and the app logs
`non-xkb compatible keymap` and drops the key — so the capability is
present, the key is sent, and nothing happens. The rig now runs a single
`zwp_virtual_keyboard_v1` client for the whole session, with one keymap,
taking keystrokes from a FIFO; nothing about the seat changes between them.

Two rig bugs are worth recording next to the app's, because both nearly
produced false findings:

- `zwlr_virtual_pointer_v1::motion_absolute` is mapped against the extent
  the *caller* declares, and the compositor stretches that over the whole
  output layout. Declaring one monitor's width on a two-monitor desktop
  doubles every x, so clicks land on the other screen and the app looks
  unresponsive.
- The first-run onboarding tour covers the ⚙ button, so a scripted click
  on it does nothing until the tour is dismissed.

**Seen once, not reproduced:** during R27's sweep a two-output session
rendered no entities at all — five loaded and the panel drew, but the
desktop stayed black. It has not recurred in four deliberate attempts
(one clean two-output start, three app restarts against the same
compositor, all rendering normally), and that session had accumulated a
lot of state: entities repinned between outputs, the virtual keyboard
restarted under the running app, the app killed and relaunched several
times. Recorded rather than filed, because "nothing renders on two
monitors" is worth recognising fast if it turns up again.

### R38 · The onboarding tip named tabs that do not exist — `FIXED`

R24 fixed one concept expressed with three words in Romanian and noted
that *"the other eight non-English locales have had no such check"*. This
is that check, done the one way it can be done without being fluent in
eight languages: not by judging word choice, but by testing a
**cross-reference** the strings make to each other.

`onboarding-tabs` exists to say "settings live across five tabs —
Inspector, Scene, Library, Appearance, Keybindings", i.e. go and find
them. Four locales named a tab something the tab is not:

| locale | the tip says | the tab reads |
|---|---|---|
| de | Inspector | **Inspektor** |
| de | Tastenkürzel | **Kurzbefehle** |
| it | Inspector | **Ispettore** |
| ja | キー割り当て | **ショートカット** |
| nl | Uiterlijk | **Weergave** |

Five wrong names across four languages, in the one string whose entire
job is to be followed.

Nothing catches this by reading a file: the tip and the tab labels are
separate messages, translated independently, and each is perfectly
reasonable on its own. Only the relationship between them is wrong.

**Fixed** by copying each locale's own tab label into its own tip — no
translation invented, the words were already in the file. A test now
asserts every `settings-tab-*` value appears in that locale's
`onboarding-tabs`, and it was confirmed to bite by putting the Dutch
defect back.

A broader terminology sweep found the rest healthy. Most locales use one
dominant word for "shortcut" plus "combination" for the key sequence
itself, which is the distinction R24 deliberately kept in Romanian.
Polish uses a single term throughout. German was the outlier with four
competing words, and the two that mattered are the ones fixed above.

### R39 · One shortcut, two names, in one language — `FIXED`

Following R38's method — test the *relationships* between strings, not
their wording — to the next kind of cross-reference: prose that spells a
chord out in words.

Five strings do it (the coach marks, the palette footer, the what's-new
panel), and each is duplicated across ten locales. German said
**`Strg+K`** in one of them and **`Ctrl+K`** in the others, in the same
file. The app renders every chord through `KeyChord::display_str`, which
is not localised, so the Keybindings tab, the palette and the help all
read `Ctrl+K` — a German user met the same shortcut under two names one
panel apart.

**Fixed narrowly**, by making the German prose match what the app
displays. That uses the word already dominant in its own file and agrees
with the UI.

**The larger question is deliberately left to the maintainer.** On a
German keyboard the key really is labelled `Strg`, and every German
desktop application says so; the honest fix is arguably to localise the
*display*, not to anglicise the prose. That means giving
`display_str` per-locale modifier names — and **only** `display_str`:
`canonical_str` is the on-disk config format and must not move, which the
stability policy pins. A real feature, a real decision, not something to
slip in under a consistency fix.

The test guards both halves. It pulls every `Ctrl+…`-shaped token out of
every locale and asserts it parses *and* is still a current default
binding, so changing a default makes the stale sentences fail rather than
quietly lie in languages nobody here reads. Confirmed by re-pointing the
command palette at `Ctrl+J`: it named every locale still saying `Ctrl+K`.
`Ctrl+A/C/V` is skipped on purpose — that is egui's own text editing, not
one of our bindings.

**The open question, answered: the display is localised now.** The
maintainer's plan for 1.2 took the honest fix. Modifier names come from
four Fluent messages (`key-mod-ctrl`, `-shift`, `-alt`, `-super`) and go
into `KeyChord::display_str`, which now takes the names as a parameter so
the Keybindings tab looks them up once a frame rather than four times a
chip. `canonical_str` does not read them — it writes the English names
unconditionally, so `config.toml` is byte-for-byte what it was in every
language. Parsing additionally accepts `Strg`, because a German user who
reads `Strg+K` in the UI may well type it into the file; it is written
back as `Ctrl+K`.

Only German changes: `Strg` is the one name certain enough to ship
without a native speaker. French `Maj`, Spanish `Mayús`, Italian `Maiusc`
and German `Umschalt` for Shift are real conventions in large
applications, and are left for the native review rather than guessed.

The narrow fix above is reversed accordingly — German prose says `Strg`
again — and the test is stricter than before. It no longer asks only
whether a chord in prose parses to a default binding; it asks whether the
token is *exactly* how that locale displays one. German `Ctrl+K` now
fails, confirmed by putting it back in the palette footer. Checked on the
rig in German: the Keybindings tab and the coach mark above it both say
`Strg+K` and `Strg+Shift+A`, and the saved config still says `Ctrl`.

### R40 · Every shortcut table assumed a US keyboard — `FIXED`

R29 and R35 made shifted keys bindable by listing what Shift produces on
each key: `~` is the backquote key, `!` is 1, `&` is 7. That is a US
keyboard. The rig's virtual keyboard can load any xkb layout, and on
French AZERTY the key left of 2 types `&` **without** Shift, so R35 made
Ctrl+that-key record and fire as `Ctrl+7`. A binding that silently lands
on the wrong key is worse than one that refuses.

Both earlier entries had also stated things that were not so. R36 said
egui emits no key at all for `~`, `@` and the like on winit, which is why
it thought they could not be recorded there; egui-winit in fact sends
`logical.or(physical)`, falling back to the physical key exactly when the
symbol has no egui name. R35 said keying off the physical key "changes
what every stored chord means"; it does not, since stored chords were
always written with US names and on US position and symbol agree. Both
entries now carry a correction.

**Fixed** by giving the key one rule, applied in one place
(`KeyCode::resolve`) and fed by every path — winit dispatch, the native
Wayland loop, the rebinder:

- **letters and named keys follow the label** — AZERTY Ctrl+A is the key
  with A printed on it, which sits where US has Q;
- **the number row follows the position** — AZERTY digits are shifted, so
  going by label would leave the whole row unbindable;
- **a symbol we have a name for follows the label, and anything else
  falls back to the position** — `²`, `$`, dead keys, `é`, and every
  non-Latin letter, so Cyrillic Ctrl+С is still Ctrl+C.

The third rule is not the obvious one, and the obvious one was tried
first. Putting *all* symbols on their position looked more uniform and
passed every AZERTY check; then a German layout showed the problem. German
`+` sits where US has `]`, so by position the key the Keybindings tab calls
`+` (opacity up) fired `]` (FPS up). The label has to win when we
understand it.

The native Wayland path needed a physical key it never had: it always
sent `physical_key: None`. It now reads the evdev scancode `wl_keyboard`
delivers, and the US-only keysym aliases are gone from both the keysym
table and the winit converter.

Checked on the rig, both backends, with the virtual keyboard's layout
switched rather than simulated:

| layout | pressed | result |
|---|---|---|
| fr | Ctrl + the `&` key | recorded as `Ctrl+1` |
| fr | Ctrl+Shift + the `²` key | perf overlay opens |
| fr | Ctrl+Shift + the key labelled A | edit mode toggles |
| de | `-`, `-`, `+` on a selected entity | opacity 80 %, 70 %, 80 % |

Unit tests pin each rule, and one asserts that recording (through egui)
and dispatch (through winit) agree for the same press on every layout
listed — a binding that records and then never fires is the failure this
whole series kept producing.

What is still US-centred is the *name*, not the key. A key the app has
no name for is shown under its US name, so AZERTY's `²` reads as `` ` ``
in the Keybindings tab and German `ü` as `[`. It is the right key with
the wrong caption. R39 localised the modifier names; key names would need
the layout at render time, a larger change. One default is affected: the
perf overlay reads `` Ctrl+Shift+` `` on keyboards where that key says
`^` (German) or `²` (French).

### R41 · What the two backends' copies disagreed on — `FIXED` (one part unverified)

Not found on screen, unlike everything above: found by *reading*, while
folding the handlers each backend kept its own copy of into shared
modules (`src/outcomes.rs`, `src/config_watch.rs`). Recorded here because
it is the same class of defect as R19, R27, R28, R30 and R37 — a fix
that reached one copy — and because the count is the argument for having
done it:

| where | winit | native Wayland |
|---|---|---|
| preset applied with *Replace* | clears the selection | kept an index into the rebuilt scene, so `Delete` could remove a character nobody selected |
| "preset loaded" toast | hard-coded English | translated |
| `D` (duplicate) | no toast | toast |
| hot-reload worker dies | banner | a log line only |
| hot-reload toasts | English | English |
| dropped file | added, selected, toasted, saved | added only — **never marked dirty**, so not saved unless something else was edited |
| dropped Shimeji pack folder | imported off-thread | refused |
| pasted Shimeji pack path | imported off-thread | imported on the UI thread (the freeze winit had fixed), all at (100, 100) |

All eight now run from one copy. A new test fails if any toast is built
from a literal or `format!`, which is how the four English toasts
survived: toasts are the one UI surface assembled in plain Rust.

Verified on the rig: keyboard duplicate and delete on both backends, the
X11 context menu, hot-reload on both, and a real XDND drop on X11 that
added, selected and saved the file. The Wayland drop was left
unverified, because the rig had no Wayland drag source — and building one
(R42) showed the fix above could never have mattered: the native overlay
had never received a drop at all.

### R42 · Native Wayland never received a single file drop — `FIXED`

The README's feature matrix called drag-and-drop **stable** on the
native Wayland backend. It had never worked, not once, since it was
written in phase E.3.

Found by closing the last "unverified" in R41: the rig gained `wldnd`, a
Wayland drag source (a surface that starts a drag on the first press,
serves `text/uri-list`, and prints what the target did), alongside
`xdnd` for X11. Every drag onto the overlay ended `cancelled`. A protocol
trace of the app showed why: it bound `wl_data_device_manager` and never
called `get_data_device`. Without a data device a Wayland client gets no
drag events of any kind.

The data device was created in `SeatHandler::new_seat`, and
smithay-client-toolkit calls that only for a seat that appears *after*
startup. The seat that exists at startup — in practice the only one — is
bound without it. The overlay's pointer and keyboard work because they
are made in `new_capability`, which fires for every seat; the data device
now is too. Nothing in the tests could see this: it is one missing
request on a live connection.

With drops arriving, three more defects surfaced, each hidden behind the
first:

- **Every drop landed in the middle of the screen.** The position was
  read from a "last drag position" that the `leave` following each drop
  had already cleared by the time the main loop saw the files. It now
  travels with the files, taken from the offer at drop time.
- **The source never heard the drop succeed.** Under `wl_data_offer` v3
  the target must call `finish`; the overlay never did, so the source got
  no `dnd_finished`. It now finishes and destroys the offer after
  reading.
- **In pass-through, a drop could only land on the ⚙ corner.** A
  compositor offers a drop only to a surface whose input region is under
  the pointer, and in pass-through that region is the corner. So the one
  place a file could be dropped put the character under the button.
  Now a drag that reaches the corner widens the region to the whole
  surface until it ends, so the file can be dropped where it should go,
  and the region goes back afterwards whether the drop succeeded or not.
  A successful drop also opens edit mode, as it does on X11.

Verified on the rig with `wldnd`, one case each:

| case | result |
|---|---|
| pass-through, dropped on the ⚙ corner | added at the corner, edit mode on, source `finished` |
| pass-through, dropped elsewhere without touching the corner | reaches the window beneath, as it should |
| pass-through, over the corner then dropped at (900, 600) | added at (900, 600) |
| edit mode, dropped at (720, 300) | added at (720, 300) |
| a `.txt` over the corner | rejected; a click afterwards passes through to the window beneath, so the region was restored |

Drops on a per-monitor *extra* surface are still placed as if on the
primary one, the same limit every other pointer event on this backend
has.

### R43 · No tray on native Wayland, and R30 on a third entry point — `FIXED`

Found while verifying the tray-menu refactor. The rig has no panel, so it
gained `snihost.py`: a stub `StatusNotifierWatcher` on the rig's private
bus that lets the app's tray register, after which the published menu can
be read and clicked over D-Bus with `gdbus` (`GetLayout`, `Event`,
`Activate`).

**The native Wayland backend never started a tray.** `tray::spawn` took a
winit `EventLoopProxy`, and only the winit path had one; the README's
feature matrix listed the tray as stable on both. The tray now sends
through an `EventSink` — winit's proxy, or a channel — and on the native
path it shares one channel with the D-Bus activation service, which the
native loop already reads and whose events are exactly the tray menu's.

**Leaving edit mode from that channel did not save.** A toggle arriving
through it — from the tray, a D-Bus call or a portal shortcut — set the
mode directly instead of going through `flip_edit_mode`, which is where
the save on the way out lives. That is R30 again: the keyboard and the ⚙
button were fixed, a third way in was not. It now goes through the same
function.

Verified on the rig, both backends: the menu reads back exactly as
`tray_menu::MENU` defines it; "Toggle edit mode" from the menu and
activating the icon both toggle; on native Wayland, a character nudged in
edit mode was on disk the moment the tray left edit mode (x 200 → 272);
and Quit from the tray exits. What the rig cannot show is a real panel
drawing the icon.

### R44 · `Ctrl+M` made the character disappear — `FIXED`

The README: "a per-entity pin and `Ctrl+M` to cycle the selected entity
through monitors". On the rig with two outputs, native Wayland, the ghost
selected on the left monitor:

1. first `Ctrl+M` → toast "Entity pinned to HEADLESS-2", nothing moves —
   the cycle starts at the first monitor in the list, which here is the
   one it is already on;
2. second `Ctrl+M` → "Entity pinned to HEADLESS-1", and the ghost is
   **gone**: not on the left monitor, not on the right one.

Both the shortcut and the Inspector's "Pin to monitor" picker only ever
rewrote the pin. Positions are global, and each monitor's surface draws
the entities pinned to it offset by its own origin, so an entity pinned to
a monitor its position is not on was drawn off the edge of that surface.
The Inspector still showed x = 158, on the left monitor, for an entity
the left monitor no longer drew.

**Fixed** where the pin changes: `move_to_pinned_monitor` carries the
entity onto the newly pinned monitor, keeping its offset from the corner
of the monitor it was shown on and clamping the whole sprite inside the
new one (`monitor::carry_to_monitor`). Clearing a pin, or pinning to the
monitor it is already on, moves nothing. Both entry points use it.
Verified on native Wayland: two presses carry the ghost from x 158 to
1758, visible and still selected on the right monitor.

Not checked on X11 in the rig, and the reason is worth keeping: sway
tiled the two X11 overlay windows side by side (each got 800 px, so the
primary's ⚙ was drawn mid-screen), and with a floating rule it puts both
at the first output's origin, ignoring the position each asks for. Real
X11 window managers place them where they ask. The fix is the same shared
code on both backends; the X11 extra windows draw with the same
`entity_on_monitor` rule the Wayland ones do.

